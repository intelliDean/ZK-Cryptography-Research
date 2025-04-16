use crate::circuit::Circuit;
use crate::gate::Ops;
use ark_bn254::{FrConfig, G1Projective as G1, G1Projective};
use ark_ff::{BigInteger, Fp, MontBackend, PrimeField};
use ark_std::iterable::Iterable;
use ark_std::rand::{rngs::StdRng, SeedableRng};
use polynomial_commitment_scheme::multilinear::multilinear_pcs::{MultiProof, KZG};
use polynomial_commitment_scheme::univariate::non_interactive_univariate_pcs::g1_to_bytes;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use polynomials::product::product_poly::ProductPoly;
use polynomials::sum::sum_poly::SumPoly;
use polynomials::univariate::uni_poly::UnivariatePoly;
use sha3::{Digest, Keccak256};
use std::borrow::Borrow;
use field_tracker::{end_tscope, start_tscope, Ft};
use sumcheck_protocol::gkr_sumcheck::{prove as sub_prove, verify as sub_verify};
use sumcheck_protocol::transcript::{to_bytes, HashTrait, Transcript};

#[derive(Debug, Clone, PartialEq)]
pub struct KZGProof<F: PrimeField> {
    trusted_setup: KZG<F>,
    commitment: G1,
    proof: [MultiProof<F>; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GKRProof<F: PrimeField> {
    output_poly: MultilinearPoly<F>,
    proof_polynomials: Vec<Vec<UnivariatePoly<F>>>,
    claimed_evaluations: Vec<(F, F)>,
    kzg_proof: KZGProof<F>,
}

pub fn prove<F: PrimeField>(
    circuit: &mut Circuit<F>,
    inputs: &[F],
) -> GKRProof<F> {
    start_tscope!("Prover");
    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
    let inputs_poly = MultilinearPoly::new(inputs.to_vec());
    // prover evaluating the circuit
    let mut circuit_evaluations = circuit.run_circuit(inputs_poly.clone());
    // turn the multilinear poly at index 0 to a vec so you could add to it.
    let mut w_0 = circuit_evaluations.first().unwrap().polynomial.to_vec();

    if w_0.len() == 1 {
        // if it's the final output of the circuit
        w_0.push(F::zero()); // add 0 to make it a valid evaluation multilinear
    }
    let output_poly = MultilinearPoly::new(w_0);

    let (mut claimed_sum, random_challenge) = initiate_protocol(&mut transcript, &output_poly);

    let num_layers = circuit.layers.len();

    let mut claimed_evaluations = Vec::with_capacity(num_layers.saturating_sub(1));
    let mut proof_polys = Vec::with_capacity(num_layers);
    let mut current_rb = Vec::new();
    let mut current_rc = Vec::new();
    let mut alpha = F::zero();
    let mut beta = F::zero();

    // circuit_evaluations.reverse();
    let mut layers = circuit.layers.clone();
    // layers.reverse();

    for (idx, _) in layers.into_iter().enumerate() {
        let w_i = if idx == num_layers - 1 {
            inputs.to_vec()
        } else {
            circuit_evaluations[idx + 1].polynomial.clone()
        };

        let fbc_poly = if idx == 0 {
            get_fbc_poly(random_challenge, circuit, idx, &w_i, &w_i)
        } else {
            get_merged_fbc_poly(
                circuit,
                idx,
                &w_i,
                &w_i,
                &current_rb,
                &current_rc,
                alpha,
                beta,
            )
        };

        let sum_check_proof = sub_prove(fbc_poly, claimed_sum, &mut transcript);
        proof_polys.push(sum_check_proof.round_univariate_polynomials);

        let next_poly = MultilinearPoly::new(w_i);
        let mid = sum_check_proof.random_challenges.len() / 2;
        let (r_b, r_c) = sum_check_proof.random_challenges.split_at(mid);

        let o_1 = next_poly.clone().full_evaluation(r_b.to_vec());
        let o_2 = next_poly.full_evaluation(r_c.to_vec());

        current_rb = r_b.to_vec();
        current_rc = r_c.to_vec();

        if idx < num_layers - 1 {
            transcript.absorb(&to_bytes(&[o_1]));
            alpha = transcript.generate_random_challenge();

            transcript.absorb(&to_bytes(&[o_2]));
            beta = transcript.generate_random_challenge();

            claimed_sum = (alpha * o_1) + (beta * o_2);
            claimed_evaluations.push((o_1, o_2));
        }
    }

    let kzg_proof = process_kzg(&inputs_poly, &mut current_rb, &mut current_rc);

    end_tscope!();

    GKRProof {
        output_poly,
        proof_polynomials: proof_polys,
        claimed_evaluations,
        kzg_proof
    }
}

pub fn process_kzg<F: PrimeField>(
    inputs_poly: &MultilinearPoly<F>,
    current_rb: &Vec<F>,
    current_rc: &Vec<F>,
) -> KZGProof<F> {
    let var_size = inputs_poly.num_var() as usize;

    let mut taus: Vec<F> = Vec::with_capacity(var_size);
    let mut rng = StdRng::from_entropy();

    for i in 0..var_size {
        taus.push(F::rand(&mut rng));
    }

    let kzg = KZG::multilinear_trusted_setup(&taus);

    let commitment: G1Projective = kzg.commit_to_polynomial(&inputs_poly);

    // let (open_at_rb, open_at_rc) = generate_open_at(var_size, &commitment);

    let w_b_proof = kzg.open_polynomial(&inputs_poly, &current_rb);
    let w_c_proof = kzg.open_polynomial(&inputs_poly, &current_rc);

    KZGProof {
        trusted_setup: kzg,
        commitment,
        proof: [w_b_proof, w_c_proof],
    }
}

// fn generate_open_at<F: PrimeField>(
//     var_size: usize,
//     commitment: &G1Projective,
// ) -> (Vec<F>, Vec<F>) {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let mut commitment_bytes = g1_to_bytes::<F>(&commitment);
//
//     let mut open_at_rb = Vec::with_capacity(var_size);
//     let mut open_at_rc = Vec::with_capacity(var_size);
//
//     for _ in 0..var_size {
//         transcript.absorb(&commitment_bytes);
//
//         let open_rb = transcript.squeeze();
//         open_at_rb.push(open_rb);
//
//         let open_rb_bytes = open_rb.into_bigint().to_bytes_be();
//         transcript.absorb(&open_rb_bytes);
//
//         let open_rc = transcript.squeeze();
//         open_at_rc.push(open_rc);
//
//         commitment_bytes = open_rc.into_bigint().to_bytes_be();
//     }
//     (open_at_rb, open_at_rc)
// }

pub fn verify<F: PrimeField>(
    proof: GKRProof<F>,
    mut circuit: Circuit<F>,
) -> bool {
    start_tscope!("Verifier");

    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());

    let (mut current_claim, init_random_challenge) =
        initiate_protocol(&mut transcript, &proof.output_poly);

    let mut alpha = F::zero();
    let mut beta = F::zero();
    let mut prev_sumcheck_random_challenges = Vec::new();

    let num_layers = circuit.layers.len();

    for (i, _) in circuit.layers.iter().enumerate() {
        let sum_check_verify = sub_verify(
            proof.proof_polynomials[i].clone(),
            current_claim,
            &mut transcript,
        );

        if !sum_check_verify.is_proof_valid {
            return false;
        }

        let current_random_challenge = sum_check_verify.random_challenges;

        //============================== KZG ====================================

        let (o_1, o_2) = if i == num_layers - 1 {

            let (eval_rb, eval_rc, w_result) = verify_input(&proof.kzg_proof, &current_random_challenge);

            if !w_result {
                return false;
            }

            (eval_rb, eval_rc)  // 'v' is the evaluation of the polynomial at 'a'
        } else {
            proof.claimed_evaluations[i]
        };

        //========================================================================

        let expected_claim = if i == 0 {
            get_verifier_claim(
                circuit.clone(),
                i,
                init_random_challenge,
                &current_random_challenge,
                o_1,
                o_2,
            )
        } else {
            get_merged_verifier_claim(
                circuit.clone(),
                i,
                &current_random_challenge,
                &prev_sumcheck_random_challenges,
                o_1,
                o_2,
                alpha,
                beta,
            )
        };

        if expected_claim != sum_check_verify.last_claimed_sum {
            return false;
        }

        prev_sumcheck_random_challenges = current_random_challenge;

        transcript.absorb(&to_bytes(&[o_1]));
        alpha = transcript.generate_random_challenge();

        transcript.absorb(&to_bytes(&[o_2]));
        beta = transcript.generate_random_challenge();

        current_claim = (alpha * o_1) + (beta * o_2);
    }
    end_tscope!();

    true
}

pub fn verify_input<F: PrimeField>(
    proof: &KZGProof<F>,
    current_random_challenge: &Vec<F>,
) -> (F, F, bool) {
    let (r_b, r_c) =
        current_random_challenge.split_at(current_random_challenge.len() / 2);

    // let proof = &proof;

    let wb_verified = proof.trusted_setup.verifier_verifies(
        proof.commitment,
        r_b,
        &proof.proof[0], //proof for r_b
    );

    let wc_verified = proof.trusted_setup.verifier_verifies(
        proof.commitment,
        r_c,
        &proof.proof[1], //proof for r_c
    );

    let w_result = wc_verified && wb_verified;

    (proof.proof[0].v, proof.proof[1].v, w_result) // 'v' is the evaluation of the polynomial at 'a'
}

fn initiate_protocol<K: HashTrait, F: PrimeField>(
    transcript: &mut Transcript<K, F>,
    output_poly: &MultilinearPoly<F>,
) -> (F, F) {
    transcript.absorb(&to_bytes(&output_poly.polynomial));

    let random_challenge = transcript.generate_random_challenge();
    let m_0 = output_poly.clone().full_evaluation(vec![random_challenge]);

    transcript.absorb(&to_bytes(&[m_0]));

    (m_0, random_challenge)
}

fn tensor_add_mul_polynomials<F: PrimeField>(
    poly_a: &[F],
    poly_b: &[F],
    op: Ops,
) -> MultilinearPoly<F> {
    let new_eval: Vec<F> = poly_a
        .iter()
        .flat_map(|a| {
            poly_b.iter().map({
                let value = op.clone();
                move |b| value.clone().operation(a, b)
            })
        })
        .collect();

    MultilinearPoly::new(new_eval)
}

// get_fbc_poly(random_challenge, &circuit, idx, &w_i, &w_i)
pub fn get_fbc_poly<F: PrimeField>(
    random_challenge: F,
    circuit: &mut Circuit<F>,
    layer_idx: usize,
    w_b: &[F],
    w_c: &[F],
) -> SumPoly<F> {
    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let add_i_eval = add_i.partial_evaluation(0, random_challenge);
    let mul_i_i_eval = mul_i.partial_evaluation(0, random_challenge);

    let summed_w_poly = tensor_add_mul_polynomials(w_b, w_c, Ops::ADD);
    let multiplied_w_poly = tensor_add_mul_polynomials(w_b, w_c, Ops::MUL);

    let add_eval_product = ProductPoly::new(vec![add_i_eval, summed_w_poly]);
    let mul_eval_product = ProductPoly::new(vec![mul_i_i_eval, multiplied_w_poly]);

    SumPoly::new(vec![add_eval_product, mul_eval_product])
}
fn get_merged_fbc_poly<F: PrimeField>(
    circuit: &mut Circuit<F>,
    layer_idx: usize,
    w_b: &[F],
    w_c: &[F],
    r_b: &[F],
    r_c: &[F],
    alpha: F,
    beta: F,
) -> SumPoly<F> {
    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let new_add_i = add_i.multi_partial_evaluate(r_b).multiply_by(alpha)
        + add_i.multi_partial_evaluate(r_c).multiply_by(beta);

    let new_mul_i = mul_i.multi_partial_evaluate(r_b).multiply_by(alpha)
        + mul_i.multi_partial_evaluate(r_c).multiply_by(beta);

    let summed_w_poly = tensor_add_mul_polynomials(w_b, w_c, Ops::ADD);
    let multiplied_w_poly = tensor_add_mul_polynomials(w_b, w_c, Ops::MUL);

    let add_product_poly = ProductPoly::new(vec![new_add_i, summed_w_poly]);
    let mul_product_poly = ProductPoly::new(vec![new_mul_i, multiplied_w_poly]);

    SumPoly::new(vec![add_product_poly, mul_product_poly])
}

fn get_verifier_claim<F: PrimeField>(
    mut circuit: Circuit<F>,
    layer_idx: usize,
    init_random_challenge: F,
    sumcheck_random_challenges: &[F],
    o_1: F,
    o_2: F,
) -> F {
    let mut all_random_challenges = Vec::with_capacity(1 + sumcheck_random_challenges.len());

    all_random_challenges.push(init_random_challenge);
    all_random_challenges.extend_from_slice(sumcheck_random_challenges);

    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let a_r = add_i.clone().full_evaluation(all_random_challenges.clone());
    let m_r = mul_i.full_evaluation(all_random_challenges);

    (a_r * (o_1 + o_2)) + (m_r * (o_1 * o_2))
}

fn get_merged_verifier_claim<F: PrimeField>(
    mut circuit: Circuit<F>,
    layer_idx: usize,
    current_random_challenge: &[F],
    previous_random_challenge: &[F],
    o_1: F,
    o_2: F,
    alpha: F,
    beta: F,
) -> F {
    let (prev_r_b, prev_r_c) =
        previous_random_challenge.split_at(previous_random_challenge.len() / 2);

    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let new_add_i = add_i.multi_partial_evaluate(prev_r_b).multiply_by(alpha)
        + add_i.multi_partial_evaluate(prev_r_c).multiply_by(beta);

    let new_mul_i = mul_i.multi_partial_evaluate(prev_r_b).multiply_by(alpha)
        + mul_i.multi_partial_evaluate(prev_r_c).multiply_by(beta);

    let a_r = new_add_i.full_evaluation(current_random_challenge.to_vec());
    let m_r = new_mul_i.full_evaluation(current_random_challenge.to_vec());

    (a_r * (o_1 + o_2)) + (m_r * (o_1 * o_2))
}

fn evaluate_input_poly<F: PrimeField>(inputs: &[F], sumcheck_random_challenges: &[F]) -> (F, F) {
    let input_poly = MultilinearPoly::new(inputs.to_vec());

    let (r_b, r_c) = sumcheck_random_challenges.split_at(sumcheck_random_challenges.len() / 2);

    let o_1 = input_poly.clone().full_evaluation(r_b.to_vec());
    let o_2 = input_poly.full_evaluation(r_c.to_vec());

    (o_1, o_2)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gate::{Gate, Ops};
    use crate::layer::Layer;
    // use ark_bn254::{Fq, Fr};
    use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

    use field_tracker::{print_summary, Ft};
    type Fr = Ft!(ark_bn254::Fr);






    fn get_circuit() -> Circuit<Fr> {
        let layer0 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)]);

        let layer1 = Layer::new(vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 2, 3, Ops::ADD),
        ]);

        let layer2 = Layer::new(vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 2, 3, Ops::ADD),
            Gate::new(2, 4, 5, Ops::ADD),
            Gate::new(3, 6, 7, Ops::MUL),
        ]);

        let circuit = vec![layer0, layer1, layer2];

        println!("Layers: {:?}", &circuit);

        Circuit::new(circuit)
    }

    fn get_input() -> MultilinearPoly<Fr> {
        MultilinearPoly::new(vec![
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ])
    }

    #[test]
    fn it_add_polys_correctly() {
        let poly_a = &[Fr::from(0), Fr::from(2)];
        let poly_b = &[Fr::from(0), Fr::from(3)];

        let expected_poly = vec![Fr::from(0), Fr::from(3), Fr::from(2), Fr::from(5)];

        let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::ADD);

        assert_eq!(result.polynomial, expected_poly);

        let poly_a = &[Fr::from(0), Fr::from(3)];
        let poly_b = &[Fr::from(0), Fr::from(0), Fr::from(0), Fr::from(2)];

        let expected_poly = vec![
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(2),
            Fr::from(3),
            Fr::from(3),
            Fr::from(3),
            Fr::from(5),
        ];

        let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::ADD);

        assert_eq!(result.polynomial, expected_poly);
    }

    #[test]
    fn it_multiplies_polys_correctly() {
        let poly_a = &[Fr::from(0), Fr::from(2)];
        let poly_b = &[Fr::from(0), Fr::from(3)];

        let expected_poly = vec![Fr::from(0), Fr::from(0), Fr::from(0), Fr::from(6)];

        let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::MUL);

        assert_eq!(result.polynomial, expected_poly);

        let poly_a = &[Fr::from(0), Fr::from(3)];
        let poly_b = &[Fr::from(0), Fr::from(0), Fr::from(0), Fr::from(2)];

        let expected_poly = vec![
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(0),
            Fr::from(6),
        ];

        let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::MUL);

        assert_eq!(result.polynomial, expected_poly);
    }

    #[test]
    fn test_gkr_protocol() {
        let mut circuit = get_circuit();

        let input = [
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ];

        println!("input: {:?}", &circuit);

        let proof = prove(&mut circuit, &input.clone());

        // println!("Proof: {:?}", proof);

        let verified = verify(proof, circuit);
        println!("Verified: {:?}", verified);
        assert_eq!(verified, true);

        print_summary!();
    }
}

use crate::circuit::Circuit;
use crate::gate::Ops;
use ark_ff::PrimeField;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use polynomials::product::product_poly::ProductPoly;
use polynomials::sum::sum_poly::SumPoly;
use polynomials::univariate::uni_poly::UnivariatePoly;
use sha3::{Digest, Keccak256};
use sumcheck_protocol::gkr_sumcheck::prove as sub_prove;
use sumcheck_protocol::transcript::{to_bytes, Transcript};

// GKR PROTOCOL PROVER
#[derive(Debug, Clone, PartialEq)]
pub struct GKRProof<F: PrimeField> {
    pub(crate) output_poly: MultilinearPoly<F>,
    pub(crate) proof_polynomials: Vec<Vec<UnivariatePoly<F>>>,
    pub(crate) claimed_evaluations: Vec<(F, F)>,
}

pub fn prove<F: PrimeField>(circuit: &mut Circuit<F>, inputs: &[F]) -> GKRProof<F> {
    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
    let inputs_poly = MultilinearPoly::new(inputs.to_vec());
    // prover evaluating the circit
    let mut circuit_evaluations = circuit.run_circuit(inputs_poly);
    // turn the multilinear poly at index 0 to a vec so you could add to it.
    let mut w_0 = circuit_evaluations.first().unwrap().polynomial.to_vec();

    if w_0.len() == 1 {
        // if it's the final output of the circuit
        w_0.push(F::zero()); // add 0 to make it a valid evaluation multilinear
    }
    let output_poly = MultilinearPoly::new(w_0);

    transcript.absorb(&to_bytes(&output_poly.polynomial));

    let random_challenge = transcript.generate_random_challenge();
    let m_0 = output_poly.clone().full_evaluation(vec![random_challenge]);

    transcript.absorb(&to_bytes(&[m_0]));

    let num_layers = circuit.layers.len();

    let mut proof = ProofBuilder::new(num_layers, m_0);

    for (idx, _) in circuit.layers.iter().enumerate() {
        let w_i = if idx == num_layers - 1 {
            inputs.to_vec()
        } else {
            circuit_evaluations[idx + 1].polynomial.clone()
        };

        let fbc_poly = if idx == 0 {
            generate_fbc_poly(random_challenge, &mut circuit.clone(), idx, &w_i, &w_i)
        } else {
            fbc_poly_with_alpha_beta(
                &mut circuit.clone(),
                idx,
                &w_i,
                &w_i,
                &proof.current_rb,
                &proof.current_rc,
                proof.alpha,
                proof.beta,
            )
        };

        proof.process_layer(&mut transcript, fbc_poly, idx, num_layers, &w_i);
    }
    proof.build(output_poly)
}

struct ProofBuilder<F: PrimeField> {
    claimed_evaluations: Vec<(F, F)>,
    proof_polys: Vec<Vec<UnivariatePoly<F>>>,
    current_rb: Vec<F>,
    current_rc: Vec<F>,
    alpha: F,
    beta: F,
    claimed_sum: F,
}

impl<F: PrimeField> ProofBuilder<F> {
    fn new(num_layers: usize, claimed_sum: F) -> Self {
        Self {
            claimed_evaluations: Vec::with_capacity(num_layers.saturating_sub(1)),
            proof_polys: Vec::with_capacity(num_layers),
            current_rb: Vec::new(),
            current_rc: Vec::new(),
            alpha: F::zero(),
            beta: F::zero(),
            claimed_sum,
        }
    }

    fn process_layer(
        &mut self,
        transcript: &mut Transcript<Keccak256, F>,
        fbc_poly: SumPoly<F>,
        idx: usize,
        num_layers: usize,
        w_i: &[F],
    ) {
        let sum_check_proof = sub_prove(fbc_poly, self.claimed_sum, transcript);
        self.proof_polys
            .push(sum_check_proof.round_univariate_polynomials);

        if idx < num_layers - 1 {
            let next_poly = MultilinearPoly::new(w_i.to_vec());
            let mid = sum_check_proof.random_challenges.len() / 2;
            let (r_b, r_c) = sum_check_proof.random_challenges.split_at(mid);

            let (current_o1, current_o2) = self.update_challenges(r_b, r_c, &next_poly, transcript);
            self.claimed_sum = self.alpha * current_o1 + self.beta * current_o2;
            self.claimed_evaluations.push((current_o1, current_o2));
        }
    }

    fn update_challenges(
        &mut self,
        r_b: &[F],
        r_c: &[F],
        next_poly: &MultilinearPoly<F>,
        transcript: &mut Transcript<Keccak256, F>,
    ) -> (F, F) {
        self.current_rb = r_b.to_vec();
        self.current_rc = r_c.to_vec();
        let current_o1 = next_poly.clone().full_evaluation(r_b.to_vec());
        let current_o2 = next_poly.clone().full_evaluation(r_c.to_vec());

        transcript.absorb(&to_bytes(&[current_o1]));
        self.alpha = transcript.generate_random_challenge();
        transcript.absorb(&to_bytes(&[current_o2]));
        self.beta = transcript.generate_random_challenge();

        (current_o1, current_o2)
    }

    fn build(self, output_poly: MultilinearPoly<F>) -> GKRProof<F> {
        GKRProof {
            output_poly,
            proof_polynomials: self.proof_polys,
            claimed_evaluations: self.claimed_evaluations,
        }
    }
}

pub fn generate_fbc_poly<F: PrimeField>(
    random_challenge: F,
    circuit: &mut Circuit<F>,
    layer_idx: usize,
    w_b: &[F],
    w_c: &[F],
) -> SumPoly<F> {
    let (add_i, mul_i) = circuit.add_i_and_mul_i_mle(layer_idx);

    let add_i_eval = add_i.partial_evaluation(0, random_challenge);
    let mul_i_i_eval = mul_i.partial_evaluation(0, random_challenge);

    let summed_w_poly = Ops::ADD.cartesian_operations(w_b, w_c);
    let multiplied_w_poly = Ops::MUL.cartesian_operations(w_b, w_c);

    let add_eval_product = ProductPoly::new(vec![add_i_eval, summed_w_poly]);
    let mul_eval_product = ProductPoly::new(vec![mul_i_i_eval, multiplied_w_poly]);

    SumPoly::new(vec![add_eval_product, mul_eval_product])
}
pub fn fbc_poly_with_alpha_beta<F: PrimeField>(
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

    let summed_w_poly = Ops::ADD.cartesian_operations(w_b, w_c);
    let multiplied_w_poly = Ops::MUL.cartesian_operations(w_b, w_c);

    let add_product_poly = ProductPoly::new(vec![new_add_i, summed_w_poly]);
    let mul_product_poly = ProductPoly::new(vec![new_mul_i, multiplied_w_poly]);

    SumPoly::new(vec![add_product_poly, mul_product_poly])
}

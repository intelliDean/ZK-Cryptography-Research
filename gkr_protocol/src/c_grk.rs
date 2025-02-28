// use crate::circuit::Circuit;
// use crate::gate::Ops;
// use ark_ff::{BigInteger, PrimeField};
// use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
// use polynomials::product::product_poly::ProductPoly;
// use polynomials::sum::sum_poly::SumPoly;
// use polynomials::univariate::uni_poly::UnivariatePoly;
// use sha3::{Digest, Keccak256};
// use sumcheck_protocol::gkr_sumcheck::{prove as sub_prove, verify as sub_verify};
// use sumcheck_protocol::transcript::{to_bytes, HashTrait, Transcript};
//
// #[derive(Debug)]
// pub struct GKRProof<F: PrimeField> {
//     output_poly: MultilinearPoly<F>,
//     proof_polynomials: Vec<Vec<UnivariatePoly<F>>>,
//     claimed_evaluations: Vec<(F, F)>,
// }
//
// pub fn prove<F: PrimeField>(circuit: &mut Circuit<F>, inputs: &[F]) -> Result<GKRProof<F>, String> {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let inputs_poly = MultilinearPoly::new(inputs.to_vec());
//     let circuit_evaluations = circuit.run_circuit(inputs_poly);
//
//     let mut w_0 = circuit_evaluations.first()
//         .map(|eval| eval.polynomial.clone())
//         .ok_or("Circuit evaluation failed")?;
//
//     if w_0.len() == 1 {
//         w_0.push(F::zero()); // Maintain multilinear validity
//     }
//
//     let output_poly = MultilinearPoly::new(w_0);
//     let (mut claimed_sum, random_challenge) = initiate_protocol(&mut transcript, &output_poly);
//
//     let num_layers = circuit.layers.len();
//     let mut claimed_evaluations = Vec::with_capacity(num_layers.saturating_sub(1));
//     let mut proof_polys = Vec::with_capacity(num_layers);
//
//     let (mut current_rb, mut current_rc) = (Vec::new(), Vec::new());
//     let (mut alpha, mut beta) = (F::zero(), F::zero());
//
//     for idx in 0..num_layers {
//         let w_i = circuit_evaluations.get(idx + 1)
//             .map(|eval| eval.polynomial.clone())
//             .unwrap_or_else(|| inputs.to_vec());
//
//         let fbc_poly = if idx == 0 {
//             get_fbc_poly(random_challenge, circuit, idx, &w_i, &w_i)
//         } else {
//             get_merged_fbc_poly(circuit, idx, &w_i, &w_i, &current_rb, &current_rc, alpha, beta)
//         };
//
//         let sum_check_proof = sub_prove(fbc_poly, claimed_sum, &mut transcript);
//         proof_polys.push(sum_check_proof.round_univariate_polynomials);
//
//         if idx < num_layers - 1 {
//             let next_poly = MultilinearPoly::new(w_i);
//             let mid = sum_check_proof.random_challenges.len() / 2;
//             let (r_b, r_c) = sum_check_proof.random_challenges.split_at(mid);
//
//             let (o_1, o_2) = (
//                 next_poly.clone().full_evaluation(r_b.to_vec()),
//                 next_poly.full_evaluation(r_c.to_vec()),
//             );
//
//             current_rb = r_b.to_vec();
//             current_rc = r_c.to_vec();
//
//             transcript.absorb(&to_bytes(&[o_1]));
//             alpha = transcript.generate_random_challenge();
//
//             transcript.absorb(&to_bytes(&[o_2]));
//             beta = transcript.generate_random_challenge();
//
//             claimed_sum = (alpha * o_1) + (beta * o_2);
//             claimed_evaluations.push((o_1, o_2));
//         }
//     }
//
//     Ok(GKRProof {
//         output_poly,
//         proof_polynomials: proof_polys,
//         claimed_evaluations,
//     })
// }
//
// pub fn verify<F: PrimeField>(proof: GKRProof<F>, circuit: Circuit<F>, inputs: &[F]) -> bool {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let (mut current_claim, init_random_challenge) = initiate_protocol(&mut transcript, &proof.output_poly);
//
//     let mut alpha = F::zero();
//     let mut beta = F::zero();
//     let mut prev_sumcheck_random_challenges = Vec::new();
//     let num_layers = circuit.layers.len();
//
//     for i in 0..num_layers {
//         let sum_check_verify = sub_verify(proof.proof_polynomials[i].clone(), current_claim, &mut transcript);
//         if !sum_check_verify.is_proof_valid {
//             return false;
//         }
//
//         let current_random_challenge = sum_check_verify.random_challenges;
//         let (o_1, o_2) = proof.claimed_evaluations.get(i)
//             .copied()
//             .unwrap_or_else(|| evaluate_input_poly(inputs, &current_random_challenge));
//
//         let expected_claim = if i == 0 {
//             get_verifier_claim(
//                 circuit.clone(), i, init_random_challenge, &current_random_challenge, o_1, o_2
//             )
//         } else {
//             get_merged_verifier_claim(
//                 circuit.clone(), i, &current_random_challenge, &prev_sumcheck_random_challenges, o_1, o_2, alpha, beta
//             )
//         };
//
//         if expected_claim != sum_check_verify.last_claimed_sum {
//             return false;
//         }
//
//         prev_sumcheck_random_challenges = current_random_challenge;
//         transcript.absorb(&to_bytes(&[o_1]));
//         alpha = transcript.generate_random_challenge();
//         transcript.absorb(&to_bytes(&[o_2]));
//         beta = transcript.generate_random_challenge();
//         current_claim = (alpha * o_1) + (beta * o_2);
//     }
//
//     true
// }
//
// fn initiate_protocol<K: HashTrait, F: PrimeField>(
//     transcript: &mut Transcript<K, F>,
//     output_poly: &MultilinearPoly<F>,
// ) -> (F, F) {
//     transcript.absorb(&to_bytes(&output_poly.polynomial));
//     let random_challenge = transcript.generate_random_challenge();
//     let m_0 = output_poly.full_evaluation(vec![random_challenge]);
//     transcript.absorb(&to_bytes(&[m_0]));
//     (m_0, random_challenge)
// }
//
// fn tensor_add_mul_polynomials<F: PrimeField>(
//     poly_a: &[F],
//     poly_b: &[F],
//     op: Ops
// ) -> MultilinearPoly<F> {
//     let new_eval = poly_a.iter()
//         .flat_map(|a| poly_b.iter().map(|b| op.clone().operation(a, b)))
//         .collect();
//
//     MultilinearPoly::new(new_eval)
// }
//
//
// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::gate::{Gate, Ops};
//     use ark_bn254::{Config, Fq, Fr, FrConfig};
//     use ark_ff::{Fp256, MontBackend};
//     use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
//     use polynomials::product::product_poly::ProductPoly;
//     use polynomials::sum::sum_poly::SumPoly;
//     use crate::layer::Layer;
//
//     fn get_circuit() -> Circuit<Fr> {
//         let layer0 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)]);
//
//         let layer1 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL), Gate::new(1, 2, 3, Ops::ADD)]);
//
//         let layer2 = Layer::new( vec![
//             Gate::new(0, 0, 1, Ops::MUL),
//             Gate::new(1, 2, 3, Ops::ADD),
//             Gate::new(2, 4, 5, Ops::ADD),
//             Gate::new(3, 6, 7, Ops::MUL),
//         ]);
//
//         let circuit = vec![layer0, layer1, layer2];
//
//
//         println!("Layers: {:?}", &circuit);
//
//         Circuit::new(circuit)
//     }
//
//     fn get_input() -> MultilinearPoly<Fr> {
//         MultilinearPoly::new(vec![
//             Fr::from(1),
//             Fr::from(2),
//             Fr::from(3),
//             Fr::from(4),
//             Fr::from(5),
//             Fr::from(6),
//             Fr::from(7),
//             Fr::from(8),
//         ])
//     }
//
//     #[test]
//     fn it_add_polys_correctly() {
//         let poly_a = &[Fq::from(0), Fq::from(2)];
//         let poly_b = &[Fq::from(0), Fq::from(3)];
//
//         let expected_poly = vec![Fq::from(0), Fq::from(3), Fq::from(2), Fq::from(5)];
//
//         let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::ADD);
//
//         assert_eq!(result.polynomial, expected_poly);
//
//         let poly_a = &[Fq::from(0), Fq::from(3)];
//         let poly_b = &[Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)];
//
//         let expected_poly = vec![
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(2),
//             Fq::from(3),
//             Fq::from(3),
//             Fq::from(3),
//             Fq::from(5),
//         ];
//
//         let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::ADD);
//
//         assert_eq!(result.polynomial, expected_poly);
//     }
//
//     #[test]
//     fn it_multiplies_polys_correctly() {
//         let poly_a = &[Fq::from(0), Fq::from(2)];
//         let poly_b = &[Fq::from(0), Fq::from(3)];
//
//         let expected_poly = vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(6)];
//
//         let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::MUL);
//
//         assert_eq!(result.polynomial, expected_poly);
//
//         let poly_a = &[Fq::from(0), Fq::from(3)];
//         let poly_b = &[Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)];
//
//         let expected_poly = vec![
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(0),
//             Fq::from(6),
//         ];
//
//         let result = tensor_add_mul_polynomials(poly_a, poly_b, Ops::MUL);
//
//         assert_eq!(result.polynomial, expected_poly);
//     }
//
//     #[test]
//     fn test_gkr_protocol() {
//
//         let mut circuit = get_circuit();
//         let input = [
//             Fr::from(1),
//             Fr::from(2),
//             Fr::from(3),
//             Fr::from(4),
//             Fr::from(5),
//             Fr::from(6),
//             Fr::from(7),
//             Fr::from(8),
//         ];
//
//         let proof = prove(&mut circuit, &input.clone());
//
//         // println!("Result: {:?}", proof);
//
//         let verified = verify(proof, circuit, &input);
//         println!("Verified: {:?}", verified);
//         assert_eq!(verified, true);
//     }
// }
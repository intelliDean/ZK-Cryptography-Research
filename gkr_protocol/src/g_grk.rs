// use crate::circuit::Circuit;
// use crate::gate::Ops;
// use ark_ff::{BigInteger, PrimeField};
// use polynomials::product::product_poly::ProductPoly;
// use polynomials::univariate::uni_poly::UnivariatePoly;
// use polynomials::sum::sum_poly::SumPoly;
// use sha3::{Digest, Keccak256};
// use sumcheck_protocol::gkr_sumcheck::{prove as sub_prove, verify as sub_verify};
// use sumcheck_protocol::transcript::{to_bytes, HashTrait, Transcript};
// use thiserror::Error;
// use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
//
// #[derive(Debug, Clone, PartialEq)]
// pub struct GKRProof<F: PrimeField> {
//     output_poly: MultilinearPoly<F>,
//     proof_polynomials: Vec<Vec<UnivariatePoly<F>>>,
//     claimed_evaluations: Vec<(F, F)>,
// }
//
// #[derive(Error, Debug)]
// pub enum GKRError {
//     #[error("Circuit evaluation failed: {0}")]
//     CircuitEvaluation(String),
//     #[error("Invalid layer index: {0}")]
//     InvalidLayer(usize),
//     #[error("Proof verification failed")]
//     VerificationFailed,
// }
//
// /// Proves the correct execution of a circuit using the GKR protocol
// pub fn prove<F: PrimeField>(circuit: &mut Circuit<F>, inputs: &[F]) -> Result<GKRProof<F>, GKRError> {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let inputs_poly = MultilinearPoly::new(inputs.to_vec());
//
//     // Evaluate circuit
//     let circuit_evaluations = circuit
//         .run_circuit(inputs_poly)
//         .map_err(|e| GKRError::CircuitEvaluation(e.to_string()))?;
//
//     // Prepare output polynomial
//     let mut w_0 = circuit_evaluations
//         .first()
//         .ok_or_else(|| GKRError::CircuitEvaluation("Empty circuit evaluation".to_string()))?
//         .polynomial
//         .to_vec();
//     if w_0.len() == 1 {
//         w_0.push(F::zero());
//     }
//     let output_poly = MultilinearPoly::new(w_0);
//
//     let (claimed_sum, random_challenge) = initiate_protocol(&mut transcript, &output_poly);
//
//     let num_layers = circuit.layers.len();
//     let mut proof = ProofBuilder::new(num_layers, claimed_sum);
//
//     for (idx, _) in circuit.layers.iter().enumerate() {
//         let w_i = if idx == num_layers - 1 {
//             inputs.to_vec()
//         } else {
//             circuit_evaluations.get(idx + 1)
//                 .ok_or(GKRError::InvalidLayer(idx + 1))?
//                 .polynomial
//                 .clone()
//         };
//
//         let fbc_poly = if idx == 0 {
//             get_fbc_poly(random_challenge, circuit, idx, &w_i, &w_i)?
//         } else {
//             get_merged_fbc_poly(
//                 circuit,
//                 idx,
//                 &w_i,
//                 &w_i,
//                 &proof.current_rb,
//                 &proof.current_rc,
//                 proof.alpha,
//                 proof.beta,
//             )?
//         };
//
//         proof.process_layer(&mut transcript, fbc_poly, idx, num_layers, &w_i)?;
//     }
//
//     Ok(proof.build(output_poly))
// }
//
// /// Verifies a GKR proof
// pub fn verify<F: PrimeField>(
//     proof: GKRProof<F>,
//     mut circuit: Circuit<F>,
//     inputs: &[F],
// ) -> Result<bool, GKRError> {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let (mut current_claim, init_random_challenge) = initiate_protocol(&mut transcript, &proof.output_poly);
//     let mut verifier = VerifierState::new();
//
//     for (i, layer) in circuit.layers.iter().enumerate() {
//         let sum_check_result = sub_verify(
//             proof.proof_polynomials.get(i)
//                 .ok_or(GKRError::InvalidLayer(i))?.clone(),
//             current_claim,
//             &mut transcript,
//         );
//
//         if !sum_check_result.is_proof_valid {
//             return Ok(false);
//         }
//
//         verifier.update(
//             &proof,
//             &mut circuit,
//             inputs,
//             i,
//             &sum_check_result.random_challenges,
//             init_random_challenge,
//             current_claim,
//             &mut transcript,
//         )?;
//
//         current_claim = verifier.current_claim;
//     }
//
//     Ok(true)
// }
//
// // Helper structs and implementations
//
// struct ProofBuilder<F: PrimeField> {
//     claimed_evaluations: Vec<(F, F)>,
//     proof_polys: Vec<Vec<UnivariatePoly<F>>>,
//     current_rb: Vec<F>,
//     current_rc: Vec<F>,
//     alpha: F,
//     beta: F,
//     claimed_sum: F,
// }
//
// impl<F: PrimeField> ProofBuilder<F> {
//     fn new(num_layers: usize, claimed_sum: F) -> Self {
//         Self {
//             claimed_evaluations: Vec::with_capacity(num_layers.saturating_sub(1)),
//             proof_polys: Vec::with_capacity(num_layers),
//             current_rb: Vec::new(),
//             current_rc: Vec::new(),
//             alpha: F::zero(),
//             beta: F::zero(),
//             claimed_sum,
//         }
//     }
//
//     fn process_layer(
//         &mut self,
//         transcript: &mut Transcript<Keccak256, F>,
//         fbc_poly: SumPoly<F>,
//         idx: usize,
//         num_layers: usize,
//         w_i: &[F],
//     ) -> Result<(), GKRError> {
//         let sum_check_proof = sub_prove(fbc_poly, self.claimed_sum, transcript);
//         self.proof_polys.push(sum_check_proof.round_univariate_polynomials);
//
//         if idx < num_layers - 1 {
//             let next_poly = MultilinearPoly::new(w_i.to_vec());
//             let mid = sum_check_proof.random_challenges.len() / 2;
//             let (r_b, r_c) = sum_check_proof.random_challenges.split_at(mid);
//
//             self.update_challenges(r_b, r_c, &next_poly, transcript)?;
//             self.claimed_sum = self.alpha * self.current_o1 + self.beta * self.current_o2;
//             self.claimed_evaluations.push((self.current_o1, self.current_o2));
//         }
//         Ok(())
//     }
//
//     fn update_challenges(
//         &mut self,
//         r_b: &[F],
//         r_c: &[F],
//         next_poly: &MultilinearPoly<F>,
//         transcript: &mut Transcript<Keccak256, F>,
//     ) -> Result<(), GKRError> {
//         self.current_rb = r_b.to_vec();
//         self.current_rc = r_c.to_vec();
//         self.current_o1 = next_poly.full_evaluation(r_b.to_vec());
//         self.current_o2 = next_poly.full_evaluation(r_c.to_vec());
//
//         transcript.absorb(&to_bytes(&[self.current_o1]));
//         self.alpha = transcript.generate_random_challenge();
//         transcript.absorb(&to_bytes(&[self.current_o2]));
//         self.beta = transcript.generate_random_challenge();
//         Ok(())
//     }
//
//     fn build(self, output_poly: MultilinearPoly<F>) -> GKRProof<F> {
//         GKRProof {
//             output_poly,
//             proof_polynomials: self.proof_polys,
//             claimed_evaluations: self.claimed_evaluations,
//         }
//     }
// }
//
// struct VerifierState<F: PrimeField> {
//     alpha: F,
//     beta: F,
//     prev_challenges: Vec<F>,
//     current_claim: F,
// }
//
// impl<F: PrimeField> VerifierState<F> {
//     fn new() -> Self {
//         Self {
//             alpha: F::zero(),
//             beta: F::zero(),
//             prev_challenges: Vec::new(),
//             current_claim: F::zero(),
//         }
//     }
//
//     fn update(
//         &mut self,
//         proof: &GKRProof<F>,
//         circuit: &mut Circuit<F>,
//         inputs: &[F],
//         i: usize,
//         current_challenges: &[F],
//         init_challenge: F,
//         last_claim: F,
//         transcript: &mut Transcript<Keccak256, F>,
//     ) -> Result<(), GKRError> {
//         let (o_1, o_2) = if i == circuit.layers.len() - 1 {
//             evaluate_input_poly(inputs, current_challenges)
//         } else {
//             proof.claimed_evaluations.get(i)
//                 .copied()
//                 .ok_or(GKRError::InvalidLayer(i))?
//         };
//
//         let expected_claim = if i == 0 {
//             get_verifier_claim(circuit.clone(), i, init_challenge, current_challenges, o_1, o_2)
//         } else {
//             get_merged_verifier_claim(
//                 circuit.clone(),
//                 i,
//                 current_challenges,
//                 &self.prev_challenges,
//                 o_1,
//                 o_2,
//                 self.alpha,
//                 self.beta,
//             )
//         };
//
//         if expected_claim != last_claim {
//             return Err(GKRError::VerificationFailed);
//         }
//
//         self.prev_challenges = current_challenges.to_vec();
//         transcript.absorb(&to_bytes(&[o_1]));
//         self.alpha = transcript.generate_random_challenge();
//         transcript.absorb(&to_bytes(&[o_2]));
//         self.beta = transcript.generate_random_challenge();
//         self.current_claim = self.alpha * o_1 + self.beta * o_2;
//         Ok(())
//     }
// }
//
// // Utility functions remain largely the same but with added error handling
// // ... (implementations of initiate_protocol, tensor_add_mul_polynomials, etc.)
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
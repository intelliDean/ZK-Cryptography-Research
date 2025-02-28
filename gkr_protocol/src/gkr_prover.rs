// use ark_ff::PrimeField;
// use sha3::Keccak256;
// use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
// use sumcheck_protocol::transcript::Transcript;
// use crate::circuit::Circuit;
// use crate::grk_protocol::{fq_vec_to_bytes, get_fbc_poly, GKRProof};
//
// pub fn prove <F: PrimeField> (circuit: &mut Circuit<F>, inputs: &[F]) -> GKRProof<F> {
//     let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
//     let inputs_poly = MultilinearPoly::new(inputs.to_vec());
//     // prover evaluating the circit
//     let mut circuit_evaluations = circuit.run_circuit(inputs_poly);
//     // turn the multilinear poly at index 0 to a vec so you could add to it.
//     let mut w_0 = circuit_evaluations.first().unwrap().polynomial.to_vec();
//
//     if w_0.len() == 1 { // if it's the final output of the circuit
//         w_0.push(F::from(0)); // add 0 to make it a valid evaluation multilinear
//     }
//     let output_poly = MultilinearPoly::new(w_0);
//
//     let (mut claimed_sum, random_challenge) = initiate_protocol(&mut transcript, &output_poly);
//
//     let num_layers = circuit.layers.len();
//
//     let mut claimed_evaluations = Vec::with_capacity(num_layers.saturating_sub(1));
//     let mut proof_polys = Vec::with_capacity(num_layers);
//     let mut current_rb = Vec::new();
//     let mut current_rc = Vec::new();
//     let mut alpha = F::from(0);
//     let mut beta = F::from(0);
//
//     // circuit_evaluations.reverse();
//     let mut layers = circuit.layers.clone();
//     // layers.reverse();
//
//     for (idx, _) in layers.into_iter().enumerate() {
//         let w_i = if idx == num_layers - 1 {
//             inputs.to_vec()
//         } else {
//             circuit_evaluations[idx + 1].polynomial.clone()
//         };
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
//             let o_1 = next_poly.clone().full_evaluation(r_b.to_vec());
//             let o_2 = next_poly.full_evaluation(r_c.to_vec());
//             current_rb = r_b.to_vec();
//             current_rc = r_c.to_vec();
//
//             transcript.absorb(&fq_vec_to_bytes(&[o_1]));
//             alpha = transcript.generate_random_challenge();
//
//             transcript.absorb(&fq_vec_to_bytes(&[o_2]));
//             beta = transcript.generate_random_challenge();
//
//             claimed_sum = (alpha * o_1) + (beta * o_2);
//             claimed_evaluations.push((o_1, o_2));
//         }
//     }
//
//     GKRProof {
//         output_poly,
//         proof_polynomials: proof_polys,
//         claimed_evaluations,
//     }
// }
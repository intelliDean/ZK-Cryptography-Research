use crate::interactive_sumcheck::{
    eval_polynomial, oracle_check, prover_evaluates_claim, verifier_verifies_claim,
};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
// use multilinear_polynomial::multilinear::*;

pub fn operations() {
    let mut poly = Multilinear::new(vec![
        Fr::from(0),
        Fr::from(0),
        Fr::from(0),
        Fr::from(3),
        Fr::from(0),
        Fr::from(0),
        Fr::from(2),
        Fr::from(5),
    ]);

    let eval_result = eval_polynomial(&poly);
    let seq = poly.polynomial.len().trailing_zeros();

    let mut uni_poly = MultilinearPoly::new(eval_result.uni_polys[0].clone());
    let mut claimed_sum = eval_result.claimed_sum;

    let mut rand_chal = Vec::with_capacity(seq as usize - 1); // Pre-allocate space

    for _ in 1..seq {
        let (random_challenge, eval_at_challenge) = verifier_verifies_claim(&uni_poly, claimed_sum);
        rand_chal.push(random_challenge);

        let (univar_poly, next_prover_poly) = prover_evaluates_claim(&poly, random_challenge);
        uni_poly = univar_poly;
        poly = next_prover_poly;

        claimed_sum = eval_at_challenge.polynomial[0];
    }

    let poly = MultilinearPoly::new(vec![
        Fr::from(0),
        Fr::from(0),
        Fr::from(0),
        Fr::from(3),
        Fr::from(0),
        Fr::from(0),
        Fr::from(2),
        Fr::from(5),
    ]);

    let mut rand = MultilinearPoly::new(rand_chal);

    let result = oracle_check(&poly, &uni_poly, claimed_sum, &mut rand);
    println!("Oracle Check Result: {:?}", result);
}

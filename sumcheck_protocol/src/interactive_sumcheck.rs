use crate::prover_sumcheck::Proof;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_std::iterable::Iterable;
use rand::Rng;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};



// (1)
pub(crate) fn eval_polynomial<F: PrimeField>(eval_form: &MultilinearPoly<F>) -> Proof<F> {
    println!("Evaluating polynomial...");
    let seq = eval_form.polynomial.len();
    assert!(
        seq > 1 && (seq & (seq - 1)) == 0,
        "Polynomial must be a power of 2!"
    );
    //the prover evaluates the polynomial at boolean hypercube and sums the result

    let half = seq / 2;
    let (var_at_0, var_at_1) = eval_form.polynomial.split_at(half);

    let sum_0: F = var_at_0.iter().copied().sum();
    let sum_1: F = var_at_1.iter().copied().sum();

    Proof {
        uni_polys: vec![vec![sum_0, sum_1]], // round check univariate polynomial
        claimed_sum: (sum_0 + sum_1),        // the first claimed sum
    }
}

// (2)
pub fn verifier_verifies_claim<F: PrimeField>(
    uni_poly: &MultilinearPoly<F>,
    claimed_sum: F,
) -> (F, MultilinearPoly<F>) {
    println!("Verifier verifying prover claim");

    let half = uni_poly.polynomial.len() / 2;
    let (var_at_0, var_at_1) = uni_poly.polynomial.split_at(half);

    let sum_0: F = var_at_0.iter().copied().sum();
    let sum_1: F = var_at_1.iter().copied().sum();

    let sum_result: F = sum_0 + sum_1;

    assert_eq!(sum_result, claimed_sum, "Rejected!");

    let challenge = generate_random_challenge();
    let eval_at_challenge = uni_poly.clone().partial_evaluation(0, challenge);

    (challenge, eval_at_challenge)
}

fn generate_random_challenge<F: PrimeField>() -> F {
    let mut rng = rand::rng();
    F::from(rng.random_range(0..1000000))
}

// (3)
pub fn prover_evaluates_claim<F: PrimeField>(
    polynomial: &MultilinearPoly<F>,
    random_challenge: F,
) -> (MultilinearPoly<F>, MultilinearPoly<F>) {
    println!("Prover evaluating verifier random challenge...");
    let next_prover_poly = polynomial.clone().partial_evaluation(0, random_challenge);

    let univariate_poly = vec![
        next_prover_poly
            .clone()
            .partial_evaluation(0, F::from(0))
            .polynomial
            .iter()
            .sum(),
        next_prover_poly
            .clone()
            .partial_evaluation(0, F::from(1))
            .polynomial
            .iter()
            .sum(),
    ];

    (Multilinear::new(univariate_poly), next_prover_poly)
}

pub fn partial_evaluation<F: PrimeField>(polynomial: &[F], var_pos: u32, eval_var_at: F) -> Vec<F> {
    let seq = polynomial.len();
    assert!(seq.is_power_of_two(), "Polynomial must be a power of 2!");

    let group_size = 2_u32.pow(seq.trailing_zeros() - var_pos);
    let mut new_polynomial = Vec::with_capacity(seq / 2);

    for chunk in polynomial.chunks((group_size as usize) * 2) {
        for i in 0..group_size as usize {
            if (i + group_size as usize) < chunk.len() {
                let (y_1, y_2) = (chunk[i], chunk[i + group_size as usize]);
                new_polynomial.push(y_1 + eval_var_at * (y_2 - y_1));
            }
        }
    }
    new_polynomial
}
pub(crate) fn full_evaluation<F: PrimeField>(polynomial: &[F], eval_at: Vec<F>) -> Vec<F> {
    let mut poly = polynomial.clone().to_vec();

    for i in (0..eval_at.len()).rev() {
        poly = partial_evaluation(&poly, 1, eval_at[i]);
    }
    poly
}

// (4)
pub fn oracle_check<F: PrimeField>(
    initial_poly: &MultilinearPoly<F>,
    univariate_poly: &MultilinearPoly<F>,
    claimed_sum: F,
    random_challenges: &mut MultilinearPoly<F>,
) -> bool {
    println!("Oracle checking proof ...");
    let (rand_chal, eval_at_challenge) = verifier_verifies_claim(&univariate_poly, claimed_sum);
    random_challenges.polynomial.push(rand_chal);

    let final_partial_eval = univariate_poly.clone().partial_evaluation(0, rand_chal);
    let mut initial_poly = initial_poly.clone();

    for &challenge in random_challenges.polynomial.iter() {
        initial_poly = initial_poly.clone().partial_evaluation(0, challenge);
    }

    if initial_poly == final_partial_eval {
        println!(
            "{:?}",
            (
                eval_at_challenge,
                initial_poly,
                final_partial_eval,
                "Proved Successfully!".to_string(),
            )
        );
        true
    } else {
        println!("Prove not successful!");
        false
    }
}

fn interactive_sumcheck_protocol<F: PrimeField>(
    mut poly: MultilinearPoly<F>,
) -> (MultilinearPoly<F>, F, MultilinearPoly<F>) {
    let eval_result = eval_polynomial(&poly);

    let seq = poly.polynomial.len().trailing_zeros();

    let mut uni_poly = Multilinear::new(eval_result.uni_polys[0].clone());
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

    (uni_poly, claimed_sum, Multilinear::new(rand_chal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::multilinear::multilinear::to_field;

    fn get_polynomial() -> MultilinearPoly<Fr> {
        MultilinearPoly::new(to_field(vec![0, 0, 0, 3, 0, 0, 2, 5]))
    }

    #[test]
    fn test_interactive_sumcheck() {
        let poly = get_polynomial();
        let mut result = interactive_sumcheck_protocol(poly);

        let v_poly = get_polynomial();

        let res = oracle_check(&v_poly, &result.0, result.1, &mut result.2);
        assert_eq!(res, true)
    }
    #[test]
    fn interactive_sumcheck_to_return_false() {
        let poly = get_polynomial();
        let mut result = interactive_sumcheck_protocol(poly);

        let v_poly = MultilinearPoly::new(to_field(vec![0, 0, 0, 3, 3, 0, 2, 5]));

        let res = oracle_check(&v_poly, &result.0, result.1, &mut result.2);
        assert_eq!(res, false);
    }
}

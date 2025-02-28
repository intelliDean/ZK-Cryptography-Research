use crate::transcript::{HashTrait, Transcript};
use ark_ff::{BigInteger, PrimeField};
use polynomials::sum::sum_poly::SumPoly;
use polynomials::univariate::uni_point::{Points, XAndY};
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use polynomials::product::product_poly::ProductPoly;

#[derive(Clone, Debug)]
pub struct GKRSumcheckProverProof<F: PrimeField> {
    pub claimed_sum: F,
    pub round_univariate_polynomials: Vec<UnivariatePoly<F>>,
    pub random_challenges: Vec<F>
}

#[derive(Clone, Debug)]
pub struct GKRSumcheckVerifierProof<F: PrimeField> {
    pub is_proof_valid: bool,
    pub random_challenges: Vec<F>,
    pub last_claimed_sum: F,
}

#[derive(Clone, Debug)]
pub struct GKRVerifierProof<F: PrimeField> {
    pub round_univariate_polynomials: Vec<UnivariatePoly<F>>,
    pub claimed_sum: F
}

pub fn prove<K: HashTrait, F: PrimeField>(
    sum_polynomial: SumPoly<F>,
    claimed_sum: F,
    transcript: &mut Transcript<K, F>,
) -> GKRSumcheckProverProof<F> {

    let num_vars = sum_polynomial.num_vars();

    let mut round_univariate_polynomials = Vec::new();
    let mut random_challenges = Vec::with_capacity(num_vars as usize);
    let mut current_polynomial = sum_polynomial.clone();

    transcript.absorb(&field_element_to_bytes(claimed_sum));

    for _round in 0..num_vars {
        let uni_poly = generate_round_univariate(&current_polynomial);
        transcript.absorb(&univariate_to_bytes(&uni_poly));

        round_univariate_polynomials.push(uni_poly);

        let random_challenge: F = transcript.generate_random_challenge();
        random_challenges.push(random_challenge);

        current_polynomial = current_polynomial.partial_evaluate(0, random_challenge);
    }

    // GKRVerifierProof {
    //     claimed_sum,
    //     round_univariate_polynomials,
    // }

    GKRSumcheckProverProof {
        claimed_sum,
        round_univariate_polynomials,
        random_challenges
    }
}

pub fn verify<K: HashTrait, F: PrimeField>(
    round_univariate_polynomials: Vec<UnivariatePoly<F>>,
    claimed_sum: F,
    transcript: &mut Transcript<K, F>,
) -> GKRSumcheckVerifierProof<F> {
    transcript.absorb(&field_element_to_bytes(claimed_sum));

    let mut current_sum = claimed_sum;
    let mut random_challenges = Vec::with_capacity(round_univariate_polynomials.len());


    for round_polynomial in &round_univariate_polynomials {

        let eval_at_zero = round_polynomial.full_coeff_evaluation(F::zero());
        let eval_at_one = round_polynomial.full_coeff_evaluation(F::one());

        if eval_at_zero + eval_at_one != current_sum {
            return GKRSumcheckVerifierProof {
                is_proof_valid: false,
                random_challenges: vec![],
                last_claimed_sum: current_sum, // this might give issue eventually
            };
        }

        transcript.absorb(&univariate_to_bytes(round_polynomial));

        let random_challenge = transcript.generate_random_challenge();

        current_sum = round_polynomial.full_coeff_evaluation(random_challenge);

        random_challenges.push(random_challenge);
    }

    GKRSumcheckVerifierProof {
        is_proof_valid: true,
        random_challenges,
        last_claimed_sum: current_sum,
    }
}

pub fn generate_round_univariate<F: PrimeField>(current_polynomial: &SumPoly<F>) -> UnivariatePoly<F> /*Vec<F>*/ {
    let degree = current_polynomial.degree();
    let num_evaluations = degree + 1;

    // let mut evaluations = Vec::with_capacity(num_evaluations);
    let mut x_and_y = Vec::with_capacity(num_evaluations);

    for i in 0..num_evaluations {
        let x = F::from(i as u64);
        let partial_eval_sum_poly = current_polynomial.partial_evaluate(0, x); // holding Vec<ProductPoly> : length of 2
        let y = partial_eval_sum_poly
            .sum_element_by_element()
            .polynomial
            .iter()
            .sum();

        x_and_y.push(XAndY::new(x, y));
    }

   let points =  Points::new(x_and_y);
    points.lagrange_interpolate()
}

pub fn univariate_to_bytes<F: PrimeField>(poly: &UnivariatePoly<F>) -> Vec<u8> {
    // Serialize the degree first
    let degree_bytes = poly.degree.into_bigint().to_bytes_le();

    // Serialize each term in co_ex (coeff followed by exp)
    let terms_bytes = poly.co_ex.iter().flat_map(|term| {
        let coeff_bytes = term.coeff.into_bigint().to_bytes_le();
        let exp_bytes = term.exp.into_bigint().to_bytes_le();
        coeff_bytes.into_iter().chain(exp_bytes.into_iter())
    });

    // Combine degree bytes and terms bytes into a single Vec<u8>
    degree_bytes.into_iter().chain(terms_bytes).collect()
}

pub fn field_element_to_bytes<F: PrimeField>(field_element: F) -> Vec<u8> {
    field_element.into_bigint().to_bytes_be()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fq;
    use sha3::{Digest, Keccak256};
    use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

    #[test]
    fn test_generate_round_univariate() {

        let sum_polynomial = SumPoly::new(vec![
            ProductPoly::new(vec![
                MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]),
                MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)])
            ]),
            ProductPoly::new(vec![
                MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]),
                MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)])
            ])
        ]);

        let univariate_poly = generate_round_univariate(&sum_polynomial);

        println!("Round Poly: {:?}", univariate_poly);
        assert_eq!(
            univariate_poly,
            UnivariatePoly::new(vec![Term::new(Fq::from(12), Fq::from(2))])
        );
    }

    #[test]
    fn test_prover_and_verifier() {
        let poly1a =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly2a =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly2a]);

        let poly1b =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly2b =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly2 = ProductPoly::new(vec![poly1b, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        let mut prover_transcript = Transcript::<Keccak256, Fq>::init(Keccak256::new());
        let mut verifier_transcript = Transcript::<Keccak256, Fq>::init(Keccak256::new());

        let result = prove(sum_polynomial, Fq::from(12), &mut prover_transcript);
        println!("GKRVerifierProof: {:?}", result);

        let verified = verify(result.round_univariate_polynomials, result.claimed_sum, &mut verifier_transcript);
        println!("===============================================");
        println!("GKRSumcheckVerifierProof: {:?}", verified);

        assert_eq!(verified.is_proof_valid, true);
    }

    #[test]
    fn test_to_make_prover_and_verifier_to_fail() {
        let poly1a =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly2a =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly2a]);

        let poly1b =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly2b =
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly2 = ProductPoly::new(vec![poly1b, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        let mut prover_transcript = Transcript::<Keccak256, Fq>::init(Keccak256::new());
        let mut verifier_transcript = Transcript::<Keccak256, Fq>::init(Keccak256::new());

        let result = prove(sum_polynomial, Fq::from(12), &mut prover_transcript);
        println!("GKRVerifierProof: {:?}", result);

        let wrong_proof = GKRSumcheckProverProof {
            round_univariate_polynomials: vec![
                UnivariatePoly::new(vec![Term::new(Fq::from(12), Fq::from(2))]),
                UnivariatePoly::new(vec![Term::new(Fq::from(3000), Fq::from(200))]),
            ],
            claimed_sum: Fq::from(12),
            random_challenges: vec![],
        };

        let verified = verify(wrong_proof.round_univariate_polynomials, wrong_proof.claimed_sum, &mut verifier_transcript);
        println!("===============================================");
        println!("GKRSumcheckVerifierProof: {:?}", verified);

        assert_eq!(verified.is_proof_valid, false);
    }
}

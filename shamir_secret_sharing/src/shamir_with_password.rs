use ark_ff::{BigInteger, PrimeField};
use ark_std::test_rng;
use polynomials::univariate::uni_point::{Points, XAndY};
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use crate::shamir_secret_sharing::comp;


fn generate_polynomial_with_password<F: PrimeField>(
    secret: F,
    degree: F,
    password: F,
) -> UnivariatePoly<F> {
    let mut rng = test_rng();
    let mut coefficients = Vec::new();

    // Generate random coefficients up to degree - 1
    let degree_int = degree.into_bigint().as_ref()[0] as usize;
    for i in 1..degree_int {
        let rand = F::rand(&mut rng);
        println!("{:?}, {:?}", i, rand);
        coefficients.push(Term::new(rand, F::from(i as u64)));
    }
    println!("Coefficients: {:?}", coefficients);

    // Compute constant term: secret - p(password)
    let constant = secret - compute_coeff(&coefficients, password);
    coefficients.push(Term::new(constant, F::zero()));

    UnivariatePoly::new(coefficients)
}

fn compute_coeff<F: PrimeField>(uni_poly: &[Term<F>], password: F) -> F {
    uni_poly
        .iter()
        .map(|term| term.coeff * password.pow(term.exp.into_bigint().as_ref()))
        .fold(F::zero(), |acc, val| acc + val)
}

fn reconstruct_secret_with_password<F: PrimeField>(shares: &[XAndY<F>], password: F) -> F {
    let mut secret = F::zero();

    for (i, point_i) in shares.iter().enumerate() {
        let mut numerator = F::one();
        let mut denominator = F::one();

        for (j, point_j) in shares.iter().enumerate() {
            if i != j {
                numerator *= password - point_j.x; // Use password instead of 0
                denominator *= point_i.x - point_j.x;
            }
        }

        let denominator_inv = denominator
            .inverse()
            .expect("Denominator must have an inverse");
        let term = point_i.y * numerator * denominator_inv;
        secret += term;
    }

    secret
}

fn generate_shares<F: PrimeField>(
    secret: F,
    num_shares: F,
    threshold: F,
    password: F,
) -> Points<F> {
    let univariate_poly = generate_polynomial_with_password(secret, threshold, password);

    let mut shares = Vec::new();
    comp(num_shares, &univariate_poly, &mut shares);
    Points { points: shares }
}

pub fn operation<F: PrimeField>(secret: F, num_shares: F, threshold: F, password: F) -> F {
    println!("Sharing secret: {:?}", secret);

    let shares = generate_shares(secret, num_shares, threshold, password);

    println!("Secret is divided into {} parts:", num_shares);
    for point in &shares.points {
        println!("x: {:?}, y: {:?}", point.x, point.y);
    }

    let threshold_usize = threshold.into_bigint().as_ref()[0] as usize;
    reconstruct_secret_with_password(&shares.points[0..threshold_usize], password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;

    #[test]
    fn test_generate_polynomial() {
        let secret = Fr::from(9023430);
        let points = Fr::from(3);
        let password = Fr::from(212);

        let uni_poly = generate_polynomial_with_password(secret, points, password);
        assert_eq!(uni_poly.degree, Fr::from(2));
        assert_eq!(uni_poly.co_ex.len(), 3);
    }

    #[test]
    fn test_generate_shares() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let password = Fr::from(212);
        let shares = generate_shares(secret, num_share, threshold, password);

        assert_eq!(
            shares.points.len(),
            10
        );
    }

    #[test]
    fn test_regenerate_secret() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let password = Fr::from(212);
        let coefficients = generate_shares(secret, num_share, threshold, password);

        let reg_secret = reconstruct_secret_with_password(&coefficients.points, Fr::from(212));
        assert_eq!(reg_secret, secret);
    }

    #[test]
    fn test_operation() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let password = Fr::from(212);

        let reg_secret = operation(secret, num_share, threshold, password);
        assert_eq!(reg_secret, Fr::from(9023430));
    }
}
use crate::shamir_secret_sharing::comp;
use ark_ff::{BigInteger, PrimeField};
use ark_std::test_rng;
use polynomials::univariate::point::Points;
use polynomials::univariate::univariate_polynomial::UnivariatePoly;

fn generate_polynomial_with_password <F: PrimeField>(secret: F, degree: F, password: F) -> UnivariatePoly<F> {

    let mut rng = test_rng();

    // Start with the secret as the constant term
    let mut coefficients = vec![];
    for i in 1..degree.into_bigint().as_ref()[0] {
        // coefficients.push(F::from(rng.random_range(1..1000u64)));
        let rand = F::rand(&mut rng);
        println!("{:?}, {:?}", i, rand);
        coefficients.push((rand, F::from(i)));
    }
    println!("Coefficients: {:?}", coefficients);
    let constant: F = secret - compute_coeff(&coefficients, password);

    coefficients.push((constant, F::zero()));

    UnivariatePoly::new(coefficients)
}

fn compute_coeff<F: PrimeField>(uni_poly: &Vec<(F, F)>, password: F) -> F {
    let mut result: F = F::zero();

    for (i, (coeff, expo)) in uni_poly.iter().enumerate() {
        result += coeff.clone() * password.pow(expo.into_bigint().as_ref());
    }
    result
}

fn reconstruct_secret_with_password<F: PrimeField>(shares: &[(F, F)], password: F) -> F {
    let mut secret = F::zero();

    for (i, &(x_i, y_i)) in shares.iter().enumerate() {
        let mut numerator = F::one();
        let mut denominator = F::one();

        for (j, &(x_j, _)) in shares.iter().enumerate() {
            if i != j {
                numerator *= password - x_j; // Use password instead of 0
                denominator *= x_i - x_j;
            }
        }
        // Use the modular inverse from PrimeField
        let denominator_inv = denominator
            .inverse()
            .expect("Denominator must have an inverse");

        let term = y_i * numerator * denominator_inv;
        secret += term;
    }

    secret
}

fn generate_shares<F: PrimeField>(secret: F, num_shares: F, threshold: F, password: F) -> Points<F> {
    let univariate_poly = generate_polynomial_with_password(secret, threshold, password);

    let mut shares = Vec::new();
    comp(num_shares, &univariate_poly, &mut shares);
    Points::new(shares)
}

pub fn operation<F: PrimeField>(secret: F, num_shares: F, threshold: F, password: F) -> F {
    println!("Sharing secret: {:?}", secret);

    let shares = generate_shares(secret, num_shares, threshold, password);

    println!("Secret is divided into {} parts:", num_shares);
    for (x, y) in &shares.x_y {
        println!("x: {}, y: {}", x, y);
    }

    let threshold_usize: usize = threshold.into_bigint().to_bytes_le()[0] as usize;

    reconstruct_secret_with_password(&shares.x_y[0..threshold_usize], password)
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
            shares.x_y.len(),
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

        let reg_secret = reconstruct_secret_with_password(&coefficients.x_y, Fr::from(212));
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
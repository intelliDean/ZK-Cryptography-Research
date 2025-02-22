use ark_ff::{BigInteger, Field, One, PrimeField, Zero};
use ark_std::{test_rng, UniformRand};
use ark_std::iterable::Iterable;
use polynomials::univariate::point::Points;
use polynomials::univariate::univariate_polynomial::UnivariatePoly;


/// Generate the polynomial for secret sharing
fn generate_polynomial<F: PrimeField>(secret: F, degree: F) -> UnivariatePoly<F> {
    //UNIVARIATE POLYNOMIAL
    let mut rng = test_rng();
    // Start with the secret as the constant term
    //adding the secret the 0 index, making it the constant
    let mut coefficients = vec![(secret, F::zero())];
    for i in 1..degree.into_bigint().as_ref()[0] {
        // coefficients.push(F::from(rng.random_range(1..1000u64)));
        coefficients.push((F::rand(&mut rng), F::from(i)));
    }
    UnivariatePoly::new(coefficients)
}

/// Generate shares based on the polynomial
fn generate_shares<F: PrimeField>(secret: F, num_shares: F, threshold: F) -> Points<F> {
    let univariate_poly = generate_polynomial(secret, threshold);

    let mut shares = Vec::new();

    comp(num_shares, &univariate_poly, &mut shares);
    Points::new(shares)
}

pub fn comp<F: PrimeField>(num_shares: F, univariate_poly: &UnivariatePoly<F>, shares: &mut Vec<(F, F)>) {
    for i in 1..=num_shares.into_bigint().as_ref()[0] {
        let x = F::from(i);
        let y = evaluate_poly_to_get_y(x, &univariate_poly);
        shares.push((x, y));
    }
}

/// Calculate the polynomial value at a given x
pub fn evaluate_poly_to_get_y<F: PrimeField>(x: F, univariate_poly: &UnivariatePoly<F>) -> F {
    univariate_poly
        .co_ex
        .iter()
        .map(|(coeff, exp)| *coeff * x.pow(exp.into_bigint().as_ref()))
        .sum()
}




/// Reconstruct the secret from shares
fn reconstruct_secret<F: PrimeField>(shares: &[(F, F)]) -> F {
    let mut secret = F::zero();

    for (i, &(x_i, y_i)) in shares.iter().enumerate() {
        let mut numerator = F::one();
        let mut denominator = F::one();

        for (j, &(x_j, _)) in shares.iter().enumerate() {
            if i != j {
                numerator *= x_j;
                denominator *= x_j - x_i;
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

/// Main function demonstrating secret sharing
pub fn operation<F: PrimeField>(secret: F, num_shares: F, threshold: F) -> F {
    println!("Sharing secret: {:?}", secret);

    let shares = generate_shares(secret, num_shares, threshold);

    println!("Secret is divided into {} parts:", num_shares);
    for (x, y) in &shares.x_y {
        println!("x: {}, y: {}", x, y);
    }

    let threshold_usize: usize = threshold.into_bigint().to_bytes_le()[0] as usize;

    reconstruct_secret(&shares.x_y[0..threshold_usize])
}

#[cfg(test)]
mod tests {

    use super::*;
    use ark_bn254::Fr;

    #[test]
    fn test_generate_polynomial() {
        let secret = Fr::from(9023430);
        let degree = Fr::from(3);

        let coefficients = generate_polynomial(secret, degree);
        assert_eq!(
            coefficients.co_ex.len(),
            degree.into_bigint().as_ref()[0] as usize
        );
    }

    #[test]
    fn test_generate_shares() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let shares = generate_shares(secret, num_share, threshold);

        assert_eq!(
            shares.x_y.len(),
            num_share.into_bigint().as_ref()[0] as usize
        );
    }

    #[test]
    fn test_regenerate_secret() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let coefficients = generate_shares(secret, num_share, threshold);

        let reg_secret = reconstruct_secret(&coefficients.x_y);
        assert_eq!(reg_secret, secret);
    }

    #[test]
    fn test_operation() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);

        let reg_secret = operation(secret, num_share, threshold);
        assert_eq!(reg_secret, Fr::from(9023430));
    }
}

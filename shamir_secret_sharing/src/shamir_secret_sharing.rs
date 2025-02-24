use ark_ff::{BigInteger, Field, One, PrimeField, Zero};
use ark_std::{test_rng, UniformRand};
use ark_std::iterable::Iterable;
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use polynomials::univariate::uni_point::{Points, XAndY};

fn generate_polynomial<F: PrimeField>(secret: F, degree: F) -> UnivariatePoly<F> {
    //UNIVARIATE POLYNOMIAL
    let mut rng = test_rng();
    // Start with the secret as the constant term
    let mut coefficients = vec![Term::new(secret, F::zero())];
    // Generate random coefficients up to degree - 1
    let degree_int = degree.into_bigint().as_ref()[0] as usize;
    for i in 1..degree_int {
        let coeff = F::rand(&mut rng);
        let exp = F::from(i as u64);
        coefficients.push(Term::new(coeff, exp));
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

pub fn comp<F: PrimeField>(
    num_shares: F,
    univariate_poly: &UnivariatePoly<F>,
    shares: &mut Vec<XAndY<F>>,
) {
    let num_shares_int = num_shares.into_bigint().as_ref()[0] as usize;
    for i in 1..=num_shares_int {
        let x = F::from(i as u64);
        let y = evaluate_poly_to_get_y(x, univariate_poly);
        shares.push(XAndY::new(x, y));
    }
}

/// Calculate the polynomial value at a given x
pub fn evaluate_poly_to_get_y<F: PrimeField>(x: F, univariate_poly: &UnivariatePoly<F>) -> F {
    univariate_poly
        .co_ex
        .iter()
        .map(|term| term.coeff * x.pow(term.exp.into_bigint().as_ref()))
        .fold(F::zero(), |acc, val| acc + val)
}

/// Reconstruct the secret from shares using Lagrange interpolation
fn reconstruct_secret<F: PrimeField>(shares: &[XAndY<F>]) -> F {
    let mut secret = F::zero();

    for (i, point_i) in shares.iter().enumerate() {
        let mut numerator = F::one();
        let mut denominator = F::one();

        for (j, point_j) in shares.iter().enumerate() {
            if i != j {
                numerator *= point_j.x;
                denominator *= point_j.x - point_i.x;
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

/// Main function demonstrating secret sharing
pub fn operation<F: PrimeField>(secret: F, num_shares: F, threshold: F) -> F {
    println!("Sharing secret: {:?}", secret);

    let shares = generate_shares(secret, num_shares, threshold);

    println!("Secret is divided into {} parts:", num_shares);
    for point in &shares.points {
        println!("x: {:?}, y: {:?}", point.x, point.y);
    }

    let threshold_usize = threshold.into_bigint().as_ref()[0] as usize;
    reconstruct_secret(&shares.points[0..threshold_usize])
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
            shares.points.len(),
            num_share.into_bigint().as_ref()[0] as usize
        );
    }

    #[test]
    fn test_regenerate_secret() {
        let secret = Fr::from(9023430);
        let threshold = Fr::from(3);
        let num_share = Fr::from(10);
        let coefficients = generate_shares(secret, num_share, threshold);

        let reg_secret = reconstruct_secret(&coefficients.points);
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

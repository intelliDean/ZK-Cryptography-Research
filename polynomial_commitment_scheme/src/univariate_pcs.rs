use ark_bn254::{Bn254, Fr, FrConfig, G1Projective, G2Projective}; // Use BLS12-381 if you want
use ark_ec::pairing::Pairing;
use ark_ec::{Group, VariableBaseMSM};
use ark_ff::{Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::{univariate::DensePolynomial, Polynomial};
use polynomials::univariate::uni_poly::UnivariatePoly;
use rand::thread_rng;
use std::borrow::Borrow;
use std::ops::Mul;

use ark_bls12_381::{Bls12_381, Config};
use ark_poly_commit::kzg10::{UniversalParams, KZG10};

pub struct PowersOfTau {
    pub powers_of_g: Vec<G1Projective>,
}

// fn setup(srs_size: usize) -> (Vec<G1Projective>, G2Projective) {
//     let rng = &mut thread_rng();
//     let tau = Fr::rand(rng); // Secret scalar (not revealed)
//
//     let g1 = G1Projective::rand(rng); // Generator in G1
//     let g2 = G2Projective::rand(rng); // Generator in G2 (for verification)
//
//     // Compute G1 powers: [g, g^tau, g^(tau^2), ...]
//     let g1_powers: Vec<G1Projective> = (0..srs_size)
//         .scan(g1, |state, _| {
//             let next = *state;
//             *state *= tau;
//             Some(next)
//         })
//         .collect();
//
//     (g1_powers, g2)
// }

fn setup(srs_size: usize) -> (Vec<G1Projective>, G2Projective) {
    let rng = &mut thread_rng();
    let tau = Fr::rand(rng); // Secret scalar (not revealed)

    let g1 = G1Projective::rand(rng); // Generator in G1
    let g2 = G2Projective::rand(rng); // Generator in G2 (for verification)

    // Compute G1 powers: [g, g^tau, g^(tau^2), ...]
    let mut g1_powers = Vec::with_capacity(srs_size);
    let mut current = g1;

    for _ in 0..srs_size {
        g1_powers.push(current);
        current *= tau;
    }

    (g1_powers, g2)
}

fn trusted_setup<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    setup_size: usize,
    contributions: &[F],
) -> (Vec<G1Projective>, G2Projective) {
    let rng = &mut thread_rng();
    // let tau = Fr::rand(rng); // Secret scalar (not revealed)

    let g1 = G1Projective::rand(rng); // Generator in G1y
    let mut g2 = G2Projective::rand(rng); // Generator in G2 (for verification)

    let mut trusted_setup = vec![g1.clone(); setup_size];
    println!("G2 Before: {:?}", g2);

    for (i, each_cont) in contributions.iter().enumerate() {
        for i in 1..setup_size {
            let cont = each_cont.pow(&[i as u64]);
            // let result: G1Projective = generator_with_scalar::<F>(trusted_setup[i], cont.into_bigint().as_ref()[0] as u32);
            trusted_setup[i] = trusted_setup[i].mul(cont);
            if i == 1 {
                g2 = g2.mul(cont);
            }
        }
        println!("After Cont {}: {:?}", i + 1, trusted_setup);
    }
    println!("G2 After: {:?}", g2);

    (trusted_setup, g2)
}

 pub fn commit_to_polynomial<F: PrimeField>() {

 }

// fn commit(
//     poly: &DensePolynomial<Fr>,
//     srs: &[G1Projective]
// ) -> G1Projective {
//     let coeffs = &poly.coeffs;
//     VariableBaseMSM::multi_scalar_mul(&srs[..coeffs.len()], coeffs)
// }

fn verify(commitment: G1Projective, proof: G1Projective, x: Fr, y: Fr, g2: G2Projective) -> bool {
    let lhs = Bn254::pairing(commitment - proof * x, g2);
    let rhs = Bn254::pairing(proof, g2) * y;
    lhs == rhs
}

#[cfg(test)]
mod tests {
    use super::*;
    // use ark_bn254::Fr;
    use ark_bn254::{Fr, FrConfig};

    #[test]
    fn test_setup() {
        let result = setup(2);
        println!("{:?}", result);
    }
    // #[test]
    // fn test_trusted_setup() {
    //
    //     let cont = [Fr::from(3), Fr::from(2)];
    //
    //     let result = trusted_setup(3, cont);
    //     println!("{:?}", result);
    // }

    #[test]
    fn test_trusted_setup() {
        let setup_size = 12; // Number of elements in the setup
        let rng = &mut thread_rng();

        // Generate contributions (random values for now)
        // let contributions: Vec<Fr> = (0..3).map(|_| Fr::rand(rng)).collect();

        let contributions = &[Fr::from(13), Fr::from(62),Fr::from(123), Fr::from(952),Fr::from(336), Fr::from(122),Fr::from(231), Fr::from(1202)];
        // Run the trusted setup function
        let result = trusted_setup(setup_size, contributions);

        // Check that the result has the correct size
        assert_eq!(
            result.0.len(),
            setup_size,
            "Output vector should have the same size as setup_size"
        );

        // Ensure values are not just the initial generator (i.e., contributions applied)
        let g1 = G1Projective::default();
        assert!(
            result.0.iter().any(|&x| x != g1),
            "At least one element should be different from the initial generator"
        );

        // Additional sanity checks (optional)
        for i in 0..setup_size {
            assert_ne!(
                result.0[i],
                G1Projective::default(),
                "No element should be the identity (zero point)"
            );
        }
    }
}

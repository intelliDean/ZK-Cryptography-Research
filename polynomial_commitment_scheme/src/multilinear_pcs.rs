use ark_bn254::{Fr, FrConfig, G1Projective as G1, G2Projective as G2};
use ark_ec::pairing::Pairing;
use ark_ec::{PrimeGroup, VariableBaseMSM};
use ark_ff::{Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::iterable::Iterable;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::Mul;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TrustedSetup<F: PrimeField> {
    pub powers_of_tau: Vec<G1>,
    pub g2_tau: Vec<G2>,
    _phantom: PhantomData<F>,
}
impl<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>> TrustedSetup<F> {
    fn new(powers_of_tau: Vec<G1>, g2_tau: Vec<G2>) -> Self {
        Self {
            powers_of_tau,
            g2_tau,
            _phantom: PhantomData,
        }
    }
    pub fn multilinear_trusted_setup(multilinear: MultilinearPoly<F>, tau_var: &[F]) -> Self {
        assert_eq!(
            multilinear.num_var(),
            tau_var.len() as u32,
            "Inconsistent variable size"
        );

        let g1 = G1::generator();
        let g2 = G2::generator();

        let bhc = generate_hypercube(multilinear.polynomial.len());
        println!("BHC: {:?}", bhc);
        let mut g1_evals = Vec::with_capacity(bhc.len());
        let g2_evals: Vec<G2> = tau_var.iter().map(|&tau| g2.mul(tau)).collect();

        for combinations in bhc {
            let mut res = F::one();

            for (i, &bit) in combinations.iter().enumerate() {
                res *= if bit {
                    tau_var[i]
                } else {
                    F::one() - tau_var[i]
                };
            }

            g1_evals.push(g1.mul(res));
        }

        println!("G1: {:?}", g1_evals);
        println!("G2: {:?}", g2_evals);
        TrustedSetup::new(g1_evals, g2_evals)
    }
}

fn generate_hypercube(n: usize) -> Vec<Vec<bool>> {
    assert!(
        n > 1 && n.is_power_of_two(),
        "Required power of 2 but got {}",
        n
    );

    let bits = n.trailing_zeros() as usize; //log₂(n)
    let total_combinations = 1 << bits;

    (0..total_combinations) //returns a bool for the boolean hypercube
        .map(|i| (0..bits).map(|b| (i & (1 << b)) != 0).rev().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_hypercube() {
        let result = generate_hypercube(4);
        let expected = vec![
            vec![false, false], // 00
            vec![false, true], //  01
            vec![true, false], //  10
            vec![true, true], //   11
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multilinear_setup() {
        let res = TrustedSetup::multilinear_trusted_setup(
            MultilinearPoly::new(vec![
                Fr::from(3),
                Fr::from(2),
                Fr::from(2),
                Fr::from(2),
                Fr::from(4),
                Fr::from(5),
                Fr::from(8),
                Fr::from(9),
            ]),
            &[Fr::from(2), Fr::from(6), Fr::from(6)],
        );
    }
}

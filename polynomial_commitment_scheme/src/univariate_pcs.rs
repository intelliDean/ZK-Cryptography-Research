use ark_bn254::{Bn254, Config, Fr, FrConfig, G1Projective as G1, G2Projective as G2};
// Use BLS12-381 if you want
use ark_ec::bls12::G1Projective;
use ark_ec::pairing::Pairing;
use ark_ec::twisted_edwards::Projective;
use ark_ec::{PrimeGroup, VariableBaseMSM};
use ark_ff::{Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::test_rng;
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use std::borrow::Borrow;
use std::ops::Mul;
// use ark_poly::univariate::DensePolynomial;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TrustedSetup {
    pub powers_of_tau: Vec<G1>,
    pub g2_tau: G2,
}

impl TrustedSetup {
    fn new(powers_of_tau: Vec<G1>, g2_tau: G2) -> Self {
        Self {
            powers_of_tau,
            g2_tau,
        }
    }

    fn trusted_setup<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
        setup_size: usize,
        contributions: &[F],
    ) -> TrustedSetup {
        let rng = &mut test_rng();

        // let tau = Fr::rand(rng); // Secret scalar (not revealed)

        let g1 = G1::generator(); // Generator in G1y
        let mut g2 = G2::generator(); // Generator in G2 (for verification)

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

        TrustedSetup::new(trusted_setup, g2)
    }

    pub fn get_tau_in_both_groups(&self) -> (G1, G2) {
        (self.powers_of_tau[1], self.g2_tau)
    }

    pub fn commit_to_polynomial<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
        &self,
        uni_poly: &UnivariatePoly<F>,
    ) -> G1 {
        let coefficients = extract_coefficients(uni_poly);

        if coefficients.len() > self.powers_of_tau.len() {
            panic!("Not enough powers of tau for the polynomial degree");
        }
        //identity element
        let mut eval = G1::default();

        //eval = ∑ (coeff[i] * powers_of_tau[i])
        for (i, coefficient) in coefficients.iter().enumerate() {
            let term = self.powers_of_tau[i].mul(*coefficient); // scalar multiplication
            eval += term; // group addition
        }
        eval
    }

    fn random_setup(setup_size: usize) -> (Vec<G1>, G2) {
        let rng = &mut test_rng();
        let tau = Fr::rand(rng); // secret scalar (SECRET)

        let g1 = G1::generator(); // Generator in G1
        let g2 = G2::generator(); // Generator in G2 (for verification)

        // compute G1 powers: [g, g^tau, g^(tau^2), ...]
        let mut g1_powers = Vec::with_capacity(setup_size);
        let mut current = g1;

        for _ in 0..setup_size {
            g1_powers.push(current);
            current *= tau;
        }

        (g1_powers, g2)
    }

    pub fn open_polynomial<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
        &self,
        uni_poly: UnivariatePoly<F>,
        eval_at: F,
    ) -> (F, F, G1) {
        let v = uni_poly.full_coeff_evaluation(eval_at);

        for mut term in uni_poly.co_ex.clone() {
            if term.exp == F::zero() {
                term.coeff = term.coeff - v;
                break;
            }
        }

        let divisor = UnivariatePoly::new(vec![
            Term::new(F::one(), F::one()),
            Term::new(eval_at, F::zero()),
        ]);

        let q_x = uni_poly.divide_polynomials(divisor);
        let mut dense = vec![F::zero(); q_x.0.co_ex.len()];

        for q in q_x.0.co_ex.clone() {
            dense[q.exp.into_bigint().as_ref()[0] as usize] += q.coeff;
        }

        let mut q_t = G1::default();

        for i in 0..dense.len() {
            let res = self.powers_of_tau[i].mul(dense[i]);
            q_t += res;
        }

        println!("Result: {:?}", dense);
        println!("Result: {:?}", q_t);

        (eval_at, v, q_t)
    }

    // pub fn verifier_verifies(commitment: G1, v: ) {
    //
    // }
}

fn extract_coefficients<F: PrimeField>(uni_poly: &UnivariatePoly<F>) -> Vec<F> {
    let mut coefficients = vec![F::zero(); uni_poly.co_ex.len()];

    for term in &uni_poly.co_ex {
        coefficients[term.exp.into_bigint().as_ref()[0] as usize] = term.coeff;
    }
    println!("Coefficients: {:?}", coefficients);
    coefficients
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

// Random setup

fn verify(commitment: G1, proof: G1, x: Fr, y: Fr, g2: G2) -> bool {
    let lhs = Bn254::pairing(commitment - proof * x, g2);
    let rhs = Bn254::pairing(proof, g2) * y;
    lhs == rhs
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::univariate::uni_poly::Term;

    fn get_uni_poly() -> UnivariatePoly<Fr> {
        UnivariatePoly::new(vec![
            Term::new(Fr::from(2), Fr::from(2)),
            Term::new(Fr::from(6), Fr::from(1)),
            Term::new(Fr::from(12), Fr::from(0)),
        ])
    }

    fn get_trusted_setup<F: PrimeField>() -> TrustedSetup {
        let cont = &[
            Fr::from(13),
            Fr::from(62),
            Fr::from(123),
            Fr::from(952),
            Fr::from(336),
            Fr::from(122),
            Fr::from(231),
            Fr::from(1202),
        ];

        let trusted_setup = TrustedSetup::trusted_setup(3, cont);

        trusted_setup
    }

    #[test]
    fn test_setup() {
        let result = TrustedSetup::random_setup(2);
        println!("{:?}", result);
    }

    #[test]
    fn test_trusted_setup() {
        // let rng = &mut test_rng();
        // Generate contributions (random values for now)
        // let contributions: Vec<Fr> = (0..3).map(|_| Fr::rand(rng)).collect();

        let setup_size = 12; // number of elements in the setup

        let contributions = &[
            Fr::from(13),
            Fr::from(62),
            Fr::from(123),
            Fr::from(952),
            Fr::from(336),
            Fr::from(122),
            Fr::from(231),
            Fr::from(1202),
        ];
        // Run the trusted setup function
        let result = TrustedSetup::trusted_setup(setup_size, contributions);
        println!("Everything: {:?}", result);

        // Check that the result has the correct size
        assert_eq!(
            result.powers_of_tau.len(),
            setup_size,
            "Output vector should have the same size as setup_size"
        );

        // Ensure values are not just the initial generator (i.e., contributions applied)
        let g1 = G1::default();
        assert!(
            result.powers_of_tau.iter().any(|&x| x != g1),
            "At least one element should be different from the initial generator"
        );

        // Additional sanity checks (optional)
        for i in 0..setup_size {
            assert_ne!(
                result.powers_of_tau[i],
                G1::default(),
                "No element should be the identity (zero point)"
            );
        }
    }

    #[test]
    fn test_extract_coefficients() {
        let uni_poly = get_uni_poly();

        let res = extract_coefficients(&uni_poly);

        let expected_res = vec![Fr::from(12), Fr::from(6), Fr::from(2)];

        assert_eq!(res, expected_res);
    }

    #[test]
    fn test_get_taus() {

        let trusted_setup: TrustedSetup = get_trusted_setup::<Fr>();

        let taus = trusted_setup.get_tau_in_both_groups();

        println!("Taus {:?}", taus);
    }

    #[test]
    fn test_commit() {
        let uni_poly = get_uni_poly();
        let trusted_set_up = get_trusted_setup::<Fr>();

        let res = trusted_set_up.commit_to_polynomial(&uni_poly);

        println!("Commit: {:?}", res);
    }

    #[test]
    fn test_open_poly() {
        let uni_poly = get_uni_poly();
        let trusted_set_up = get_trusted_setup::<Fr>();

        let res = trusted_set_up.open_polynomial(uni_poly, Fr::from(4));

        println!("V and Proof: {:?}", res);
    }
}

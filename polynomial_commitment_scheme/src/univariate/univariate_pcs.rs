use ark_bn254::{Bn254, Fr, FrConfig, G1Projective as G1, G2Projective as G2};
use ark_ec::pairing::Pairing;
use ark_ec::{PrimeGroup, VariableBaseMSM};
use ark_ff::{Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::test_rng;
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::Mul;
// use ark_poly::univariate::DensePolynomial;


#[derive(Clone, PartialEq, Debug)]
pub struct UniProof<F: PrimeField> {
    v: F, //evaluation at a
    proof_of_v: G1,
}

impl<F: PrimeField> UniProof<F> {
    fn new(v: F, proof_of_v: G1) -> UniProof<F> {
        Self {
            v,
            proof_of_v,
        }
    }
}


#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TrustedSetup<F: PrimeField> {
    pub powers_of_tau: Vec<G1>,
    pub g2_tau: G2,
    _phantom: PhantomData<F>,
}

impl<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>> TrustedSetup<F> {
    pub(crate) fn new(powers_of_tau: Vec<G1>, g2_tau: G2) -> Self {
        Self {
            powers_of_tau,
            g2_tau,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn initiate_univariate_trusted_setup(setup_size: usize, init_tau: F) -> TrustedSetup<F> {
        let g1 = G1::generator(); // Generator in G1
        let mut g2_tau = G2::generator(); // Generator in G2

        let mut trusted_setup = vec![g1; setup_size];

        // for each_cont in init_tau {
        let mut cont_power = F::one(); // replacing pow with `each_cont^0 = 1`

        for i in 1..setup_size {
            cont_power *= init_tau; //  this satisfies `each_cont^i` except 'each_cont^0'
            trusted_setup[i] = trusted_setup[i].mul(cont_power);
            println!("{:?}", trusted_setup);

            if i == 1 {
                g2_tau = g2_tau.mul(cont_power);
            }
        }
        // }

        TrustedSetup::new(trusted_setup, g2_tau)
    }

    pub(crate) fn contribute_to_setup(&mut self, tau: F) -> Self {
        let powers_of_tau = self.powers_of_tau.clone();
        let mut cont_power = F::one();

        for i in 1..powers_of_tau.len() {
            cont_power *= tau;

            self.powers_of_tau[i] = powers_of_tau[i].mul(cont_power);

            if i == 1 {
                self.g2_tau = self.g2_tau.mul(cont_power);
            }
        }

        self.clone()
    }

    // with this, all the contributions are made ready fom the beginning and the setup is done
    fn univariate_trusted_setup(setup_size: usize, contributions: &[F]) -> TrustedSetup<F> {
        let g1 = G1::generator(); // Generator in G1
        let mut g2_tau = G2::generator(); // Generator in G2

        let mut trusted_setup = vec![g1; setup_size];

        for each_cont in contributions {
            let mut cont_power = F::one(); // replacing pow with `each_cont^0 = 1`

            for i in 1..setup_size {
                cont_power *= each_cont; //  this satisfies `each_cont^i` except 'each_cont^0'
                trusted_setup[i] = trusted_setup[i].mul(cont_power);
                println!("{:?}", trusted_setup);
                if i == 1 {
                    g2_tau = g2_tau.mul(cont_power);
                }
            }
        }

        TrustedSetup::new(trusted_setup, g2_tau)
    }

    pub fn get_tau_in_both_groups(&self) -> (G1, G2) {
        (self.powers_of_tau[1], self.g2_tau)
    }

    pub fn commit_to_polynomial(&self, uni_poly: &UnivariatePoly<F>) -> G1 {
        let poly_degree = uni_poly.degree();

        if poly_degree > F::from(self.powers_of_tau.len() as u32) {
            panic!("Insufficient Powers of Tau {}", poly_degree);
        }

        let commitment = self.compute(uni_poly);

        commitment
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

    pub fn open_polynomial(&self, uni_poly: UnivariatePoly<F>, eval_at: F) -> UniProof<F> {
        let v = uni_poly.full_coeff_evaluation(eval_at);

        let mut numerator = uni_poly.clone();

        let mut found = false;

        // f(t) - v e.g v = 3; poly = 2x^2 + 4 therefore 2x^2 + 4 - 3 === 2x^2 + 1
        for mut term in &mut numerator.co_ex {
            if term.exp == F::zero() {
                term.coeff = term.coeff - v;
                found = true;
                break;
            }
        }

        if !found {
            numerator.co_ex.push(Term::new(-v, F::zero()));
        }

        // this gives (x - a) vibe as the root of the polynomial
        let divisor = UnivariatePoly::new(vec![
            Term::new(F::one(), F::one()),
            Term::new(eval_at.neg(), F::zero()),
        ]);

        let q_x = numerator.divide_polynomials(divisor); // to return quotient

        // q_x is a univariate poly in the clear, it needs to be combined with G1
        let q_t = self.compute(&q_x.0); // g^coeff; g = power of tau

        UniProof::new(v, q_t) // (f(a), proof)
    }

    pub fn verifier_verifies(&self, commitment: G1, a: F, proof: &UniProof<F>) -> bool {
        let g1 = G1::generator();
        let g2 = G2::generator();

        let ft_v = commitment + g1.mul(proof.v.neg()); // f(tau)  - v
        let tau_a = self.g2_tau + g2.mul(a.neg()); // (tau - a)

        //using bilinear pairing G1 x G2 = GT
        let lhs = Bn254::pairing(ft_v, g2); // ((f(tau) - v), g^2)
        let rhs = Bn254::pairing(proof.proof_of_v, tau_a); // (q_tau, (tau - a))

        lhs == rhs
    }

    fn compute(&self, uni_poly: &UnivariatePoly<F>) -> G1 {
        let mut result = G1::default();

        // ∑(coeff[i] * powers_of_tau[i]) no need to convert to dense
        for term in &uni_poly.co_ex {
            let exp = term.exp.into_bigint().as_ref()[0] as usize; // convert exponent to index
            let term_result = self.powers_of_tau[exp].mul(term.coeff); //g^coeff
            result += term_result;
        }
        result
    }
}

fn extract_coefficients<F: PrimeField>(uni_poly: &UnivariatePoly<F>) -> Vec<F> {
    let mut coefficients = vec![F::zero(); uni_poly.co_ex.len()];

    for term in &uni_poly.co_ex {
        coefficients[term.exp.into_bigint().as_ref()[0] as usize] = term.coeff;
    }
    println!("Coefficients: {:?}", coefficients);
    coefficients
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::univariate::uni_poly::Term;

    pub fn get_uni_poly() -> UnivariatePoly<Fr> {
        UnivariatePoly::new(vec![
            Term::new(Fr::from(2), Fr::from(2)),
            Term::new(Fr::from(6), Fr::from(1)),
            Term::new(Fr::from(12), Fr::from(0)),
        ])
    }

    pub fn get_trusted_setup<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    ) -> TrustedSetup<F> {
        let tau = F::from(5);
        TrustedSetup::initiate_univariate_trusted_setup(3, tau)
    }

    #[test]
    fn test_setup() {
        let result = TrustedSetup::<Fr>::random_setup(2);
        println!("{:?}", result);
    }

    #[test]
    fn test_trusted_setup() {
        let setup_size = 12; // number of elements in the setup

        let contributions = &[
            Fr::from(5),
            Fr::from(62),
            Fr::from(123),
            Fr::from(952),
            Fr::from(336),
            Fr::from(122),
            Fr::from(231),
            Fr::from(1202),
        ];
        // Run the trusted setup function
        let result = TrustedSetup::univariate_trusted_setup(setup_size, contributions);
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
    fn test_initiate_trusted_setup() {
        let setup_size = 3; // number of elements in the setup

        let tau = Fr::from(5);
        // Run the trusted setup function
        let result = TrustedSetup::initiate_univariate_trusted_setup(setup_size, tau);
        println!("Trusted Setup: {:?}", result);

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
    fn test_contribute_to_trusted_setup() {
        let contributed_tau = Fr::from(1029);

        let mut trusted_setup = get_trusted_setup();

        let res = trusted_setup.contribute_to_setup(contributed_tau);

        println!("Contributed: {:?}", res);
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
        let trusted_setup = get_trusted_setup::<Fr>();

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

    #[test]
    fn test_verify() {
        let uni_poly = get_uni_poly();
        let mut trusted_setup = get_trusted_setup::<Fr>();

        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(820));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(83420));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(5650));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(2343));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(353));

        let commit = trusted_setup.commit_to_polynomial(&uni_poly);

        let a = Fr::from(4);

        let proof = trusted_setup.open_polynomial(uni_poly, a);

        let verify = trusted_setup.verifier_verifies(commit, a, &proof);

        assert_eq!(verify, true);

        println!("Verify: {:?}", verify);
    }
}

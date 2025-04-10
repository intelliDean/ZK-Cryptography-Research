use ark_bn254::{Bn254, Fr, FrConfig, G1Projective as G1, G1Projective, G2Projective as G2, G1Affine};
use ark_ec::pairing::Pairing;
use ark_ec::{AffineRepr, PrimeGroup, VariableBaseMSM};
use ark_ff::{BigInteger, Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::test_rng;
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::Mul;
use sumcheck_protocol::transcript::Transcript;
use sha3::{Digest, Keccak256};

#[derive(Clone, PartialEq, Debug)]
pub struct NI_UniProof<F: PrimeField> {
    v: F, //evaluation at a
    proof_of_v: G1,
    commitment: G1Projective,
}

impl<F: PrimeField> NI_UniProof<F> {
    fn new(v: F, proof_of_v: G1, commitment: G1Projective) -> NI_UniProof<F> {
        Self { v, proof_of_v, commitment }
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

    fn initiate_univariate_trusted_setup(setup_size: usize, init_tau: F) -> TrustedSetup<F> {
        let g1 = G1::generator(); // Generator in G1
        let mut g2_tau = G2::generator(); // Generator in G2

        let mut trusted_setup = vec![g1; setup_size];

        let mut cont_power = F::one(); // replacing pow with `each_cont^0 = 1`

        for i in 1..setup_size {
            cont_power *= init_tau; //  this satisfies `each_cont^i` except 'each_cont^0'
            trusted_setup[i] = trusted_setup[i].mul(cont_power);
            println!("{:?}", trusted_setup);

            if i == 1 {
                g2_tau = g2_tau.mul(cont_power);
            }
        }

        TrustedSetup::new(trusted_setup, g2_tau)
    }

    fn contribute_to_setup(&mut self, tau: F) -> Self {
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
}

//============================ PROVER =============================================================
fn prover_side_of_the_protocol<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    trusted_setup: &TrustedSetup<F>,
    uni_poly: &UnivariatePoly<F>,
) -> NI_UniProof<F> {

    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());

    let poly_degree = uni_poly.degree();

    if poly_degree > F::from(trusted_setup.powers_of_tau.len() as u32) {
        panic!("Insufficient Powers of Tau {}", poly_degree);
    }

    let commitment: G1Projective = compute(trusted_setup, uni_poly);

    transcript.absorb(&g1_to_bytes::<F>(&commitment));
    let a = transcript.squeeze();

    open_polynomial(trusted_setup, uni_poly.clone(), a, commitment)
}

pub fn open_polynomial<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>> (
    trusted_setup: &TrustedSetup<F>,
    uni_poly: UnivariatePoly<F>,
    eval_at: F,
    commitment: G1Projective
) -> NI_UniProof<F> {
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
    let q_t = compute(trusted_setup, &q_x.0); // g^coeff; g = power of tau

    NI_UniProof::new(v, q_t, commitment) // (f(a), proof)
}

fn compute<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    trusted_setup: &TrustedSetup<F>, uni_poly: &UnivariatePoly<F>
) -> G1 {
    let mut result = G1::default();

    // ∑(coeff[i] * powers_of_tau[i]) no need to convert to dense
    for term in &uni_poly.co_ex {
        let exp = term.exp.into_bigint().as_ref()[0] as usize; // convert exponent to index
        let term_result = trusted_setup.powers_of_tau[exp].mul(term.coeff); //g^coeff
        result += term_result;
    }
    result
}

pub(crate) fn g1_to_bytes<F: PrimeField>(point: &G1Projective) -> Vec<u8> {
    let mut bytes = Vec::new();
    let byte_len = F::BigInt::NUM_LIMBS * 8;

    let affine: G1Affine = (*point).into();
    if affine.is_zero() {
        return vec![0u8; byte_len * 2];
    }

    let x_bytes = affine.x.into_bigint().to_bytes_be();
    bytes.extend_from_slice(&x_bytes);
    if x_bytes.len() < byte_len {
        bytes.extend(vec![0; byte_len - x_bytes.len()]);
    }

    let y_bytes = affine.y.into_bigint().to_bytes_be();
    bytes.extend_from_slice(&y_bytes);
    if y_bytes.len() < byte_len {
        bytes.extend(vec![0; byte_len - y_bytes.len()]);
    }

    bytes
}
//========================= VERIFIER =============================================================

pub fn verifier_verifies<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(trusted_setup: &TrustedSetup<F>, proof: &NI_UniProof<F>) -> bool {

    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());

    transcript.absorb(&g1_to_bytes::<F>(&proof.commitment));
    let a = transcript.squeeze();


    let g1 = G1::generator();
    let g2 = G2::generator();

    let ft_v = proof.commitment + g1.mul(proof.v.neg()); // f(tau)  - v
    let tau_a = trusted_setup.g2_tau + g2.mul(a.neg()); // (tau - a)

    //using bilinear pairing G1 x G2 = GT
    let lhs = Bn254::pairing(ft_v, g2); // ((f(tau) - v), g^2)
    let rhs = Bn254::pairing(proof.proof_of_v, tau_a); // (q_tau, (tau - a))

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

    fn get_trusted_setup<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    ) -> TrustedSetup<F> {
        let tau = F::from(5);
        TrustedSetup::initiate_univariate_trusted_setup(3, tau)
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
    fn test_non_interactive_univariate_pcs() {
        let uni_poly = get_uni_poly();
        let mut trusted_setup = get_trusted_setup::<Fr>();

        let contributed = trusted_setup.contribute_to_setup(Fr::from(820));
        let contributed = trusted_setup.contribute_to_setup(Fr::from(83420));
        let contributed = trusted_setup.contribute_to_setup(Fr::from(5650));
        let contributed = trusted_setup.contribute_to_setup(Fr::from(2343));
        let contributed = trusted_setup.contribute_to_setup(Fr::from(353));


        let proof = prover_side_of_the_protocol(&contributed, &uni_poly);

        let verify = verifier_verifies(&contributed, &proof);

        assert_eq!(verify, true);

        println!("Verified: {:?}", verify);
    }
}

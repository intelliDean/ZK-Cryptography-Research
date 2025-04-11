use crate::univariate::univariate_pcs::TrustedSetup;
use ark_bn254::{
    Bn254, Fr, FrConfig, G1Affine, G1Projective as G1, G1Projective, G2Projective as G2,
};
use ark_ec::pairing::Pairing;
use ark_ec::{AffineRepr, PrimeGroup, VariableBaseMSM};
use ark_ff::{BigInteger, Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::test_rng;
use polynomials::univariate::uni_poly::{Term, UnivariatePoly};
use sha3::{Digest, Keccak256};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::Mul;
use sumcheck_protocol::transcript::Transcript;

#[derive(Clone, PartialEq, Debug)]
pub struct NI_UniProof<F: PrimeField> {
    v: F,                     //evaluation at a
    proof_of_v: G1,           // evaluation at a
    commitment: G1Projective, // evaluation at the powers of tau
}

impl<F: PrimeField> NI_UniProof<F> {
    fn new(v: F, proof_of_v: G1, commitment: G1Projective) -> NI_UniProof<F> {
        Self {
            v,
            proof_of_v,
            commitment,
        }
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

pub fn open_polynomial<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    trusted_setup: &TrustedSetup<F>,
    uni_poly: UnivariatePoly<F>,
    eval_at: F,
    commitment: G1Projective,
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
    trusted_setup: &TrustedSetup<F>,
    uni_poly: &UnivariatePoly<F>,
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

// to add G1 to the transcript, i need to convert it to a bytes
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

pub fn verifier_verifies<F: PrimeField + Borrow<Fp<MontBackend<FrConfig, 4>, 4>>>(
    trusted_setup: &TrustedSetup<F>,
    proof: &NI_UniProof<F>,
) -> bool {
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
    fn test_non_interactive_univariate_pcs() {
        let uni_poly = get_uni_poly();
        let mut trusted_setup = get_trusted_setup();

        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(820));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(83420));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(5650));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(2343));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(353));

        let proof = prover_side_of_the_protocol(&trusted_setup, &uni_poly);

        let verify = verifier_verifies(&trusted_setup, &proof);

        assert_eq!(verify, true);

        println!("Verified: {:?}", verify);
    }

    #[test]
    fn test_non_interactive_univariate_pcs_to_fail() {
        let uni_poly = get_uni_poly();
        let mut trusted_setup = get_trusted_setup();

        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(820));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(83420));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(5650));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(2343));
        trusted_setup = trusted_setup.contribute_to_setup(Fr::from(353));

        let proof = prover_side_of_the_protocol(&trusted_setup, &uni_poly);

        let false_proof = NI_UniProof::new(Fr::from(120), proof.proof_of_v, proof.commitment);

        let verify = verifier_verifies(&trusted_setup, &false_proof);

        assert_eq!(verify, false);

        println!("Verified: {:?}", verify);
    }
}

use crate::prover_sumcheck::{convert_to_bytes, Proof};
use crate::transcript::Transcript;
use ark_ff::{BigInteger, PrimeField};
use ark_std::iterable::Iterable;
use sha3::{Digest, Keccak256};
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

pub(crate) fn verifier_verifies_provers_claim<F: PrimeField>(
    proof: &Proof<F>,
    initial_poly: MultilinearPoly<F>,
) -> bool {
    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());

    //the verifier and the prover MUST add same things to the Transcript if they want to get the same result
    transcript.absorb(&*convert_to_bytes(&initial_poly.polynomial));
    transcript.absorb(proof.claimed_sum.into_bigint().to_bytes_be().as_slice());

    let mut claimed_sum = proof.claimed_sum;
    let mut challenges = Vec::with_capacity(proof.uni_polys.len());

    for uni_poly in &proof.uni_polys {
        // Evaluate the univariate polynomial over the boolean hypercube and compare it with the claimed sum
        if claimed_sum != uni_poly[0] + uni_poly[1] {
            return false;
        }
        transcript.absorb(&*convert_to_bytes(uni_poly));

        // Sample a challenge
        let challenge = transcript.squeeze();
        challenges.push(challenge);

        // Evaluate the univariate polynomial at the challenge
        claimed_sum = uni_poly[0] + challenge * (uni_poly[1] - uni_poly[0]);
    }

    if claimed_sum != initial_poly.full_evaluation(challenges) {// evaluation(.polynomial, challenges) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prover_sumcheck::prover_proves_claim;
    use ark_bn254::Fr;
    use polynomials::multilinear::multilinear::to_field;

    #[test]
    fn test_verifier_verify_proof() {
        let p = to_field(vec![0, 0, 0, 3, 0, 0, 2, 5]);
        let poly: MultilinearPoly<Fr> = MultilinearPoly::new(p.clone());

        let proof = prover_proves_claim(poly.clone());

        let v_poly = MultilinearPoly {
            polynomial: p,
        };
        let verify = verifier_verifies_provers_claim(&proof, v_poly.clone());

        assert_eq!(verify, true);
    }
}

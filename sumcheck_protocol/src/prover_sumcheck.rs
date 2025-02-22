use crate::transcript::*;
use ark_ff::{BigInteger, PrimeField};
use ark_std::iterable::Iterable;
use ark_std::vec::Vec;
use sha3::{Digest, Keccak256};
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Debug, PartialEq)]
pub struct Proof<F: PrimeField> {
    pub uni_polys: Vec<Vec<F>>,
    pub claimed_sum: F,
}

pub(crate) fn prover_proves_claim<F: PrimeField>(init_poly: MultilinearPoly<F>) -> Proof<F> {
    let mut transcript = Transcript::<Keccak256, F>::init(Keccak256::new());
    let seq = init_poly.polynomial.len();
    let iterations = seq.trailing_zeros() as usize; // Convert to usize once
    let mut uni_polys = Vec::with_capacity(iterations);

    // Compute the initial claimed sum more efficiently
    let (first_half, second_half) = init_poly.polynomial.split_at(seq / 2);
    let claimed_sum = first_half.iter().chain(second_half.iter()).sum::<F>();

    // Add initial data to the transcript
    transcript.absorb(&*convert_to_bytes(&init_poly.polynomial));
    transcript.absorb(claimed_sum.into_bigint().to_bytes_be().as_slice());

    let mut poly = init_poly.clone();

    for _ in 0..iterations {
        let half_len = poly.polynomial.len() / 2;
        let (first, last) = poly.polynomial.split_at(half_len);

        // Compute and store univariate polynomial
        let univariate_poly = vec![first.iter().sum(), last.iter().sum()];
        uni_polys.push(univariate_poly.clone());

        // Absorb into transcript and generate challenge
        transcript.absorb(&*convert_to_bytes(&univariate_poly));
        let challenge = transcript.squeeze();

        // Perform partial evaluation in-place
        poly = poly.partial_evaluation(0, challenge);
    }

    Proof {
        uni_polys,
        claimed_sum,
    }
}

pub(crate) fn convert_to_bytes<F: PrimeField>(field_elements: &Vec<F>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let byte_len = F::BigInt::NUM_LIMBS * 8; // Each limb is 8 bytes

    for element in field_elements {
        let element_bytes = element.into_bigint().to_bytes_be();
        // Pad the bytes to ensure consistent length
        bytes.extend_from_slice(&element_bytes);
        if element_bytes.len() < byte_len {
            bytes.extend(vec![0; byte_len - element_bytes.len()]);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::multilinear::multilinear::to_field;

    #[test]
    fn test_prover_proves_claim() {
        let poly: MultilinearPoly<Fr> =
            MultilinearPoly::new(to_field(vec![0, 0, 0, 3, 0, 0, 2, 5]));

        let proof = prover_proves_claim(poly.clone());
        assert_eq!(proof.claimed_sum, Fr::from(10));
        assert_eq!(proof.uni_polys.len(), 3);
    }
}

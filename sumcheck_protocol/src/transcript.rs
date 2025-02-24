use ark_ff::{BigInteger, PrimeField};
use ark_std::iterable::Iterable;
use sha3::{Digest, Keccak256};
use std::marker::PhantomData;
use std::mem::take;

pub struct Transcript<K: HashTrait, F: PrimeField> {
    _field: PhantomData<fn() -> F>, // More idiomatic PhantomData usage
    hash_function: K,
}

impl<K: HashTrait, F: PrimeField> Transcript<K, F> {
    pub fn init(hash_function: K) -> Self {
        Self {
            _field: PhantomData,
            hash_function,
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        self.hash_function.append(data);
    }

    pub fn squeeze(&mut self) -> F {
        let hash_output = self.hash_function.generate_hash();
        F::from_be_bytes_mod_order(&hash_output)
    }
}

pub trait HashTrait {
    fn append(&mut self, data: &[u8]);
    fn generate_hash(&mut self) -> Vec<u8>;
}

impl HashTrait for Keccak256 {
    fn append(&mut self, data: &[u8]) {
        self.update(data);
    }

    fn generate_hash(&mut self) -> Vec<u8> {
        take(self).finalize().to_vec() // Avoids cloning
    }
}

#[cfg(test)]
mod test {
    use super::Keccak256;
    use super::Transcript;
    use ark_bn254::Fq;
    use ark_ff::{BigInteger, PrimeField};
    use sha3::Digest;

    #[test]
    fn test_hash() {
        let mut transcript = Transcript::<Keccak256, Fq>::init(Keccak256::new());

        transcript.absorb(Fq::from(7).into_bigint().to_bytes_be().as_slice());
        transcript.absorb("girl".as_bytes());

        let challenge = transcript.squeeze();
        let challenge1 = transcript.squeeze();

        dbg!(challenge);
        dbg!(challenge1);
    }
}

use ark_bn254::{Bn254, Fr, FrConfig, G1Projective as G1, G2Projective as G2};
use ark_ec::pairing::{Pairing, PairingOutput};
use ark_ec::{PrimeGroup, VariableBaseMSM};
use ark_ff::{Field, Fp, MontBackend, PrimeField, UniformRand};
use ark_poly::Polynomial;
use ark_std::iterable::Iterable;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{Mul, Sub};
// use ark_ec::{G1Affine, G2Affine}; // Adjust based on your library
use ark_std::One;


#[derive(Clone, PartialEq, Debug)]
pub struct MultiProof<F: PrimeField> {
    pub v: F,               //full evaluation at all vars
    pub quotients: Vec<G1>, //all encrypted quotients
}

impl<F: PrimeField> MultiProof<F> {
    fn new(v: F, quotients: Vec<G1>) -> MultiProof<F> {
        Self { v, quotients }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct KZG<F: PrimeField> {
    pub powers_of_tau: Vec<G1>,
    pub g2_tau: Vec<G2>,
    _phantom: PhantomData<F>,
}
impl<F: PrimeField> KZG<F> {
    fn new(powers_of_tau: Vec<G1>, g2_tau: Vec<G2>) -> Self {
        Self {
            powers_of_tau,
            g2_tau,
            _phantom: PhantomData,
        }
    }

    /*For Multilinear, I was thinking that instead of making one contributor to have contributed all
        the variables ot tau, each member should contribute one tau each. When all members are done
        contributing, then they can go ahead to use all the taus to create the setup
    */
    pub fn multilinear_trusted_setup(tau_var: &[F]) -> Self  {

        println!("Tau len that comes in: {:?}", tau_var.len());

        let g1 = G1::generator();
        let g2 = G2::generator();

        let eval_points = 1 << tau_var.len(); //2^num_of_vars e.g 2^3 for 3 vars

        let bhc = generate_hypercube(eval_points);

        let mut g1_evals = Vec::with_capacity(bhc.len());

        let mut g2_evals: Vec<G2> = Vec::with_capacity(tau_var.len());

        for (i, combinations) in bhc.iter().enumerate() {
            let mut res = F::one();

            for (j, &bit) in combinations.iter().enumerate() {
                res *= if bit {
                    //true or false means 1 or 0
                    tau_var[j]
                } else {
                    F::one().sub(tau_var[j])
                };
            }

            //encrypting all taus in G2
            if i < tau_var.len() {
                g2_evals.push(g2.mul_bigint(tau_var[i].into_bigint()));
            }

            g1_evals.push(g1.mul_bigint(res.into_bigint()));
        }

        KZG::new(g1_evals, g2_evals)
    }

    pub fn commit_to_polynomial (&self, multilinear: &MultilinearPoly<F>) -> G1 {
        assert!(self.powers_of_tau.len().is_power_of_two());
        assert_eq!(
            multilinear.polynomial.len(),
            self.powers_of_tau.len(),
            "Inconsistent variable size"
        );

        self.evaluate_with_lagrange_basis(&multilinear.polynomial)
    }

    fn evaluate_with_lagrange_basis(&self, multilinear: &[F]) -> G1 {
        assert_eq!(
            self.powers_of_tau.len(),
            multilinear.len(),
            "Inconsistent variable size"
        );

        let mut res = G1::default();
        // zip them together
        for (tau, eval) in self.powers_of_tau.iter().zip(multilinear.iter()) {
            res += tau.mul_bigint(eval.into_bigint());
        }
        res
    }

    pub fn open_polynomial(&self, multilinear: &MultilinearPoly<F>, open_at: &[F]) -> MultiProof<F> {
        //full evaluation of the polynomial to get V
        let v = multilinear.clone().full_evaluation(open_at.to_vec());

        //to get the polynomial to perform the proof, we do f(x) - V
        let mut new_polynomial = MultilinearPoly::new(
            //subtract each element in the boolean hypercube by v
            multilinear.polynomial.iter().map(|p| *p - v).collect()
        );

        let mut quotients = Vec::with_capacity(open_at.len());

        for  var in open_at {

            let f_1 = new_polynomial.clone().partial_evaluation(0, F::one());
            let f_0 = new_polynomial.clone().partial_evaluation(0, F::zero());

            // to get quotient, we do f(1) - f(0)
            let quo = get_quotient(&f_1.polynomial, &f_0.polynomial);

            println!("quotients: {:?}, open vars: {:?}", quo.polynomial.len(), open_at.len());
            //we blow up by adding 0 e.g., 0a + 3bc
            let blown = blow_up_poly_with_zero(&quo, open_at.len());

            quotients.push(self.evaluate_with_lagrange_basis(&blown.polynomial));

            new_polynomial = new_polynomial.clone().partial_evaluation(0, *var);
        }

        MultiProof::new(v, quotients)
    }

    pub fn verifier_verifies(&self, commitment: G1, all_a: &[F], proof: &MultiProof<F>) -> bool {
        println!("powers of tau: {:?}", self.powers_of_tau.len());
        println!("all a: {:?}", all_a.len());

        let g1 = G1::generator();
        let g2 = G2::generator();

        let ft_v = commitment + g1.mul_bigint(proof.v.neg().into_bigint()); // f(tau)  - v

        let mut rhs = PairingOutput::default();

        for (i, a) in all_a.iter().enumerate() {
            let tau_a = self.g2_tau[i] + g2.mul_bigint(a.neg().into_bigint()); // (tau - a)

           let res = Bn254::pairing(proof.quotients[i], tau_a);

           rhs += res;
        }

       //  //using bilinear pairing G1 x G2 = GT
        let lhs = Bn254::pairing(ft_v, g2); // ((f(tau) - v), g^1)

        lhs == rhs
    }
}

fn get_quotient<F: PrimeField>(f_1: &Vec<F>, f_0: &Vec<F>) -> MultilinearPoly<F> {
    assert_eq!(f_1.len(), f_0.len());

    let mut poly = Vec::with_capacity(f_1.len());

    //f(1) - f(0)
    //element-wise subtraction
    for (p1, p0) in f_1.iter().zip(f_0.iter()) {
        poly.push(p1.sub(p0));
    }

    MultilinearPoly::new(poly)
}

fn blow_up_poly_with_zero<F: PrimeField>(
    poly: &MultilinearPoly<F>,
    var_size: usize,
) -> MultilinearPoly<F> {
    let orig_len = poly.polynomial.len();
    let target_size: usize = 1 << var_size;

    assert!(
        target_size.is_power_of_two() && orig_len.is_power_of_two(),
        "polynomial must be a power of 2"
    );

    println!("target_size: {}, original len: {}", target_size, orig_len);

    assert!(
        target_size > orig_len,
        "Blown size must be > the original size"
    );

    /*if the poly is [2,6], which is 1 variable and I need to blow it up to 3 variables which is
    8 eval points, this will blow it up like this [2, 6, 2, 6, 2, 6, 2, 6] by adding 0 like 0a, 0b*/
    let new_poly: Vec<F> = poly
        .polynomial
        .iter()
        .cycle()
        .take(target_size)
        .cloned()
        .collect();

    MultilinearPoly::new(new_poly)
}

fn generate_hypercube(n: usize) -> Vec<Vec<bool>> {
    assert!(
        n > 1 && n.is_power_of_two(),
        "Required power of 2 but got {}",
        n
    );

    let bits = n.ilog2() as usize; //log₂(n)
    let total_combinations = 1 << bits;

    (0..total_combinations) //returns a bool for the boolean hypercube
        .map(|i| (0..bits).map(|b| (i & (1 << b)) != 0).rev().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_poly() -> MultilinearPoly<Fr> {

        MultilinearPoly::new(vec![
                Fr::from(3),
                Fr::from(2),
                Fr::from(2),
                Fr::from(2),
                Fr::from(4),
                Fr::from(5),
                Fr::from(8),
                Fr::from(9),
            ])
    }


    #[test]
    fn test_generate_hypercube() {
        let result = generate_hypercube(4);

        let expected = vec![
            vec![false, false], // 00
            vec![false, true],  //  01
            vec![true, false],  //  10
            vec![true, true],   //   11
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multilinear_setup() {
        let res = KZG::multilinear_trusted_setup(&[Fr::from(2), Fr::from(6), Fr::from(6)]);
        println!("Result: {:?}", res);

        assert_eq!(res.powers_of_tau.len(), 8);
    }

    #[test]
    fn test_commitment() {
        let trusted_setup =
            KZG::multilinear_trusted_setup(&[Fr::from(2), Fr::from(6), Fr::from(6)]);

        let poly = get_poly();

        let res = trusted_setup.commit_to_polynomial(&poly);

        println!("Result: {:?}", res);
    }

    #[test]
    fn test_open_commitment() {
        let trusted_setup =
            KZG::multilinear_trusted_setup(&[Fr::from(5), Fr::from(2), Fr::from(3)]);

        let poly = get_poly();

        let res = trusted_setup.open_polynomial(&poly, &[Fr::from(6), Fr::from(4), Fr::from(0)]);
        assert_eq!(res.quotients.len(), 3);

        println!("Result: {:?}", res);
    }

    #[test]
    fn test_blow_up() {
        let poly: MultilinearPoly<Fr> = MultilinearPoly::new(vec![Fr::from(4), Fr::from(18)]);

        let res = blow_up_poly_with_zero(&poly, 3);

        assert_eq!(
            res,
            MultilinearPoly::new(vec![
                Fr::from(4),
                Fr::from(18),
                Fr::from(4),
                Fr::from(18),
                Fr::from(4),
                Fr::from(18),
                Fr::from(4),
                Fr::from(18)
            ])
        );

        println!("Result: {:?}", res);
    }

    #[test]
    fn test_protocol() {

        //Trusted Setup ceremony
        let trusted_setup = KZG::multilinear_trusted_setup(
            &[Fr::from(5), Fr::from(2), Fr::from(3)]
        );

        let poly = MultilinearPoly::new(vec![
            Fr::from(3),
            Fr::from(2),
            Fr::from(2),
            Fr::from(2),
            Fr::from(4),
            Fr::from(5),
            Fr::from(8),
            Fr::from(9),
        ]);

        //Prover commits to the polynomial and sends the commitment to the verifier
        let commitment = trusted_setup.commit_to_polynomial(&poly);

        //The verifier having received the commitment, sends 'a' to the prover to open the poly at 'a'
        let a = &[Fr::from(6), Fr::from(4), Fr::from(0)];

        //the prover opens the polynomial at 'a' and sends the proof (v, quotients) to the verifier
        let proof = trusted_setup.open_polynomial(&poly, a);

        //the verifier verifies the proof sent by the prover
        let verify = trusted_setup.verifier_verifies(commitment, a, &proof);


        //End of protocol
        assert_eq!(verify, true);

        println!("Verify: {:?}", verify);
    }

    #[test]
    fn test_verify_to_fail() {

        let trusted_setup = KZG::multilinear_trusted_setup(
            &[Fr::from(5), Fr::from(2), Fr::from(3)]
        );

        let poly = MultilinearPoly::new(vec![
            Fr::from(3),
            Fr::from(2),
            Fr::from(2),
            Fr::from(2),
            Fr::from(4),
            Fr::from(5),
            Fr::from(8),
            Fr::from(9),
        ]);

        //Prover commits to the polynomial and sends the commitment to the verifier
        let commitment = trusted_setup.commit_to_polynomial(&poly);

     //The verifier having received the commitment, sends 'a' to the prover to open the poly at 'a'
        let a = &[Fr::from(6), Fr::from(4), Fr::from(0)];

        //the prover opens the polynomial at 'a' and sends the proof (v, quotients) to the verifier
        let proof = trusted_setup.open_polynomial(&poly, a);

        let false_proof = MultiProof::new(Fr::from(120), proof.quotients.clone());

        //the verifier verifies the proof sent by the prover
        let verify = trusted_setup.verifier_verifies(commitment, a, &false_proof);

        //End of protocol
        assert_eq!(verify, false);

        println!("Verify: {:?}", verify);
    }
}
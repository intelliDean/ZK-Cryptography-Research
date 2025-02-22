use ark_ff::PrimeField;
#[derive(Debug, Clone, PartialEq)]
pub struct CoeffMulti<F: PrimeField> {
    pub coeff_rep: Vec<(Vec<F>, F)>,
}

impl<F: PrimeField> CoeffMulti<F> {
    fn new(coeff_rep: Vec<(Vec<F>, F)>) -> Self {
        Self { coeff_rep }
    }

    pub fn multilinear_coeff_full_evaluation(self, evaluate_at: Vec<F>) -> F {
        let mut result: F = F::zero();

        for poly in self.coeff_rep {
            result += poly.1.clone() * multiply(&poly.0, &evaluate_at);
        }
        result
    }

    pub(crate) fn coeff_partial_evaluation(
        &mut self,
        eval_var_at: F,
        var_pos: u8,
    ) -> CoeffMulti<F> {
        if self.coeff_rep[0].0.len() - 1 >= var_pos as usize {
            let mut new_poly = Vec::new();

            for poly in self.coeff_rep.iter_mut() {
                if poly.0[var_pos as usize] != F::one() {
                    continue;
                }
                poly.1 *= eval_var_at;
                poly.0[var_pos as usize] = F::zero();
            }

            let res = check_complete(&self.coeff_rep);
            if res > F::zero() {
                return CoeffMulti::new(vec![(self.coeff_rep[0].0.clone(), res)]);
            }

            new_poly = self.coeff_rep.clone();
            CoeffMulti::new(new_poly)
        } else {
            CoeffMulti::new(vec![])
        }
    }
}

fn multiply<F: PrimeField>(p0: &Vec<F>, p1: &Vec<F>) -> F {
    p0.iter()
        .zip(p1.iter())
        .map(|(exponent, base)| base.pow(exponent.into_bigint().as_ref()))
        .product()
}

fn check_complete<F: PrimeField>(multilinear: &Vec<(Vec<F>, F)>) -> F {
    let mut result: F = F::zero();
    for poly_struct in multilinear {
        if poly_struct.0.iter().any(|&poly| poly == F::one()) {
            return F::zero();
        }
        result += poly_struct.1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Fr, FrConfig};
    use ark_ff::{Fp256, MontBackend};

    fn get_poly() -> CoeffMulti<Fp256<MontBackend<FrConfig, 4>>> {
        let representation = CoeffMulti::new(vec![
            (vec![Fr::from(1), Fr::from(1), Fr::from(0)], Fr::from(2)),
            (vec![Fr::from(0), Fr::from(1), Fr::from(1)], Fr::from(3)),
        ]);
        representation
    }

    #[test]
    fn test_partial_evaluation_2_vars() {
        let mut poly = CoeffMulti {
            coeff_rep: vec![
                (vec![Fr::from(1), Fr::from(0)], Fr::from(2)),
                (vec![Fr::from(0), Fr::from(1)], Fr::from(3)),
            ],
        }; //2a + 3b
        let result = poly.coeff_partial_evaluation(Fr::from(3), 1);

        let expected_result = vec![
            (vec![Fr::from(1), Fr::from(0)], Fr::from(2)),
            (vec![Fr::from(0), Fr::from(0)], Fr::from(9)),
        ]; //2a + 9;
        let expected_result = CoeffMulti::new(expected_result);
        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_partial_evaluation_3_vars() {
        let mut poly = CoeffMulti {
            coeff_rep: vec![
                (vec![Fr::from(1), Fr::from(1), Fr::from(0)], Fr::from(3)),
                (vec![Fr::from(0), Fr::from(1), Fr::from(0)], Fr::from(2)),
                (vec![Fr::from(0), Fr::from(1), Fr::from(1)], Fr::from(6)),
            ], // 3ab + 2b + 6bc,
        };

        let result =
            poly.coeff_partial_evaluation(Fr::from(3) /*b = 3*/, 1 /*index 1 = b*/);

        let expected_result = CoeffMulti::new(vec![
            (vec![Fr::from(1), Fr::from(0), Fr::from(0)], Fr::from(9)),
            (vec![Fr::from(0), Fr::from(0), Fr::from(0)], Fr::from(6)),
            (vec![Fr::from(0), Fr::from(0), Fr::from(1)], Fr::from(18)),
        ]); //9a + 6 + 18c

        assert_eq!(result, expected_result);
    }

    #[test]
    fn test_multilinear_coeff_full_evaluation() {
        let representation = get_poly(); //2ab + 3bc
        let evaluated_at = vec![Fr::from(1), Fr::from(2), Fr::from(3)]; //a = 1, b = 2, c = 3

        let result = representation.multilinear_coeff_full_evaluation(evaluated_at);

        assert_eq!(result, Fr::from(22));
    }

    #[test]
    fn even_if_the_var_numbers_exceeds_it_does_not_panic() {
        let representation = get_poly();
        let evaluated_at = vec![
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ]; //a = 1, b = 2, c = 3

        let result = representation.multilinear_coeff_full_evaluation(evaluated_at);

        assert_eq!(result, Fr::from(22));
    }
}

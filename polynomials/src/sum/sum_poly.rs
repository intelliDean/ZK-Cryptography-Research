use crate::product::product_poly::ProductPoly;
use ark_ff::PrimeField;
use crate::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Clone, Debug, PartialEq)]
pub struct SumPoly<F: PrimeField> {
    fbc_poly: Vec<ProductPoly<F>>, // most likely 2 ProductPoly at a time
}

impl<F: PrimeField> SumPoly<F> {
    pub fn new(product_polys: Vec<ProductPoly<F>>) -> Self {
        let var_size = product_polys[0].num_var();

        for product_poly in product_polys.iter().skip(1) {
            assert_eq!(
                product_poly.num_var(),
                var_size,
                "Inconsistent polynomial size"
            );
        }

        Self {
            fbc_poly: product_polys,
        }
    }

    pub fn degree(&self) -> usize {
        self.fbc_poly[0].degree()
    }

    pub fn num_vars(&self) -> u32 {
        self.fbc_poly[0].num_var()
    }

    pub fn full_evaluation(&self, values: Vec<F>) -> F {
        let mut result = F::zero();

        for product_poly in self.fbc_poly.iter() {
            result += product_poly.clone().full_evaluation(values.clone());
        }
        result
    }

    pub fn partial_evaluate(&self, var_pos: u32, eval_var_at: F) -> Self {
        let mut partially_evaluated_fbc_poly = Vec::new();

        for product_poly in self.fbc_poly.iter() {
            let partially_evaluated_product_poly =
                product_poly.partial_evaluate(var_pos, eval_var_at);

            partially_evaluated_fbc_poly.push(partially_evaluated_product_poly);
        }

        Self {
            fbc_poly: partially_evaluated_fbc_poly,
        }
    }

    pub fn sum_element_by_element(&self) -> MultilinearPoly<F> {
        assert!(self.fbc_poly.len() > 1, "Must be above 1");

        //This is a product poly that contains 2 Multilinear polys
        let first_multilinear = self.fbc_poly[0].multiply_element_by_element();

        let mut resultant_values = first_multilinear.polynomial.to_vec();

        for product_polynomial in self.fbc_poly.iter().skip(1) {
            //the relation between the product poly is multiplication
            let multilinear_poly: MultilinearPoly<F> = product_polynomial.multiply_element_by_element();

            // the relationship between the sum poly is addition
            for (i, value) in multilinear_poly.polynomial.iter().enumerate() {
                resultant_values[i] += value
            }
        }

        MultilinearPoly::new(resultant_values)
    }

    pub fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for product_polynomial in &self.fbc_poly {
            bytes.extend_from_slice(&product_polynomial.convert_to_bytes());
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fq;

    #[test]
    fn test_evaluate_sum_poly() {
        // First product polynomial
        let poly1a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly1b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly1b]);

        // Second product polynomial
        let poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(1)]);
        let poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let product_poly2 = ProductPoly::new(vec![poly2a, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        // a = 1, b = 2
        let values = vec![Fq::from(1), Fq::from(2)];

        assert_eq!(sum_polynomial.full_evaluation(values), Fq::from(32));
    }

    #[test]
    fn test_partial_evaluate_sum_poly() {

        let product_poly1 = ProductPoly::new(vec![
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]),
            MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)])
        ]);

        // Second product polynomial
        let poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(1)]);
        let poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let product_poly2 = ProductPoly::new(vec![poly2a, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);
        let evaluated_sum_poly = sum_polynomial.partial_evaluate(0, Fq::from(2));

        // Expected partial evaluations:
        let expect_poly1a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(4)]);
        let expect_poly1b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(6)]);
        let expect_product1 = ProductPoly::new(vec![expect_poly1a, expect_poly1b]);

        // For second product: poly2a(2) * poly2b = [0,2] * [0,2]
        let expect_poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(2)]);
        let expect_poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(4)]);
        let expect_product2 = ProductPoly::new(vec![expect_poly2a, expect_poly2b]);

        let expected_sum_poly = SumPoly::new(vec![expect_product1, expect_product2]);

        assert_eq!(evaluated_sum_poly.fbc_poly[0].product_poly, expected_sum_poly.fbc_poly[0].product_poly);
        assert_eq!(evaluated_sum_poly.fbc_poly[1].product_poly, expected_sum_poly.fbc_poly[1].product_poly);
    }

    #[test]
    fn test_add_polynomials_element_by_element() {
        // First product polynomial: (2x)(3y)
        let poly1a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly1b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly1b]);

        // Second product polynomial: (1x)(2y)
        let poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(1)]);
        let poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let product_poly2 = ProductPoly::new(vec![poly2a, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        let expected_sum = MultilinearPoly::new(vec![
            Fq::from(0),
            Fq::from(0),
            Fq::from(0),
            Fq::from(8)  // (2*3) + (1*2) = 6 + 2 = 8
        ]);

        assert_eq!(sum_polynomial.sum_element_by_element(), expected_sum);
    }

    #[test]
    fn test_degree_sum_poly() {
        let poly1a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly1b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly1b]);

        let poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(1)]);
        let poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let product_poly2 = ProductPoly::new(vec![poly2a, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        assert_eq!(sum_polynomial.degree(), 2);
    }

    #[test]
    fn test_number_of_variables() {
        let poly1a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let poly1b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(3)]);
        let product_poly1 = ProductPoly::new(vec![poly1a, poly1b]);

        let poly2a = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(1)]);
        let poly2b = MultilinearPoly::new(vec![Fq::from(0), Fq::from(0), Fq::from(0), Fq::from(2)]);
        let product_poly2 = ProductPoly::new(vec![poly2a, poly2b]);

        let sum_polynomial = SumPoly::new(vec![product_poly1, product_poly2]);

        assert_eq!(sum_polynomial.num_vars(), 2);
    }
}
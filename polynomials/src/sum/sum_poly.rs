use crate::product::product_poly::ProductPoly;
use ark_ff::PrimeField;
use crate::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Clone, Debug, PartialEq)]
pub struct SumPoly<F: PrimeField> {
    fbc_poly: Vec<ProductPoly<F>>,
}

impl<F: PrimeField> SumPoly<F> {
    pub fn new(product_polys: Vec<ProductPoly<F>>) -> Self {
        let var_size = product_polys[0].product_poly[0].polynomial.len();

        for product_poly in product_polys.iter() {
            for multilinear in product_poly.product_poly.iter() {
                assert_eq!(
                    multilinear.polynomial.len(),
                    var_size,
                    "Polynomial must have same var size"
                );
            }
        }

        Self {
            fbc_poly: product_polys,
        }
    }

    pub fn evaluate(&self, values: Vec<F>) -> F {
        let mut result = F::zero();

        for product_poly in self.fbc_poly.iter() {
            result += product_poly.clone().full_evaluation(values.clone());
        }
        result
    }

    pub fn partial_evaluate(&self, var_pos: u32, eval_var_at: F) -> Self {
        let mut partially_evaluated_fbc_poly = Vec::new();

        for product_polynomial in self.fbc_poly.iter() {
            let partially_evaluated_product_poly =
                product_polynomial.partial_evaluate(var_pos, eval_var_at);

            partially_evaluated_fbc_poly.push(partially_evaluated_product_poly);
        }

        Self {
            fbc_poly: partially_evaluated_fbc_poly,
        }
    }

    pub fn add_polynomials_element_wise(&self) -> MultilinearPoly<F> {
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

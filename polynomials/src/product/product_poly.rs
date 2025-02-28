use ark_ff::PrimeField;
use crate::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Clone, Debug, PartialEq)]
pub struct ProductPoly<F: PrimeField> {
   pub product_poly: Vec<MultilinearPoly<F>>,
}

impl<F: PrimeField> ProductPoly<F> {
    pub fn new(product_poly: Vec<MultilinearPoly<F>>) -> Self {
        let var_size = product_poly[0].polynomial.len();
        for poly in product_poly.iter() {
            assert_eq!(
                poly.polynomial.len(),
                var_size,
                "Polynomial must have same var size"
            );
        }

        Self {
            product_poly
        }
    }

    pub fn degree(&self) -> usize {
        self.product_poly.len() // the length of the product poly is the degree of the polynomial
    }

   pub fn num_var(&self) -> u32 {
        self.product_poly[0].num_var()
    }
    pub fn partial_evaluate(&self, var_pos: u32, eval_var_at: F) -> ProductPoly<F> {
        let mut new_product_poly = Vec::new();
        for poly in self.product_poly.iter() {
            new_product_poly.push(
                poly.clone().partial_evaluation(var_pos, eval_var_at)
            )
        }
        ProductPoly::new(new_product_poly)
    }

    // pub fn full_evaluation(mut self, eval_at: Vec<F>) -> F {
    //     let mut result = F::one();
    //     for poly in self.product_poly.iter() {
    //         result *= poly.clone().full_evaluation(eval_at.clone());
    //     }
    //     result
    // }

    pub fn full_evaluation(mut self, eval_at: Vec<F>) -> F {
        self.product_poly
            .iter()
            .map(|poly| poly.clone().full_evaluation(eval_at.clone()))
            .product()
    }


    //For GKR, I will be 2 polynomials but this function is made flexible to multiply any polynomial
    pub fn multiply_element_by_element(&self) -> MultilinearPoly<F> {
        assert!(self.product_poly.len() > 1, "Must be above 1");

        let mut base_poly = self.product_poly[0].polynomial.to_vec();

        for multilinear_poly in self.product_poly.iter().skip(1) {
            for (i, var) in multilinear_poly.polynomial.iter().enumerate() {
                base_poly[i] *= var
            }
        }
        MultilinearPoly::new(base_poly)
    }
    pub fn convert_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        for polynomial in &self.product_poly {
            bytes.extend_from_slice(&polynomial.convert_to_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_ff::{One, Zero};
    use crate::multilinear::multilinear::to_field;

    #[test]
    #[should_panic]
    fn test_new_to_panic() {
        let polys = vec![
            MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![0, 0, 4])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![1, 3, 5, 7])),
        ];

        let product_poly = ProductPoly::new(polys);
        println!("{:?}", product_poly);
    }
    #[test]
    fn test_new() {
        let polys = vec![
            MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![0, 0, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![1, 3, 5, 7])),
        ];

        let product_poly = ProductPoly::new(polys);
        println!("{:?}", product_poly);
        assert_eq!(product_poly.degree(), 3);
    }

    #[test]
    fn test_partial_evaluate() {
        let polys = vec![
            MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![0, 0, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![1, 3, 5, 7])),
        ];

        let product_poly = ProductPoly::new(polys);
        let new_poly = product_poly.partial_evaluate(0, Fr::zero());

        println!("{:?}", new_poly);
    }

    #[test]
    fn test_full_evaluation() {
        let polys = vec![
            // MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3, 5, 7])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![1, 2, 6, 8])),
        ];

        let product_poly = ProductPoly::new(polys);

        let eval_at = vec![Fr::zero(), Fr::one()];
        let result = product_poly.full_evaluation(eval_at);

        println!("{:?}", result);
        assert_eq!(result, Fr::from(6));
    }

    #[test]
    fn test_multiply_element_by_element() {
        let polys = vec![
            MultilinearPoly::new(to_field::<i32, Fr>(vec![2, 3])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![3, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![6, 2])),
        ];

        let product_poly = ProductPoly::new(polys);
        let result = product_poly.multiply_element_by_element();
        assert_eq!(result, MultilinearPoly::new(to_field::<i32, Fr>(vec![36, 30])));

    }
}

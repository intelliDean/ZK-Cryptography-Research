use ark_ff::PrimeField;
use crate::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Clone, Debug, PartialEq)]
pub struct ProductPoly<F: PrimeField> {
    product_poly: Vec<MultilinearPoly<F>>,
    degree: usize,
}

impl<F: PrimeField> ProductPoly<F> {
    pub fn new(polys: Vec<MultilinearPoly<F>>) -> Self {
        let var_size = polys[0].polynomial.len();
        for poly in polys.iter() {
            assert_eq!(
                poly.polynomial.len(),
                var_size,
                "Polynomial must have same var size"
            );
        }

        Self {
            product_poly: polys.clone(),
            degree: polys.len(),
        }
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

    pub fn full_evaluation(mut self, eval_at: Vec<F>) -> F {
        let mut result = F::zero();
        for poly in self.product_poly.iter() {
            result += poly.clone().full_evaluation(eval_at.clone());
        }
        result
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
        assert_eq!(product_poly.degree, 3);
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
            MultilinearPoly::new(to_field::<i32, Fr>(vec![0, 0, 4, 5])),
            MultilinearPoly::new(to_field::<i32, Fr>(vec![1, 3, 5, 7])),
        ];

        let product_poly = ProductPoly::new(polys);

        let eval_at = vec![Fr::zero(), Fr::one()];
        let new_poly = product_poly.full_evaluation(eval_at);

        println!("{:?}", new_poly);
    }
}

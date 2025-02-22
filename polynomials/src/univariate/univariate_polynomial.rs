
use ark_ff::PrimeField;
use crate::univariate::point::Points;

//SPARSE COEFFICIENT REPRESENTATION
#[derive(Debug, Clone, PartialEq)]
pub struct UnivariatePoly<F: PrimeField> {
    pub co_ex: Vec<(F, F)>, //tuple of coefficient and exponent, e.g., 6x^2
    pub degree: F,          //the term with the highest amount of exponent
}

impl<F: PrimeField> UnivariatePoly<F> {
    // this acts as the constructor
    pub fn new(co_exp: Vec<(F, F)>) -> UnivariatePoly<F> {
        let mut deg: F = F::zero();
        // sparse rep, this sets the degree of the polynomial
        deg = co_exp.iter().map(|&(_, exp)| exp).max().unwrap_or(deg);

        UnivariatePoly {
            co_ex: co_exp,
            degree: deg,
        }
    }
    pub fn degree(&self) -> F {
        self.degree
    }

    pub fn full_coeff_evaluation(&self, var_value: F) -> F {
        self.co_ex.iter().
            // result is initialized to zero
            fold(F::zero(), |result, &(coeff, exp)| {  //the reference of each co_ex is gotten
                let term = coeff * var_value.pow(exp.into_bigint().as_ref()); //coeff x var_value ^ exponent
                println!("result = {}", result + term); // Print intermediate results
                result + term
            })
    }

    pub fn generate_points(&self) -> Points<F> {
        let degree = self.degree.into_bigint().as_ref()[0];
        let points = (0..=degree) // degree + 1 = num of points needed
            .map(|i| {
                let x = F::from(i); // Convert i to field element
                let y = self.full_coeff_evaluation(x); // Evaluate polynomial at x
                (x, y) // Return (x, y) pair
            })
            .collect(); // Collect into Vec<(F, F)>

        Points { x_y: points }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Fr, FrConfig};
    use ark_ff::{Fp256, MontBackend};

    fn get_vec() -> UnivariatePoly<Fp256<MontBackend<FrConfig, 4>>> {
        UnivariatePoly::new(vec![
            (Fr::from(2), Fr::from(2)),
            (Fr::from(5), Fr::from(1)),
            (Fr::from(16), Fr::from(0)),
        ])
    }
    fn uni_poly() -> UnivariatePoly<Fp256<MontBackend<FrConfig, 4>>> {
        UnivariatePoly::new(vec![
            (Fr::from(6), Fr::from(2)),
            (Fr::from(3), Fr::from(1)),
            (Fr::from(5), Fr::from(0)),
        ])
    }

    #[test]
    fn test_univariate_poly() {
        // tuple of (coeff, exponent)
        let univariate_poly = get_vec();
        assert_eq!(univariate_poly.degree(), Fr::from(2));
    }

    #[test]
    fn test_evaluate() {
        let uni_poly = get_vec();
        let result = uni_poly.full_coeff_evaluation(Fr::from(2));
        assert_eq!(result, Fr::from(34));
    }

    #[test]
    fn test_interpolate() {
        //points x and y
        let points = Points::new(
            vec![(Fr::from(2), Fr::from(4)), (Fr::from(3), Fr::from(5))]
        );
        let result = points.interpolate();

        assert_eq!(result.degree, Fr::from(1));
        println!("result = {:?}", result);
        assert_eq!(
            result.co_ex,
            vec![(Fr::from(1), Fr::from(1)), (Fr::from(2), Fr::from(0))]
        );
    }
    #[test]
    fn test_generate_points() {
        let uni_poly = uni_poly();

        let result = uni_poly.generate_points();
        let points = Points::new(vec![
            (Fr::from(0), Fr::from(5)),
            (Fr::from(1), Fr::from(14)),
            (Fr::from(2), Fr::from(35)),
        ]);
        assert_eq!(result, points);
        // println!("{:?}", result);
    }

    #[test]
    fn test_generate_points_and_interpolate() {
        let uni_poly = uni_poly();
        let points = uni_poly.generate_points();

        println!("{:?}", points);
        let new_poly = points.interpolate();
        assert_eq!(new_poly, uni_poly);
        // println!("{:?}", result);
    }
}
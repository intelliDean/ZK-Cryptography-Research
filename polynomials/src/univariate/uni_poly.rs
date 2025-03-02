use std::collections::HashMap;
// use crate::univariate::point::Points;
use crate::univariate::uni_point::{Points, XAndY};
use ark_ff::PrimeField;
// use crate::univariate::uni_point::Points;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term<F: PrimeField> {
    pub coeff: F,
    pub exp: F,
}

impl<F: PrimeField> Term<F> {
    pub fn new(coeff: F, exp: F) -> Self {
        Self { coeff, exp }
    }
}

//=================================================================

//SPARSE COEFFICIENT REPRESENTATION
#[derive(Debug, Clone, PartialEq)]
pub struct UnivariatePoly<F: PrimeField> {
    pub co_ex: Vec<Term<F>>,
    pub degree: F,
}

impl<F: PrimeField> UnivariatePoly<F> {
    // this acts as the constructor
    pub fn new(co_exp: Vec<Term<F>>) -> UnivariatePoly<F> {
        // sparse rep, this sets the degree of the polynomial
        UnivariatePoly {
            co_ex: co_exp.clone(),
            degree: co_exp
                .iter()
                .map(|term| term.exp)
                .max()
                .unwrap_or(F::zero()),
        }
    }
    pub fn degree(&self) -> F {
        self.degree
    }

    pub fn is_zero(&self) -> bool {
        self.co_ex.is_empty() || self.co_ex.iter().all(|t| t.coeff.is_zero())
    }

    pub fn full_coeff_evaluation(&self, eval_at: F) -> F {
        self.co_ex.iter().
            // result is initialized to zero
            fold(F::zero(), |result, term| {  //the reference of each co_ex is gotten
                let eval = term.coeff * eval_at.pow(term.exp.into_bigint().as_ref()); //coeff x var_value ^ exponent
                println!("result = {}", result + eval); // Print intermediate results
                result + eval
            })
    }

    pub fn generate_points(&self) -> Points<F> {
        let degree_int = self.degree.into_bigint().as_ref()[0]; // Convert degree to u64
        let points = (0..=degree_int) // degree + 1 points
            .map(|i| {
                let x = F::from(i); // Convert i to field element
                let y = self.full_coeff_evaluation(x); // Evaluate polynomial at x
                XAndY::new(x, y) // Create XAndY instance
            })
            .collect(); // Collect into Vec<XAndY<F>>

        Points { points } // Return Points with the collected Vec
    }

    pub fn divide_polynomials(
        self,
        divisor: UnivariatePoly<F>,
    ) -> (UnivariatePoly<F>, UnivariatePoly<F>) {
        if divisor.co_ex.is_empty() || divisor.co_ex[0].coeff.is_zero() {
            panic!("Division by zero polynomial or empty divisor");
        }

        let mut quotient = Vec::new();
        let mut remainder = self.co_ex;

        // Early return if dividend degree is less than divisor degree
        if remainder.is_empty() || remainder[0].exp < divisor.co_ex[0].exp {
            return (
                UnivariatePoly::new(quotient),
                UnivariatePoly::new(remainder),
            );
        }

        while !remainder.is_empty() && remainder[0].exp >= divisor.co_ex[0].exp {
            // Compute leading term of quotient
            let lead_coeff = remainder[0].coeff * divisor.co_ex[0].coeff.inverse().unwrap();
            let lead_exp = remainder[0].exp - divisor.co_ex[0].exp; // Assumes F supports subtraction
            let quotient_term = Term::new(lead_coeff, lead_exp);
            quotient.push(quotient_term.clone());

            // Subtract divisor * quotient_term from remainder efficiently
            let mut remainder_map: HashMap<F, F> =
                remainder.iter().map(|t| (t.exp, t.coeff)).collect();

            for div_term in &divisor.co_ex {
                let new_exp = div_term.exp + quotient_term.exp; // Assumes F supports addition
                let new_coeff = div_term.coeff * quotient_term.coeff;
                remainder_map
                    .entry(new_exp)
                    .and_modify(|c| *c -= new_coeff) // Direct subtraction
                    .or_insert(F::zero() - new_coeff);
            }

            // Convert back to Vec, filter out zero coefficients, and sort
            remainder = remainder_map
                .into_iter()
                .filter(|&(_, coeff)| !coeff.is_zero())
                .map(|(exp, coeff)| Term::new(coeff, exp))
                .collect();
            remainder.sort_unstable_by(|a, b| b.exp.cmp(&a.exp)); // Requires F: Ord
        }

        // If remainder is empty, return a zero polynomial
        if remainder.is_empty() {
            remainder = vec![Term::new(F::zero(), F::zero())];
        }

        (
            UnivariatePoly::new(quotient),
            UnivariatePoly::new(remainder),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Fr, FrConfig};
    use ark_ff::{Fp256, MontBackend};

    fn get_vec() -> UnivariatePoly<Fp256<MontBackend<FrConfig, 4>>> {
        UnivariatePoly::new(vec![
            Term::new(Fr::from(2), Fr::from(2)),
            Term::new(Fr::from(5), Fr::from(1)),
            Term::new(Fr::from(16), Fr::from(0)),
        ])
    }
    fn uni_poly() -> UnivariatePoly<Fp256<MontBackend<FrConfig, 4>>> {
        UnivariatePoly::new(vec![
            Term::new(Fr::from(6), Fr::from(2)),
            Term::new(Fr::from(3), Fr::from(1)),
            Term::new(Fr::from(5), Fr::from(0)),
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
        let points = Points::new(vec![
            XAndY::new(Fr::from(2), Fr::from(4)),
            XAndY::new(Fr::from(3), Fr::from(5)),
        ]);
        let result = points.lagrange_interpolate();

        assert_eq!(result.degree, Fr::from(1));
        println!("result = {:?}", result);
        assert_eq!(
            result.co_ex,
            vec![
                Term::new(Fr::from(1), Fr::from(1)),
                Term::new(Fr::from(2), Fr::from(0))
            ]
        );
    }
    #[test]
    fn test_generate_points() {
        let uni_poly = uni_poly();

        let result = uni_poly.generate_points();
        let points = Points::new(vec![
            XAndY::new(Fr::from(0), Fr::from(5)),
            XAndY::new(Fr::from(1), Fr::from(14)),
            XAndY::new(Fr::from(2), Fr::from(35)),
        ]);
        assert_eq!(result, points);
        // println!("{:?}", result);
    }

    #[test]
    fn test_generate_points_and_interpolate() {
        let uni_poly = uni_poly();
        let points = uni_poly.generate_points();

        println!("{:?}", points);
        let new_poly = points.newton_interpolate();
        assert_eq!(new_poly, uni_poly);
        println!("{:?}", new_poly);
    }

    #[test]
    fn test_divide_polynomials() {
        // Example using a simple field (e.g., Fp64 for testing)
        let dividend = UnivariatePoly::new(vec![
            Term::new(Fr::from(1), Fr::from(2)),   // x^2
            Term::new(Fr::from(3), Fr::from(1)),   // 2x
            Term::new(Fr::from(-18), Fr::from(0)), // 1
        ]);
        let divisor = UnivariatePoly::new(vec![
            Term::new(Fr::from(1), Fr::from(1)),  // x
            Term::new(Fr::from(-3), Fr::from(0)), // 1
        ]);
        let (quotient, remainder) = dividend.divide_polynomials(divisor);

        assert_eq!(
            quotient.co_ex,
            vec![
                // Resultant Polynomial: 3x^2 + 8x + 20
                Term {
                    coeff: Fr::from(1),
                    exp: Fr::from(1)
                },
                Term {
                    coeff: Fr::from(6),
                    exp: Fr::from(0)
                }
            ]
        );

        assert_eq!(
            remainder.co_ex,
            vec![
                //  Remainder: 74
                Term {
                    coeff: Fr::from(0),
                    exp: Fr::from(0)
                }
            ]
        );

        assert_eq!(quotient.degree(), Fr::from(1)); // x + 1
        assert_eq!(remainder.degree(), Fr::from(0)); // 0
        assert!(remainder.is_zero());

        println!("Quotient: {:?}", quotient);
        println!("Remainder: {:?}", remainder);
    }
}

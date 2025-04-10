use crate::univariate::uni_poly::{Term, UnivariatePoly};
use ark_ff::PrimeField;
use std::collections::HashMap;
// use crate::univariate::uni_poly::{Term, UnivariatePoly};

#[derive(Debug, Clone, PartialEq)]
pub struct XAndY<F: PrimeField> {
    pub x: F,
    pub y: F,
}

impl<F: PrimeField> XAndY<F> {
    pub fn new(x: F, y: F) -> Self {
        Self { x, y }
    }
}
//===============================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct Points<F: PrimeField> {
    pub points: Vec<XAndY<F>>,
}

impl<F: PrimeField> Points<F> {
    pub fn new(points: Vec<XAndY<F>>) -> Points<F> {
        Points { points }
    }
    pub fn lagrange_interpolate(&self) -> UnivariatePoly<F> {
        let mut polynomial = Vec::new();

        for (i, point_i) in self.points.iter().enumerate() {
            // Initialize basis polynomial as 1 (constant term)
            let mut basis_poly: HashMap<usize, F> = HashMap::new();
            basis_poly.insert(0, F::one());

            // Compute Lagrange basis polynomial
            for (j, point_j) in self.points.iter().enumerate() {
                if i == j {
                    continue;
                }
                if point_i.x == point_j.x {
                    panic!("Duplicate x values in interpolation points");
                }

                // Compute factor = 1 / (x_i - x_j)
                let factor = (point_i.x - point_j.x)
                    .inverse()
                    .expect("Division by zero in field arithmetic");

                // Multiply basis by (x - x_j) * factor
                let mut new_basis = HashMap::new();
                for (deg, &coeff) in &basis_poly {
                    // x term: coeff * factor * x (degree + 1)
                    *new_basis.entry(deg + 1).or_insert(F::zero()) += coeff * factor;
                    // constant term: -coeff * factor * x_j
                    *new_basis.entry(*deg).or_insert(F::zero()) -= coeff * factor * point_j.x;
                }
                basis_poly = new_basis;
            }

            // Scale basis by y_i and add to polynomial
            for (deg, coeff) in basis_poly {
                Self::add_term1(
                    &mut polynomial,
                    Term {
                        coeff: coeff * point_i.y,
                        exp: F::from(deg as u32),
                    },
                );
            }
        }
        polynomial.sort_unstable_by(|a, b| b.exp.cmp(&a.exp));
        UnivariatePoly::new(polynomial)
    }

    pub fn newton_interpolate(&self) -> UnivariatePoly<F> {
        if self.points.is_empty() {
            return UnivariatePoly::new(vec![Term {
                coeff: F::zero(),
                exp: F::zero(),
            }]);
        }

        // Compute divided differences
        let n = self.points.len();
        let mut divided_diffs = vec![vec![F::zero(); n]; n];

        // Initialize with y-values
        for (i, point) in self.points.iter().enumerate() {
            divided_diffs[i][0] = point.y;
        }

        // Calculate divided differences
        for j in 1..n {
            for i in 0..n - j {
                let denom = self.points[i + j].x - self.points[i].x;
                if denom.is_zero() {
                    panic!("Duplicate x values in Newton interpolation");
                }
                divided_diffs[i][j] = (divided_diffs[i + 1][j - 1] - divided_diffs[i][j - 1])
                    / denom;
            }
        }

        // Build the Newton polynomial
        let mut polynomial = Vec::new();
        Self::add_term1(&mut polynomial, Term {
            coeff: divided_diffs[0][0],
            exp: F::zero(),
        });

        // For each term in the Newton form: f[x_0,...,x_k] * (x - x_0) * ... * (x - x_{k-1})
        for k in 1..n {
            let coeff = divided_diffs[0][k];
            let mut basis = HashMap::new();
            basis.insert(0, F::one()); // Start with constant 1

            // Multiply by (x - x_i) for i = 0 to k-1
            for i in 0..k {
                let x_i = self.points[i].x;
                let mut new_basis = HashMap::new();
                for (deg, &coeff) in &basis {
                    // x term: coeff * x
                    *new_basis.entry(deg + 1).or_insert(F::zero()) += coeff;
                    // constant term: -coeff * x_i
                    *new_basis.entry(*deg).or_insert(F::zero()) -= coeff * x_i;
                }
                basis = new_basis;
            }

            // Scale by the divided difference and add to polynomial
            for (deg, basis_coeff) in basis {
                Self::add_term1(&mut polynomial, Term {
                    coeff: basis_coeff * coeff,
                    exp: F::from(deg),
                });
            }
        }
        polynomial.sort_unstable_by(|a, b| b.exp.cmp(&a.exp));

        UnivariatePoly::new(polynomial)
    }

    fn add_term1(polynomial: &mut Vec<Term<F>>, term: Term<F>) {
        if term.coeff.is_zero() {
            return; // Skip zero terms
        }
        match polynomial.iter_mut().find(|t| t.exp == term.exp) {
            Some(existing) => {
                existing.coeff += term.coeff;
                if existing.coeff.is_zero() {
                    // Remove term if coefficient becomes zero
                    polynomial.retain(|t| !t.coeff.is_zero());
                }
            }
            None => polynomial.push(term),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    // use ark_bn254::Fr;
    use field_tracker::{print_summary, Ft};
    type Fr = Ft!(ark_bn254::Fr);

    #[test]
    fn test_lagrange_interpolate() {
        //points x and y
        let points = Points::new(vec![
            XAndY::new(Fr::from(0), Fr::from(1)),
            XAndY::new(Fr::from(1), Fr::from(2)),
            XAndY::new(Fr::from(2), Fr::from(5)),
        ]);
        let result = points.lagrange_interpolate();
        //add: 36, sub: 36, mul: 88, inv: 12


        // assert_eq!(result.degree, Fr::from(1));
        println!("result = {:?}", result.co_ex);
        // assert_eq!(
        //     result.co_ex,
        //     vec![Term::new(Fr::from(1), Fr::from(1)), Term::new(Fr::from(2), Fr::from(0))]
        // );

        print_summary!();
    }

    #[test]
    fn test_newton_interpolate() {
        //points x and y
        // let points = Points::new(vec![
        //     XAndY::new(Fr::from(2), Fr::from(4)),
        //     XAndY::new(Fr::from(3), Fr::from(5)),
        // ]);

        let points = Points::new(vec![
            XAndY::new(Fr::from(0), Fr::from(1)),
            XAndY::new(Fr::from(1), Fr::from(2)),
            XAndY::new(Fr::from(2), Fr::from(4)),
        ]);
        let result = points.newton_interpolate();
        //add: 16, sub: 22, mul: 19, inv: 0

        // assert_eq!(result.degree, Fr::from(1));
        println!("result = {:?}", result);
        // assert_eq!(
        //     result,
        //     UnivariatePoly::new(vec![Term::new(Fr::from(1), Fr::from(1)), Term::new(Fr::from(2), Fr::from(0))])
        // );

        print_summary!();
    }
}

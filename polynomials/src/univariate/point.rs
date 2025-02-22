use std::collections::HashMap;
use ark_ff::PrimeField;
use crate::univariate::univariate_polynomial::UnivariatePoly;

#[derive(Debug, Clone, PartialEq)]
pub struct Points<F: PrimeField> {
    pub x_y: Vec<(F, F)>,
}

impl<F: PrimeField> Points<F> {
    pub fn new(x_y: Vec<(F, F)>) -> Points<F> {
        Points { x_y }
    }
    pub fn interpolate(&self) -> UnivariatePoly<F> {
        let mut polynomial: Vec<(F, F)> = Vec::new();

        for (i, &(x_i, y_i)) in self.x_y.iter().enumerate() {
            // Initialize basis polynomial as 1 (constant term)
            let mut basis_poly: HashMap<F, F> = HashMap::new();
            basis_poly.insert(F::zero(), F::one());

            for (j, &(x_j, _)) in self.x_y.iter().enumerate() {
                if i == j {
                    continue; // Skip the current point
                }
                // Lagrange basis factor
                let factor = F::one() / (x_i - x_j);
                let mut new_basis_poly: HashMap<F, F> = HashMap::new();

                // Multiply the current basis polynomial by (x - x_j) * factor
                for (&deg, &coeff) in &basis_poly {
                    // Multiply by x (shift degree by 1)
                    *new_basis_poly.entry(deg + F::one()).or_insert(F::zero()) += coeff * factor;
                    // Subtract x_j (constant term)
                    *new_basis_poly.entry(deg).or_insert(F::zero()) += -coeff * factor * x_j;
                }

                basis_poly = new_basis_poly;
            }

            // Multiply the basis polynomial by y_i and add to the final polynomial
            for (&deg, &coeff) in &basis_poly {
                Self::add_term(&mut polynomial, (coeff * y_i, deg));
            }
        }
        // Sort the polynomial by degree in descending order
        polynomial.sort_by(|a, b| b.1.cmp(&a.1));
        UnivariatePoly::new(polynomial)
    }

    // Adds a term to the polynomial, combining like terms if necessary.
    fn add_term(polynomial: &mut Vec<(F, F)>, term: (F, F)) {
        if let Some(existing) = polynomial.iter_mut().find(|(_, d)| *d == term.1) {
            existing.0 += term.0; // Combine like terms
        } else {
            polynomial.push(term); // Add new term
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_interpolate() {

    }
}
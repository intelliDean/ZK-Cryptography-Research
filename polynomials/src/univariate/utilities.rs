use std::collections::HashMap;

#[derive(Debug)]
pub struct Points {
    pub coefficient: usize,
    pub degree: usize,
}

pub fn interpolate(points: &[(usize, usize)]) -> HashMap<usize, f64> {
    // This will store the resulting polynomial coefficients
    let mut polynomial: HashMap<usize, f64> = HashMap::new();

    for (i, &(x_i, y_i)) in points.iter().enumerate() {
        let mut basis_poly: HashMap<usize, f64> = HashMap::new();
        basis_poly.insert(0, 1.0); // Initialize basis polynomial as 1

        // Construct the Lagrange basis polynomial L_i(x)
        for (j, &(x_j, _)) in points.iter().enumerate() {
            if i == j {
                continue; // Skip when i == j
            }

            let factor = 1.0 / ((x_i as f64) - (x_j as f64));

            // Update the basis polynomial by multiplying with (x - x_j)
            let mut new_basis_poly: HashMap<usize, f64> = HashMap::new();

            for (&degree, &coeff) in &basis_poly {
                // Multiply each term by x
                *new_basis_poly.entry(degree + 1).or_insert(0.0) += coeff * factor;

                // Subtract x_j times the current term
                *new_basis_poly.entry(degree).or_insert(0.0) -= coeff * factor * x_j as f64;
            }
            basis_poly = new_basis_poly;
        }

        // Multiply L_i(x) by y_i and add to the resulting polynomial
        for (&degree, &coeff) in &basis_poly {
            *polynomial.entry(degree).or_insert(0.0) += coeff * y_i as f64;
        }
    }

    polynomial
}

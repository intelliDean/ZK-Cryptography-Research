use ark_ff::PrimeField;

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct Term2 {
    coeff: f64,
    exp: u32,
}

fn divide_polynomials_f64(dividend: Vec<Term2>, divisor: Vec<Term2>) -> (Vec<Term2>, Vec<Term2>) {
    let mut quotient = Vec::new();
    let mut remainder = dividend.clone();

    while !remainder.is_empty() && remainder[0].exp >= divisor[0].exp {
        // Compute leading term of quotient
        let lead_coeff = remainder[0].coeff / divisor[0].coeff; // Floating-point division
        let lead_exp = remainder[0].exp - divisor[0].exp;
        let quotient_term = Term2 {
            coeff: lead_coeff,
            exp: lead_exp,
        };
        quotient.push(quotient_term.clone());

        // Multiply quotient term by divisor
        let mut to_subtract = HashMap::new();
        for term in &divisor {
            let new_exp = term.exp + quotient_term.exp;
            let new_coeff = term.coeff * quotient_term.coeff;
            *to_subtract.entry(new_exp).or_insert(0.0) += new_coeff;
        }

        // Subtract from remainder
        let mut new_remainder = HashMap::new();
        for term in &remainder {
            *new_remainder.entry(term.exp).or_insert(0.0) += term.coeff;
        }
        for (exp, coeff) in to_subtract {
            *new_remainder.entry(exp).or_insert(0.0) -= coeff;
        }

        // Convert back to Vec<Term> and remove zero coefficients
        remainder = new_remainder
            .into_iter()
            .filter(|(_, coeff)| coeff.abs() > 1e-9) // Avoid floating-point precision issues
            .map(|(exp, coeff)| Term2 { coeff, exp })
            .collect::<Vec<_>>();
        remainder.sort_by(|a, b| b.exp.cmp(&a.exp)); // Ensure highest degree term first
    }

    (quotient, remainder)
}

#[cfg(test)]

mod tests {
    use super::*;
    use ark_bn254::{Fr, FrConfig};
    #[test]
    fn test_polynomial_division_f64() {
        // Example: (3x^4 + 6x^3 + 2x - 16) / (3x - 4)
        let dividend = vec![
            Term2 { coeff: 1.0, exp: 2 },
            Term2 { coeff: 3.0, exp: 1 },
            Term2 {
                coeff: -18.0,
                exp: 0,
            }
        ];
        let divisor = vec![
            Term2 { coeff: 1.0, exp: 1 },
            Term2 {
                coeff: -3.0,
                exp: 0,
            },
        ];

        let (quotient, remainder) = divide_polynomials_f64(dividend, divisor);

        println!("Quotient: {:?}", quotient);
        println!("Remainder: {:?}", remainder);
    }
}

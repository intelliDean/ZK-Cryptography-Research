use ark_ff::PrimeField;

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
struct Term2 {
    coeff: f64,
    exp: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Term<F: PrimeField> {
    coeff: F,
    exp: u32,
}

fn divide_polynomials<F: PrimeField>(
    dividend: Vec<Term<F>>,
    divisor: Vec<Term<F>>,
) -> (Vec<Term<F>>, Vec<Term<F>>) {
    let mut quotient = Vec::new();
    let mut remainder = dividend.clone();

    while !remainder.is_empty() && remainder[0].exp >= divisor[0].exp {
        // to compute leading term of quotient
        let lead_coeff = remainder[0].coeff * divisor[0].coeff.inverse().unwrap(); // Modular division

        let lead_exp = remainder[0].exp - divisor[0].exp;
        let quotient_term = Term { coeff: lead_coeff, exp: lead_exp };
        quotient.push(quotient_term.clone());

        // multiply quotient term by divisor
        let mut to_subtract = HashMap::new();
        for term in &divisor {
            let new_exp = term.exp + quotient_term.exp;
            let new_coeff = term.coeff * quotient_term.coeff;
            to_subtract.entry(new_exp).or_insert(F::ZERO).add_assign(new_coeff);
        }

        // subtract from the remainder
        let mut new_remainder = HashMap::new();
        for term in &remainder {
            new_remainder
                .entry(term.exp)
                .or_insert(F::ZERO)
                .add_assign(term.coeff);
        }
        for (exp, coeff) in to_subtract {
            new_remainder.entry(exp).or_insert(F::ZERO).sub_assign(coeff);
        }

        // i converted back to Vec<Term<F>> and remove zero coefficients
        remainder = new_remainder
            .into_iter()
            .filter(|(_, coeff)| bool::from(!coeff.is_zero()))
            .map(|(exp, coeff)| Term { coeff, exp })
            .collect::<Vec<_>>();
        remainder.sort_by(|a, b| b.exp.cmp(&a.exp)); // Ensure the highest degree term first
    }

    (quotient, remainder)
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
            Term2 { coeff: -10.0, exp: 0 },
            // Term2 {
            //     coeff: -16.0,
            //     exp: 0,
            // },
        ];
        let divisor = vec![
            Term2 { coeff: 1.0, exp: 1 },
            Term2 {
                coeff: -2.0,
                exp: 0,
            },
        ];

        let (quotient, remainder) = divide_polynomials_f64(dividend, divisor);

        println!("Quotient: {:?}", quotient);
        println!("Remainder: {:?}", remainder);
    }

    #[test]
    fn test_polynomial_division() {
        // Example: (6x^3 + 4x^2 + 8x - 6) / (2x - 4)
        let dividend = vec![
            Term { coeff: Fr::from(6), exp: 3 },
            Term { coeff: Fr::from(4), exp: 2 },
            Term { coeff: Fr::from(8), exp: 1 },
            Term {
                coeff: Fr::from(-6),
                exp: 0,
            },
        ];
        let divisor = vec![
            Term { coeff: Fr::from(2), exp: 1 },
            Term {
                coeff: Fr::from(-4),
                exp: 0,
            },
        ];

        let (quotient, remainder) = divide_polynomials(dividend, divisor);

        assert_eq!(quotient, vec![ // Resultant Polynomial: 3x^2 + 8x + 20
            Term { coeff: Fr::from(3), exp: 2 },
            Term { coeff: Fr::from(8), exp: 1 },
            Term { coeff: Fr::from(20), exp: 0 }
        ]);

        assert_eq!(remainder, vec![ //  Remainder: 74
            Term { coeff: Fr::from(74), exp: 0 }
        ]);


        println!("Quotient: {:?}", quotient);
        println!("Remainder: {:?}", remainder);
    }
}
use std::collections::HashSet;

pub(crate) fn polynomial_evaluation_form(expression: &str) -> Vec<i32> {
    let mut terms = Vec::new();
    let mut unique_vars: HashSet<u32> = HashSet::new();

    check_signs(expression); // Validate the expression for invalid signs once.

    for term in expression.split(['+', '-']) {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }

        // Determine if the term is negative.
        let is_negative = expression.contains(&format!("- {}", term))
            || expression.contains(&format!("-{}", term));

        // Extract coefficient or default to 1.
        let coeff_str: String = term.chars().take_while(|c| c.is_numeric()).collect();
        let mut coeff: i32 = coeff_str.parse().unwrap_or(1);
        if is_negative {
            coeff = -coeff;
        }

        // Extract variables and add to the unique set.
        let vars: String = term.chars().filter(|c| c.is_alphabetic()).collect();
        for c in vars.chars() {
            unique_vars.insert(c as u32);
        }

        terms.push((vars, coeff));
    }

    // Convert unique_vars HashSet to a sorted Vec for consistent ordering.
    let mut unique_vars: Vec<u32> = unique_vars.into_iter().collect();
    unique_vars.sort_unstable();

    // Compute the number of variables and binary evaluations.
    let num_var = unique_vars.len();
    let num_bin = 2_u32.pow(num_var as u32);

    // Compute evaluations and return the result.
    compute_evaluation1(terms, unique_vars, num_var, num_bin).unwrap_or_default()
}

fn compute_evaluation1(
    terms: Vec<(String, i32)>,
    unique_vars: Vec<u32>,
    num_var: usize,
    num_bin: u32,
) -> Option<Vec<i32>> {
    let mut max_result: i32 = 0;
    // let mut x_and_y: Vec<(Vec<u32>, i32)> = Vec::new();
    let mut x_and_y: Vec<(Vec<u32>, i32)> = Vec::with_capacity(num_bin as usize);
    let mut eval_poly: Vec<i32> = Vec::with_capacity(num_bin as usize);

    for num in 0..num_bin {
        // let mut binary_vec = Vec::with_capacity(num_var);
        let mut res: i32 = 0;

        let binary_vec: Vec<u32> = (0..num_var)
            .map(|i| ((num >> (num_var - 1 - i)) & 1))
            .collect();

        for (term_vars, coeff) in &terms {
            let mut term_value: u32 = 1;

            for var in term_vars.chars() {
                let var_index = unique_vars.iter().position(|&v| v == var as u32)?;
                term_value *= binary_vec[var_index];
            }

            res += coeff * term_value as i32;
        }
        eval_poly.push(res);

        x_and_y.push((binary_vec, res));
        print_result(&x_and_y.last().unwrap().0, res); // Pass reference to avoid cloning
        max_result = max_result.max(res);
    }

    println!("Max Evaluation: {}", max_result);
    Some(eval_poly)
}

fn check_signs(expression: &str) {
    if expression
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '+' && c != '-' && !c.is_whitespace())
    {
        panic!("Invalid polynomial : '{}'", expression);
    }
}

fn print_result(binary_vec: &Vec<u32>, res: i32) {
    println!("{:?} = {}", binary_vec, res);
}

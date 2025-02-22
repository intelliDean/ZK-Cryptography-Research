use std::collections::HashSet;

pub(crate) fn eval_bool_hypercube_with_expression(expression: &str) -> Vec<(Vec<u32>, i32)> {
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
    compute_evaluation(terms, unique_vars, num_var, num_bin).unwrap_or_default()
}

fn check_signs(expression: &str) {
    if expression
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '+' && c != '-' && !c.is_whitespace())
    {
        panic!("Invalid polynomial : '{}'", expression);
    }
}

pub(crate) fn eval_bool_hypercube_with_rep(
    representation: Vec<(String, i32)>,
) -> Vec<(Vec<u32>, i32)> {
    let mut unique_vars: Vec<u32> = Vec::new();

    for term in &representation {
        for c in term.0.chars() {
            let c_u32 = c as u32;
            if !unique_vars.contains(&c_u32) {
                unique_vars.push(c_u32);
            }
        }
    }
    unique_vars.sort_unstable();

    let num_var = unique_vars.len();
    let num_bin = 2_u32.pow(num_var as u32);

    let evaluation = compute_evaluation(representation, unique_vars, num_var, num_bin);

    evaluation.unwrap_or(vec![])
}

fn compute_evaluation(
    terms: Vec<(String, i32)>,
    unique_vars: Vec<u32>,
    num_var: usize,
    num_bin: u32,
) -> Option<Vec<(Vec<u32>, i32)>> {
    let mut max_result: i32 = 0;
    // let mut x_and_y: Vec<(Vec<u32>, i32)> = Vec::new();
    let mut x_and_y: Vec<(Vec<u32>, i32)> = Vec::with_capacity(num_bin as usize);

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

        x_and_y.push((binary_vec, res));
        print_result(&x_and_y.last().unwrap().0, res); // Pass reference to avoid cloning
        max_result = max_result.max(res);

    }


    println!("Max Evaluation: {}", max_result);
    Some(x_and_y)
}

fn print_result(binary_vec: &Vec<u32>, res: i32) {
    println!("{:?} = {}", binary_vec, res);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_eval_bool_hypercube_pos() {
        let expression = "2ab + 3bc";
        let result = eval_bool_hypercube_with_expression(expression);

        assert_eq!(
            result,
            vec![
                (vec![0, 0, 0], 0),
                (vec![0, 0, 1], 0),
                (vec![0, 1, 0], 0),
                (vec![0, 1, 1], 3),
                (vec![1, 0, 0], 0),
                (vec![1, 0, 1], 0),
                (vec![1, 1, 0], 2),
                (vec![1, 1, 1], 5)
            ]
        );
    }
    #[test]
    fn test_eval_bool_hypercube_neg() {
        let expression = "2ab + 3cd - 2e";
        let result = eval_bool_hypercube_with_expression(expression);


        // assert_eq!(
        //     result,
        //     vec![
        //         (vec![0, 0, 0], 0),
        //         (vec![0, 0, 1], -2),
        //         (vec![0, 1, 0], 0),
        //         (vec![0, 1, 1], -2),
        //         (vec![1, 0, 0], 2),
        //         (vec![1, 0, 1], 0),
        //         (vec![1, 1, 0], 5),
        //         (vec![1, 1, 1], 3)
        //     ]
        // );
    }
    #[test]
    #[should_panic]
    fn test_eval_bool_hypercube_should_panic() {
        let expression = "2a + 3ab * 2c";
        let result = eval_bool_hypercube_with_expression(expression);
    }

    #[test]
    fn test_eval_bool_hypercube_rep() {
        let representation = vec![(String::from("ab"), 3), (String::from("c"), -1)];
        let result2 = eval_bool_hypercube_with_rep(representation);
        assert_eq!(
            result2,
            vec![
                (vec![0, 0, 0], 0),
                (vec![0, 0, 1], -1),
                (vec![0, 1, 0], 0),
                (vec![0, 1, 1], -1),
                (vec![1, 0, 0], 0),
                (vec![1, 0, 1], -1),
                (vec![1, 1, 0], 3),
                (vec![1, 1, 1], 2)
            ]
        );
    }
}

// https://github.com/intelliDean/ZeeKay.git

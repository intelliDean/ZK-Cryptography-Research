use std::ops::{Add, Mul};
use ark_ff::{BigInteger, PrimeField};
use num_traits::{NumCast, ToPrimitive};

#[derive(Debug, Clone, PartialEq)]
pub struct MultilinearPoly<F: PrimeField> {
    pub polynomial: Vec<F>,
}

pub trait Multilinear<F: PrimeField> {
    fn new(polynomial: Vec<F>) -> Self;
    fn partial_evaluation(self, var_pos: u32, eval_var_at: F) -> MultilinearPoly<F>;
    fn full_evaluation(self, eval_at: Vec<F>) -> F;
    fn multi_partial_evaluate(&self, values: &[F]) -> Self;
    fn multiply_by(&self, value: F) -> Self;
    fn convert_to_bytes(&self) -> Vec<u8>;
    fn tensor_add(self, other: MultilinearPoly<F>) -> MultilinearPoly<F>;
    fn tensor_multiply(self, other: MultilinearPoly<F>) -> MultilinearPoly<F>;
    fn element_sum(self, other: MultilinearPoly<F>) -> MultilinearPoly<F>;
    fn num_var(&self) -> u32;
    fn scalar_mul(self, field_element: F) -> Self;

    /// generates pairs of indices for partial evaluation.
    /// `index` is the position of the variable to evaluate, and `n_vars` is the total number of variables.
    fn pairs(index: usize, n_vars: usize) -> impl Iterator<Item = (usize, usize)> {
        let inverted_index = n_vars - index - 1;
        (0..(1 << (n_vars - 1))).map(move |val| {
            let insert_zero = Self::insert_bit(val, inverted_index);
            let insert_one = insert_zero | (1 << inverted_index);
            (insert_zero, insert_one)
        })
    }
    /// Inserts a zero bit at the specified index in the binary representation of `value`.
    fn insert_bit(value: usize, index: usize) -> usize {
        let high = value >> index;
        let low = value & ((1 << index) - 1);
        (high << (index + 1)) | low
    }

    fn partial_evaluation2(self, var_pos: u32, eval_var_at: F) -> MultilinearPoly<F>;

}

impl<F: PrimeField> Multilinear<F> for MultilinearPoly<F> {
    fn new(polynomial: Vec<F>) -> Self {
        Self { polynomial }
    }

    //TODO: MOST EFFICIENT
    /// Partially evaluates the multilinear polynomial at the given variable position.
    /// `var_pos` is the index of the variable to evaluate, and `eval_var_at` is the value to evaluate it at.
    /// Returns a new `MultilinearPoly` representing the partially evaluated polynomial.
    fn partial_evaluation(self, var_pos: u32, eval_var_at: F) -> MultilinearPoly<F> {
        let poly_len = self.polynomial.len();

        if poly_len == 1 {
            return self;
        }

        assert!(
            poly_len.is_power_of_two(),
            "Polynomial length must be a power of 2! Found: {}",
            poly_len
        );

        let n_vars = poly_len.trailing_zeros();
        assert!(var_pos < n_vars, "Invalid variable position!");

        let new_polynomial: Vec<F> = Self::pairs(var_pos as usize, n_vars as usize)
            .map(|(y1_idx, y2_idx)| {
                let y_1 = self.polynomial[y1_idx];
                let y_2 = self.polynomial[y2_idx];

                y_1 + eval_var_at * (y_2 - y_1) // y1 + r * (y2 - y1)
            })
            .collect();

        MultilinearPoly::new(new_polynomial)
    }

    fn full_evaluation(mut self, eval_at: Vec<F>) -> F {
        let num_vars = eval_at.len() as u32;
        assert_eq!(
            self.num_var(),
            num_vars,
            "Invalid number of vars: {}",
            num_vars
        );

        for eval_value in eval_at {
            //this always starts from the beginning, first with a;
            // then b becomes the first next time
            self = self.partial_evaluation(0, eval_value);
        }

        self.polynomial.pop().unwrap()
    }

    fn multi_partial_evaluate(&self, values: &[F]) -> Self {
        if values.len() > self.num_var() as usize {
            panic!("Invalid number of values");
        }

        let mut poly = self.clone();

        for value in values {
            poly = poly.partial_evaluation(0, *value);
        }

        poly
    }


    // fn multi_partial_evaluate(&self, values: &[F]) -> Self {
    //     if values.len() > self.num_var() as usize {
    //         panic!("Invalid number of values");
    //     }
    //
    //     let mut poly = self.clone();
    //
    //     for value in values {
    //         poly = poly.partial_evaluation(0, *value);
    //     }
    //
    //     poly
    // }

    fn multiply_by(&self, value: F) -> Self {
        let result = self.polynomial.iter().map(|eval| *eval * value).collect();

        Self::new(result)
    }
    fn convert_to_bytes(&self) -> Vec<u8> {
        const BYTES_PER_LIMB: usize = 8;
        let byte_len = F::BigInt::NUM_LIMBS * BYTES_PER_LIMB;

        self.polynomial
            .to_vec()
            .into_iter()
            .flat_map(|element| {
                let mut bytes = element.into_bigint().to_bytes_be();
                let padding = byte_len.saturating_sub(bytes.len());
                std::iter::repeat(0).take(padding).chain(bytes.into_iter())
            })
            .collect()
    }

    // adding each element in the polynomial to each element in the other polynomial
    fn tensor_add(self, other: MultilinearPoly<F>) -> MultilinearPoly<F> {
        let new_poly: Vec<F> = self
            .polynomial
            .into_iter()
            .flat_map(|var| {
                other
                    .polynomial
                    .iter()
                    .map(move |&other_var| var + other_var)
            })
            .collect();

        MultilinearPoly::new(new_poly)
    }

    // multiplying each element in the polynomial to each element in the other polynomial
    fn tensor_multiply(self, other: MultilinearPoly<F>) -> MultilinearPoly<F> {
        let new_poly: Vec<F> = self
            .polynomial
            .into_iter()
            .flat_map(|var| {
                other
                    .polynomial
                    .iter()
                    .map(move |&other_var| var * other_var)
            })
            .collect();

        MultilinearPoly::new(new_poly)
    }
    fn element_sum(self, other: MultilinearPoly<F>) -> MultilinearPoly<F> {
        assert_eq!(
            self.polynomial.len(),
            other.polynomial.len(),
            "Inconsistent polynomials"
        );

        let new_poly: Vec<F> = self
            .polynomial
            .into_iter()
            .zip(other.polynomial)
            .map(|(a, b)| a + b)
            .collect();

        MultilinearPoly::new(new_poly)
    }

    fn num_var(&self) -> u32 {
        self.polynomial.len().ilog2()
    }

    fn scalar_mul(self, field_element: F) -> Self {
        let scaled_values: Vec<F> = self
            .polynomial
            .iter()
            .map(|value| *value * field_element)
            .collect();

        MultilinearPoly::new(scaled_values)
    }

    //partial evaluation taking a vec of PrimeField
    fn partial_evaluation2(self, var_pos: u32, eval_var_at: F) -> MultilinearPoly<F> {
        let poly_len = self.polynomial.len();

        // Ensure the polynomial length is a power of 2
        assert!(
            poly_len > 1 && poly_len.is_power_of_two(),
            "Polynomial length must be a power of 2! Found: {}",
            poly_len
        );

        let num_vars = poly_len.trailing_zeros(); // Number of variables
        assert!(
            (0..num_vars).contains(&var_pos),
            "Invalid variable position! Must be between 0 and {} (inclusive), found: {}",
            num_vars - 1,
            var_pos
        );

        let group_size = 1 << (num_vars - 1 - var_pos); // Adjusted for 0-based indexing

        let mut new_polynomial = Vec::with_capacity(poly_len / 2);

        // Iterate over the polynomial in chunks of size 2 * group_size
        for chunk in self.polynomial.chunks_exact(2 * group_size) {
            for i in 0..group_size {
                let (y_1, y_2) = (chunk[i], chunk[i + group_size]);

                // Perform partial evaluation: y_1 + r * (y_2 - y_1)
                new_polynomial.push(y_1 + eval_var_at * (y_2 - y_1));

                #[cfg(debug_assertions)] // Only prints in debug mode
                println!(
                    "[{:?}, {:?}] = {:?}",
                    y_1,
                    y_2,
                    new_polynomial.last().unwrap()
                );
            }
        }

        MultilinearPoly {
            polynomial: new_polynomial,
        }
    }



}

pub fn partial_evaluation_with_u32(
    polynomial: Vec<u32>,
    var_pos: u32,
    eval_var_at: u32,
) -> Vec<u32> {
    let poly_len = polynomial.len() as u32;

    // Ensure the polynomial length is a power of 2
    assert!(
        poly_len.is_power_of_two(),
        "Polynomial length must be a power of 2! Found: {}",
        poly_len
    );

    let num_vars = poly_len.trailing_zeros(); // Number of variables
    assert!(
        (0..num_vars).contains(&var_pos),
        "Invalid variable position! Must be between 1 and {} (inclusive), found: {}",
        num_vars,
        var_pos
    );

    // let group_size = 1 << (var_pos - 1);
    let group_size = 1 << (num_vars - 1 - var_pos); // Equivalent to 2_u32.pow(var_pos - 1)

    let mut new_polynomial = Vec::with_capacity((poly_len / 2) as usize);

    // Iterate over the polynomial in chunks of size 2 * group_size
    for chunk in polynomial.chunks_exact((2 * group_size) as usize) {
        for i in 0..group_size as usize {
            let (y_1, y_2) = (chunk[i], chunk[i + group_size as usize]);

            // Compute the partial evaluation
            let result = y_1 + eval_var_at * (y_2 - y_1);
            new_polynomial.push(result);

            #[cfg(debug_assertions)]
            println!("[y_1: {:?}, y_2: {:?}] -> {:?}", y_1, y_2, result);
        }
    }

    new_polynomial
}

impl<F: PrimeField> Mul for MultilinearPoly<F> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let result = self
            .polynomial
            .iter()
            .zip(other.polynomial.iter())
            .map(|(a, b)| *a * *b)
            .collect();

        MultilinearPoly::new(result)
    }
}
impl<F: PrimeField> Add for MultilinearPoly<F> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let result = self
            .polynomial
            .iter()
            .zip(other.polynomial.iter())
            .map(|(a, b)| *a + *b)
            .collect();

        MultilinearPoly::new(result)
    }
}

pub fn to_field<T, F: PrimeField>(poly: Vec<T>) -> Vec<F>
where
    F: From<T>,
{
    poly.into_iter().map(F::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use num_traits::{One, Zero};

    #[test]
    fn test_to_field() {
        let num = vec![2, 3, 4, 5];
        let res: Vec<Fr> = to_field(num);
        println!("{:?}", res);
        assert_eq!(
            res,
            vec![Fr::from(2), Fr::from(3), Fr::from(4), Fr::from(5)]
        );
    }

    fn get_poly() -> MultilinearPoly<Fr> {
        MultilinearPoly::new(to_field(vec![0, 0, 0, 2, 0, 0, 3, 5]))
    }
    #[test]
    fn test_partial_evaluation() {
        let polynomial = get_poly();
        // let polynomial = MultilinearPoly::new(to_field(vec![0, 0, 3, 3, 0, 4, 3, 7]));
        let polynomial = MultilinearPoly::new(to_field(vec![2, 3, 4, 5]));
        // let polynomial = MultilinearPoly::new(to_field(vec![88, 0]));

        let var_pos = 1; // Change this to 1, 2, or 3 to tests different cases
        let eval_var_at = Fr::from(0);

        let result = polynomial.partial_evaluation(var_pos, eval_var_at);
        println!("{:?}", result);
        // let expected = MultilinearPoly::new(to_field(vec![0, 8, 3, 11]));
        // assert_eq!(result, expected);
    }

    #[test]
    fn test_sum_polynomial() {
        let poly1: MultilinearPoly<Fr> = MultilinearPoly::new(to_field(vec![3, 3, 3, 5]));
        let poly2 = MultilinearPoly::new(to_field(vec![6, 8]));

        let result = poly1.tensor_add(poly2);
        let expected_poly = MultilinearPoly::new(to_field(vec![9, 11, 9, 11, 9, 11, 11, 13]));

        assert_eq!(result, expected_poly);
    }
    #[test]
    fn test_element_sum_polynomial() {
        let poly1: MultilinearPoly<Fr> = MultilinearPoly::new(to_field(vec![3, 3, 3, 5]));
        let poly2 = MultilinearPoly::new(to_field(vec![0, 0, 3, 7]));

        let result = poly1.element_sum(poly2);
        let expected_poly = MultilinearPoly::new(to_field(vec![3, 3, 6, 12]));

        assert_eq!(result, expected_poly);
    }

    #[test]
    fn test_multiply_polynomial() {
        let poly1: MultilinearPoly<Fr> = MultilinearPoly::new(to_field(vec![3, 5]));
        let poly2 = MultilinearPoly::new(to_field(vec![2, 3, 5]));

        let result = poly1.tensor_multiply(poly2);
        let expected_poly = MultilinearPoly::new(to_field(vec![6, 9, 15, 10, 15, 25]));

        assert_eq!(result, expected_poly);
    }
    #[test]
    fn test_partial_evaluation2() {
        let polynomial = get_poly();

        let var_pos = 0; // Change this to 1, 2, or 3 to tests different cases
        let eval_var_at = Fr::from(2);

        let result = polynomial.partial_evaluation2(var_pos, eval_var_at);
        println!("{:?}", result);
        let expected = MultilinearPoly::new(to_field(vec![0, 0, 6, 8]));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partial_evaluation_with_u32() {
        let polynomial = vec![0, 0, 3, 3, 0, 4, 3, 7];

        let input = 0; // Change this to 1, 2, or 3 to tests different cases

        let result = partial_evaluation_with_u32(polynomial, input, 2);
        assert_eq!(result, vec![0, 8, 3, 11]);
    }
    #[test]
    #[should_panic]
    fn test_partial_evaluation_to_panic() {
        // let polynomial = vec![0, 0, 3, 3, 0, 4, 3, 7];
        let polynomial = vec![0, 1, 0, 0, 0, 0, 0, 0];

        let input = 0; // Change this to 1, 2, or 3 to tests different cases

        let result = partial_evaluation_with_u32(polynomial, input, 4);
        assert_eq!(result, vec![0, 3, 8, 11]);
    }

    #[test]
    fn test_full_evaluation() {
        let polynomial = get_poly();
        // let poly =   MultilinearPoly::new(to_field(vec![2, 3, 4, 5]));

        let var = vec![Fr::from(2), Fr::from(3), Fr::from(5)];
        // let var = vec![Fr::zero(), Fr::one()];
        let result = polynomial.full_evaluation(var);
        println!("{:?}", result);

        assert_eq!(result, Fr::from(48));
    }
}

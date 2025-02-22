use crate::gate::{Gate, Ops};
use crate::layer::Layer;
use ark_ff::PrimeField;
use std::marker::PhantomData;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Circuit<F: PrimeField> {
    pub circuit: Layer,
    pub layer_witness: Vec<MultilinearPoly<F>>,
    _phantom: PhantomData<F>,
}

impl<F: PrimeField> Circuit<F> {
    fn new(layers: Layer) -> Self {
        Circuit {
            circuit: layers,
            layer_witness: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn run_circuit(&mut self, inputs: MultilinearPoly<F>) -> Vec<MultilinearPoly<F>> {
        // Initialize the witness layer with the inputs
        self.layer_witness.push(inputs);

        let mut layers = self.circuit.layers.clone(); // Clone to avoid borrowing conflicts
        layers.reverse();
        println!("Layers to evaluate: {:?}", layers);
        self.compute(&mut layers); // Compute all layers

        self.layer_witness.clone() // Return all computed layer evaluations
    }
    //This returns a layer w_i before it's all evaluated
    pub fn partial_circuit_run(
        &mut self,
        inputs: MultilinearPoly<F>,
        layer_id: usize,
    ) -> MultilinearPoly<F> {
        let length = self.circuit.layers.len();
        println!("Circuit length: {}", length);
        match layer_id {
            // If layer_id == self.circuit.layers.len(),
            // return inputs directly, no need to compute
            _ if layer_id == length => inputs,

            // If layer_id is out of bounds,
            // return empty vector
            _ if layer_id > length => MultilinearPoly::new(vec![]),

            _ => {
                // run the code with any other inputs
                self.circuit.layers.reverse();
                self.layer_witness.push(inputs);
                let idx = length - 1 - layer_id;

                // Convert to Vec to avoid borrowing `self`
                let mut layers = self.circuit.layers[..=idx].to_vec();

                self.compute(&mut layers); // this will compute up to the layer_id

                self.layer_witness.last().unwrap().clone() // Return particular layer evaluation
            }
        }
    }

    fn compute(&mut self, layers: &mut [Vec<Gate>]) {
        for layer in layers {
            // this will make use of the last layer
            // which was last pushed into the vec as the current layer
            if let Some(current_layer) = self.layer_witness.last() {
                // let mut witness = Vec::with_capacity(layer.len()); // Allocate space for new layer
                let mut witness = vec![F::from(0); layer.len()];

                for gate in layer {
                    let left_value = &current_layer.polynomial[gate.left];
                    let right_value = &current_layer.polynomial[gate.right];

                    let result = gate.ops.operation(left_value, right_value);

                    // with this flexibility, you can have 2 or more gates evaluating to one output
                    witness[gate.output] += result;
                }

                // Optionally print witness
                if cfg!(debug_assertions) {
                    println!("witness: {:?}", witness);
                }

                self.layer_witness.push(MultilinearPoly::new(witness));
            }
        }
    }
    //This returns a layer w_i when it's all evaluated
    pub fn w_i_polynomial(&self, layer_index: usize) -> MultilinearPoly<F> {
        if layer_index >= self.clone().layer_witness.len() {
            return MultilinearPoly::new(vec![]);
        }
        self.layer_witness[layer_index].clone()
    }

    pub fn add_i_and_mul_i_mle(
        &mut self,
        layer_index: usize,
    ) -> (MultilinearPoly<F>, MultilinearPoly<F>) {
        let number_of_layer_variables = num_of_layer_variables(layer_index);
        let boolean_hypercube_combinations = 1 << number_of_layer_variables; // 2 ^ number_of_layer_variables

        let mut add_i_values = vec![F::default(); boolean_hypercube_combinations];
        let mut mul_i_values = vec![F::default(); boolean_hypercube_combinations];

        let layer = &self.circuit.layers[layer_index];

        for gate in layer.iter() {
            let position_index =
                combine_and_convert_to_decimal(layer_index, gate.output, gate.left, gate.right);

            match gate.ops {
                Ops::ADD => add_i_values[position_index] = F::one(),
                Ops::MUL => mul_i_values[position_index] = F::one(),
            }
        }

        show_combinations(
            number_of_layer_variables,
            boolean_hypercube_combinations,
            &mut add_i_values,
            &mut mul_i_values,
        );

        let add_i_polynomial = MultilinearPoly::new(add_i_values);
        let mul_i_polynomial = MultilinearPoly::new(mul_i_values);

        (add_i_polynomial, mul_i_polynomial)
    }
}

fn show_combinations<F: PrimeField>(
    number_of_layer_variables: usize,
    boolean_hypercube_combinations: usize,
    add_i_values: &mut Vec<F>,
    mul_i_values: &mut Vec<F>,
) {
    println!("bhc - add_i - mul_i");
    for i in 0..boolean_hypercube_combinations {
        let binary_combination = format!("{:0width$b}", i, width = number_of_layer_variables);
        let add_eval = add_i_values[i];
        let mul_eval = mul_i_values[i];

        println!(
            "{}:  -  {}  -  {}",
            binary_combination, add_eval, mul_eval
        );
    }
}

pub fn num_of_layer_variables(layer_index: usize) -> usize {
    if layer_index == 0 {
        return 3;
    }

    let var_a_length = layer_index;
    let var_b_length = var_a_length + 1;
    let var_c_length = var_a_length + 1;

    let num_of_variables = var_a_length + var_b_length + var_c_length;

    num_of_variables
}

pub fn combine_and_convert_to_decimal(
    layer_index: usize,
    variable_a: usize,
    variable_b: usize,
    variable_c: usize,
) -> usize {
    // Convert each decimal number to a padded binary string
    let a_binary = decimal_to_padded_binary(variable_a, layer_index);
    let b_binary = decimal_to_padded_binary(variable_b, layer_index + 1);
    let c_binary = decimal_to_padded_binary(variable_c, layer_index + 1);

    // Combine the binary strings
    let combined_binary = format!("{}{}{}", a_binary, b_binary, c_binary);

    // Convert the combined binary string back to a decimal number
    usize::from_str_radix(&combined_binary, 2).expect("Failed to parse combined binary string")
}


pub fn decimal_to_padded_binary(decimal_number: usize, bit_length: usize) -> String {
    format!("{:0>width$b}", decimal_number, width = bit_length)
}


#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::multilinear::multilinear::to_field;

    fn get_layer() -> Layer {
        Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)])
    }
    fn get_circuit() -> Circuit<Fr> {
        let mut layers = get_layer();

        let layer1 = vec![Gate::new(0, 0, 1, Ops::MUL), Gate::new(1, 2, 3, Ops::ADD)];

        let layer2 = vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 2, 3, Ops::ADD),
            Gate::new(2, 4, 5, Ops::ADD),
            Gate::new(3, 6, 7, Ops::MUL),
        ];

        layers.update_layer_gate(layer1.clone());
        layers.update_layer_gate(layer2.clone());

        println!("Layers: {:?}", &layers);

        Circuit::new(layers.clone())
    }

    fn get_input() -> MultilinearPoly<Fr> {
        MultilinearPoly::new(vec![
            Fr::from(1),
            Fr::from(2),
            Fr::from(3),
            Fr::from(4),
            Fr::from(5),
            Fr::from(6),
            Fr::from(7),
            Fr::from(8),
        ])
    }

    #[test]
    fn test_add_i_and_mul_i_mle() {
        let mut circuit = get_circuit();
        let input = get_input();
        let res = circuit.run_circuit(input.clone());

        let w_i = circuit.add_i_and_mul_i_mle(0);
        // let expected_layer = MultilinearPoly::new(to_field(vec![2, 7, 11, 56]));
        println!("res: {:?}", w_i);
        // assert_eq!(w_i, expected_layer);
    }

    #[test]
    fn test_w_i_polynomial() {
        let mut circuit = get_circuit();
        let input = get_input();
        let res = circuit.run_circuit(input.clone());

        let w_i = circuit.w_i_polynomial(1);
        let expected_layer: MultilinearPoly<Fr> =
            MultilinearPoly::new(to_field(vec![2, 7, 11, 56]));
        println!("w_i: {:?}", w_i);
        // assert_eq!(w_i, expected_layer);
    }
    #[test]
    fn test_w_i_polynomial_empty() {
        let mut circuit = get_circuit();

        let w_i = circuit.w_i_polynomial(1);
        let expected_layer = MultilinearPoly::new(vec![]);

        assert_eq!(w_i, expected_layer);
    }

    #[test]
    fn test_run_circuit() {
        let mut circuit = get_circuit();

        let inputs = get_input();

        let result = circuit.run_circuit(inputs);
        println!("Result: {:?}", result);
        assert_eq!(
            result,
            vec![
                get_input(),
                MultilinearPoly::new(vec![Fr::from(2), Fr::from(7), Fr::from(11), Fr::from(56)]),
                MultilinearPoly::new(vec![Fr::from(14), Fr::from(67)]),
                MultilinearPoly::new(vec![Fr::from(938)])
            ]
        );
    }

    #[test]
    fn test_partial_circuit_run() {
        let mut circuit = get_circuit();

        let inputs = get_input();

        let result = circuit.partial_circuit_run(inputs, 3);
        println!("Result: {:?}", result);
        assert_eq!(result, get_input());
    }
}

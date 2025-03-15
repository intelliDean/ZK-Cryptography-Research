use crate::gate::{Gate, Ops};
use ark_ff::PrimeField;
use polynomials::multilinear::multilinear::{Multilinear, MultilinearPoly};
use std::marker::PhantomData;
use crate::layer::Layer;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Circuit<F: PrimeField> {
    pub layers: Vec<Layer>,
    pub layer_witness: Vec<MultilinearPoly<F>>,
    _phantom: PhantomData<F>,
}

impl<F: PrimeField> Circuit<F> {
    pub(crate) fn new(layers: Vec<Layer>) -> Self {
        Circuit {
            layers,
            layer_witness: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn run_circuit(&mut self, inputs: MultilinearPoly<F>) -> Vec<MultilinearPoly<F>> {
        // initialize the witness layer with the inputs
        self.layer_witness.push(inputs);

        let mut circuit = self.layers.clone();
        circuit.reverse();
        println!("Layers to evaluate: {:?}", circuit);
        self.compute(&mut circuit); // Compute all layers

        self.layer_witness.reverse();
        self.layer_witness.clone() // Return all computed layer evaluations
    }
    //This returns a layer w_i by evaluating the circuit up to the layer index
    pub fn partial_circuit_run(
        &mut self,
        inputs: MultilinearPoly<F>,
        layer_id: usize,
    ) -> MultilinearPoly<F> {
        let length = self.layers.len();
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
                self.layers.reverse();
                self.layer_witness.push(inputs);
                let idx = length - 1 - layer_id;

                // Convert to Vec to avoid borrowing `self`
                let mut layers = self.layers[..=idx].to_vec();

                self.compute(&mut layers); // this will compute up to the layer_id

                self.layer_witness.last().unwrap().clone() // Return particular layer evaluation
            }
        }
    }

    fn compute(&mut self, circuit: &mut [Layer]) {
        for layer in circuit {
            // this will make use of the last layer
            // which was last pushed into the vec as the current layer
            if let Some(current_layer) = self.layer_witness.last() {
                // let mut witness = Vec::with_capacity(layer.len()); // Allocate space for new layer
                let mut witness = vec![F::from(0); layer.gates.len()];

                for mut gate in &layer.gates {
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
    //This returns a layer w_i when the circuit are already evaluated
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
        //this shifts 1 the number of variable times e.g
        // if num_var is 3, boolean_hypercube_combinations will be 8
        let boolean_hypercube_combinations = 1 << number_of_layer_variables; // 2 ^ number_of_layer_variables

        //this initializes a vec with 0 depending on the number of boolean_hypercube_combinations
        let mut add_i_values = vec![F::default(); boolean_hypercube_combinations];
        let mut mul_i_values = vec![F::default(); boolean_hypercube_combinations];

        //this gets the layer gates
        let layer = &self.layers[layer_index];

        for gate in layer.gates.iter() {
            //using the layer info, this gets which gate is valid
            println!("layer_idx: {}, a: {}, b: {}, c: {}", layer_index, gate.output, gate.left, gate.right);
            let position_index =
                combine_and_convert_to_decimal(layer_index, gate.output, gate.left, gate.right);

            //this turns the valid gate from 0 to 1
            match gate.ops {
                Ops::ADD => add_i_values[position_index] = F::one(),
                Ops::MUL => mul_i_values[position_index] = F::one(),
            }
        }

        //this will print out all the boolean hypercube combinations and their evaluation
        show_combinations(
            number_of_layer_variables,
            &mut add_i_values,
            &mut mul_i_values,
        );

        //this turns it into MultilinearPoly
        let add_i_polynomial = MultilinearPoly::new(add_i_values);
        let mul_i_polynomial = MultilinearPoly::new(mul_i_values);

        //returns the tuple of both add_1 and mul_i polynomial
        (add_i_polynomial, mul_i_polynomial)
    }
}

fn show_combinations<F: PrimeField>(
    number_of_layer_variables: usize,
    add_i_values: &[F],
    mul_i_values: &[F],
) {
    println!("bhc - add_i - mul_i");

    // Calculate the number of combinations based on the number of layer variables
    let boolean_hypercube_combinations = 1 << number_of_layer_variables; // 2^number_of_layer_variables

    for i in 0..boolean_hypercube_combinations {
        let binary_comb = format!("{:0width$b}", i, width = number_of_layer_variables);
        let add_eval = add_i_values[i];
        let mul_eval = mul_i_values[i];

        println!("{}:  -  {}  -  {}", binary_comb, add_eval, mul_eval);
    }
}

pub fn num_of_layer_variables(layer_index: usize) -> usize {
    // Most concise version
    if layer_index == 0 {
        3
    } else {
        3 * layer_index + 2 // 3 x 1 + 2 = 5 for layer 1, 3 x 2 + 2 = 8 for layer 2, etc
    }
}

pub fn combine_and_convert_to_decimal(
    layer_idx: usize,
    var_a: usize,
    var_b: usize,
    var_c: usize,
) -> usize {
    let a_bits = layer_idx;
    let bc_bits = layer_idx + 1;

    // Optional: Add bound checking
    assert!(var_a < (1 << a_bits), "var_a too large");
    assert!(var_b < (1 << bc_bits), "var_b too large");
    assert!(var_c < (1 << bc_bits), "var_c too large");

    let a_shifted = var_a << (2 * bc_bits);
    let b_shifted = var_b << bc_bits;

    a_shifted | b_shifted | var_c
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use polynomials::multilinear::multilinear::to_field;

    fn get_layer() -> Layer {
        Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)])
    }
    pub fn get_circuit() -> Circuit<Fr> {
        let layer0 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL)]);

        let layer1 = Layer::new(vec![Gate::new(0, 0, 1, Ops::MUL), Gate::new(1, 2, 3, Ops::ADD)]);

        let layer2 = Layer::new( vec![
            Gate::new(0, 0, 1, Ops::MUL),
            Gate::new(1, 2, 3, Ops::ADD),
            Gate::new(2, 4, 5, Ops::ADD),
            Gate::new(3, 6, 7, Ops::MUL),
        ]);

        let circuit = vec![layer0, layer1, layer2];


        println!("Layers: {:?}", &circuit);

        Circuit::new(circuit)
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

        let w_i = circuit.add_i_and_mul_i_mle(1);
        // let expected_layer = MultilinearPoly::new(to_field(vec![2, 7, 11, 56]));
        println!("res: {:?}", w_i);
        // assert_eq!(w_i, expected_layer);
    }

    #[test]
    fn test_num_of_layer_variables() {
        // Assert Equal
        assert_eq!(num_of_layer_variables(0), 3);
        assert_eq!(num_of_layer_variables(1), 5);
        assert_eq!(num_of_layer_variables(2), 8);
        assert_eq!(num_of_layer_variables(3), 11);
        assert_eq!(num_of_layer_variables(4), 14);

        // Assert Not Equal
        assert_ne!(num_of_layer_variables(2), 7);
        assert_ne!(num_of_layer_variables(3), 9);
    }

    #[test]
    fn test_combine_and_convert_to_decimal() {

        let res = combine_and_convert_to_decimal(1, 0, 0, 1);
        println!("res: {:?}", res);
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
                MultilinearPoly::new(vec![Fr::from(938)]),
                MultilinearPoly::new(vec![Fr::from(14), Fr::from(67)]),
                MultilinearPoly::new(vec![Fr::from(2), Fr::from(7), Fr::from(11), Fr::from(56)]),
                get_input(),
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

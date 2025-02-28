use ark_ff::PrimeField;
use crate::gate::Gate;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layer {
    pub gates: Vec<Gate>,
}

impl Layer {
    pub(crate) fn new(gates: Vec<Gate>) -> Self {
        Layer {
            gates
        }
    }
    pub(crate) fn update_layer_gate(&mut self, extra_gate: Gate) {
        self.gates.push(extra_gate);
    }
}

#[cfg(test)]
mod tests {
    use ark_bn254::Fr;
    use crate::gate::Ops;
    use super::*;

    #[test]
    fn test_new() {
        let layer = vec![
            Gate::new(0, 0, 1, Ops::ADD),
            Gate::new(1, 2, 3, Ops::MUL),
            Gate::new(2, 4, 5, Ops::ADD)
        ];
        let res = Layer::new(layer);
        assert_eq!(res.gates[0].left, 0);
    }
    #[test]
    fn test_update_layer_gate() {
        let gates = vec![
            Gate::new(0, 0, 1, Ops::ADD),
            Gate::new(1, 2, 3, Ops::MUL),
            Gate::new(2, 4, 5, Ops::ADD)
        ];
        let mut res = Layer::new(gates);
        let extra_gate = Gate::new(1, 2, 3, Ops::MUL);
        res.update_layer_gate(extra_gate);
        assert_eq!(res.gates.len(), 4);
    }
}
use ark_ff::PrimeField;
use crate::gate::Gate;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layer {
    pub layers: Vec<Vec<Gate>>,
}

impl Layer {
    pub(crate) fn new(first_layer: Vec<Gate>) -> Self {
        Layer {
            layers: vec![first_layer],
        }
    }
    pub(crate) fn update_layer_gate(&mut self, layer_gates: Vec<Gate>) {
        self.layers.push(layer_gates);
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
        assert_eq!(res.layers[0][0].left, 0);
    }
    #[test]
    fn test_update_layer_gate() {
        let layer0 = vec![
            Gate::new(0, 0, 1, Ops::ADD),
            Gate::new(1, 2, 3, Ops::MUL),
            Gate::new(2, 4, 5, Ops::ADD)
        ];
        let mut res = Layer::new(layer0);
        let layer1 = vec![
            Gate::new(0, 0, 1, Ops::ADD),
            Gate::new(1, 2, 3, Ops::MUL),
            Gate::new(2, 4, 5, Ops::ADD)
        ];
        res.update_layer_gate(layer1);
        assert_eq!(res.layers.len(), 2);
    }
}
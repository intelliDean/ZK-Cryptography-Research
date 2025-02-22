use ark_ff::PrimeField;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Ops {
    ADD,
    MUL,
}

impl Ops {
    pub fn operation<F: PrimeField>(&self, left: &F, right: &F) -> F {
        match self {
            Ops::ADD => *left + *right,
            Ops::MUL => *left * *right,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Gate {
    // gate is basically the index position of the real values
    pub(crate) output: usize,   // the evaluation between left and right due to the ops
    pub(crate) left: usize, // i used usize for the left and right index
    pub(crate) right: usize,
    pub(crate) ops: Ops, // an enum of Operations
}

impl Gate {
    #[inline(always)]
    pub(crate) fn new(output: usize, left: usize, right: usize, ops: Ops) -> Self {
        Self {
            output,
            left,
            right,
            ops
        }
    }
}

#[cfg(test)]
mod tests {
    use ark_bn254::Fr;
    use super::*;
    #[test]
    fn test_gate() {
        let output = 15;
        let left = 3;
        let right = 5;
        let op = Ops::MUL;
        let gate = Gate::new(output, left, right, op);

        assert_eq!(gate.ops, Ops::MUL);
        assert_eq!(gate.output, output);
    }
}
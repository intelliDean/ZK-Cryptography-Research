use super::utilities;
pub struct Data {
    pub(crate) degree: usize,
    pub(crate) co_ex: Vec<(usize, usize)>,
}

impl Data {
    pub(crate) fn degree(&self) -> usize {
        self.degree
    }

    pub(crate) fn evaluate(&self, x: usize) -> usize {
        let co_ex = &self.co_ex;

        let mut result = 0;

        for co in co_ex.iter() {
            result += x.pow(co.1 as u32) * co.0;
        }
        result
    }

    pub(crate) fn interpolate(&self, p: usize) -> Option<utilities::Points> {
        let mut new_vec: Vec<usize> = Vec::new();

        for point in self.co_ex.iter() {
            if point.0 == p {
                continue;
            }
            new_vec.push(point.0);
        }

        for point in self.co_ex.iter() {
            let result = point.1 as f64 * utilities::get_lx(&new_vec, point.0, p);

            if result > 0.0 {
                return Some(utilities::Points {
                    coefficient: point.0,
                    degree: point.1,
                });
            }
        }
        None
    }
}
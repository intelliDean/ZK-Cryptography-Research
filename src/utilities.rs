use crate::lagrand::Data;

#[derive(Debug)]
pub(crate) struct Points {
    pub(crate) coefficient: usize,
    pub(crate) degree: usize,
}

pub(crate) fn create_data(data: Vec<(usize, usize)>) -> Data {
    let mut deg: usize = 0;

    for i in 0..data.len() {
        if data[i].1 > deg {
            deg = data[i].1;
        }
    }
    Data {
        degree: deg,
        co_ex: data,
    }
}

pub fn get_lx(points: &Vec<usize>, n: usize, p: usize) -> f64 {
    let mut result: f64 = 1.0;

    for i in 0..points.len() {
        let num = points[i];

        result *= (n as f64 - num as f64) / (p as f64 - num as f64);
    }
    result
}

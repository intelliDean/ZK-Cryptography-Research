mod lagrand;
mod utilities;

fn main() {
    let mut data: Vec<(usize, usize)> = Vec::new();

    data.push((2, 4));
    data.push((4, 8));
    data.push((5, 4));
    data.push((3, 12));
    data.push((8, 1));

    let created_data: lagrand::Data = utilities::create_data(data);

    let degree = created_data.degree();

    let evaluate = created_data.evaluate(4);

    let points = created_data.interpolate(4);

    println!("The degree is {}.", degree);

    println!("The evaluation is {}.", evaluate);

    if let Some(point) = points {
        println!("Interpolation points: {:?}", point);
    }
}

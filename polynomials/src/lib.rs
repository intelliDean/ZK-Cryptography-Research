
pub mod multilinear;
pub mod univariate;
mod product;
pub mod sum;


pub fn bitwise() {
    let num = 56;
    let num_bin = format!("{:08b}", num);

    let move_num = num ^ 3; //56 + 3
    let move_num_bin = format!("{:08b}", move_num);
    println!(
        "
        Num: {}, {}
  Moved Num: {}, {}
    ",
        num, num_bin, move_num, move_num_bin
    );

    binary_multiplication(5, 2);

    binary_division(20, 2);
}

fn binary_multiplication(num: u8, shift_by: u8) {
    println!("Multiplication");
    let num_bin = format!("{:08b}", num);

    // for every movement in binary, it's by 2
    // left shift is a multiplication
    // right shift is a division
    // eg 5 << 2 == 5 x 2 x 2 (2 shifts)

    let move_num: u8 = num << shift_by;

    let move_num_bin = format!("{:08b}", move_num);
    println!(
        "
        Num: {}, {}
  Moved Num: {}, {}
    ",
        num, num_bin, move_num, move_num_bin
    );
}

fn binary_division(num: u8, shift_by: u8) {
    println!("Division");
    let num_bin = format!("{:08b}", num);

    // for every movement in binary, it's by 2
    // left shift is a multiplication
    // right shift is a division
    // eg 20 << 2 == 20 / 2 / 2 (2 shifts)

    let move_num: u8 = num >> shift_by;

    let move_num_bin = format!("{:08b}", move_num);
    println!(
        "
        Num: {}, {}
  Moved Num: {}, {}
    ",
        num, num_bin, move_num, move_num_bin
    );
}




pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        bitwise();
    }
}

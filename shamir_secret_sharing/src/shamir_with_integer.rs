use rand::Rng;

#[derive(Debug, Clone)]
struct DeanField {
    num: i32,
    den: i32,
}

impl DeanField {
    //to create a new DeanField
    fn new(num: i32, den: i32) -> DeanField {
        let mut field = DeanField { num, den };
        field.reduce();
        field
    }

    //to get the Greatest Common Divisor (GCD)
    fn reduce(&mut self) {
        let gcd = gcd(self.num.abs(), self.den.abs());
        self.num /= gcd;
        self.den /= gcd;
    }

    //to multiply to two DeanFields
    fn multiply(&self, other: &DeanField) -> DeanField {
        let mut result = DeanField::new(self.num * other.num, self.den * other.den);
        result.reduce();
        result
    }

    // to add two DeanFields
    fn add(&self, other: &DeanField) -> DeanField {
        let mut result = DeanField::new(
            self.num * other.den + self.den * other.num,
            self.den * other.den,
        );
        result.reduce();
        result
    }
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b) //recursion
    }
}

fn calculate_y(x: i32, poly: &[i32]) -> i32 {
    let mut y = 0;
    let mut temp = 1;

    for &coeff in poly {
        y += coeff * temp;
        temp *= x;
    }
    y
}
fn secret_sharing(secret: i32, num_of_shares: usize, thresholds: usize) -> Vec<(i32, i32)> {
    let mut rng = rand::rng();
    let mut poly = vec![secret];

    // Generate k-1 random coefficients
    for _ in 1..thresholds {
        let mut p = 0;
        while p == 0 {
            //use the threshold to randomly generate a polynomial of threshold - 1
            p = rng.random_range(1..997); // random number to generate a polynomial
        }
        poly.push(p);
    }

    let mut results = Vec::new();

    for x in 1..=num_of_shares {
        let x = x as i32; //sequential number of x
        let y = calculate_y(x, &poly); // evaluate the polynomial at x value to get the y value
        results.push((x, y)); // the x and y become the shares to the secret custodians
    }
    results
}

fn regenerate_secret(x: &[i32], y: &[i32], thresholds: usize) -> u32 {
    let mut ans = DeanField::new(0, 1);

    for i in 0..thresholds {
        // the threshold is needed to regenerate the polynomial
        let mut l = DeanField::new(y[i], 1);

        for j in 0..thresholds {
            if i != j {
                let temp = DeanField::new(-x[j], x[i] - x[j]);
                l = l.multiply(&temp);
            }
        }
        ans = ans.add(&l);
    }

    ans.num.abs_diff(0)
}

pub(crate) fn operation(secret: i32, num_of_shares: usize, thresholds: usize) -> u32 {
    println!("Sharing secret: {}", secret);

    // Generate shares
    let points = secret_sharing(secret, num_of_shares, thresholds);

    println!("Secret is divided into {} parts:", num_of_shares);
    for (x, y) in &points {
        println!("x: {}, y: {}", x, y);
    }

    println!("We can generate secret from any {} parts", thresholds);

    // For demonstration, use first k points to reconstruct
    // In real usage, thresholds >= num_of_shares
    let x: Vec<i32> = points[0..thresholds].iter().map(|&(x, _)| x).collect(); //takes the x out of the point
    let y: Vec<i32> = points[0..thresholds].iter().map(|&(_, y)| y).collect(); //takes the y out of the point

    regenerate_secret(&x, &y, thresholds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_dean_field_creation() {
        let dean_field = DeanField::new(1, 2);
        assert_eq!(dean_field.num, 1);
    }

    #[test]
    fn test_secret_sharing() {
        let secret = 2390;
        let num_of_shares = 7;
        let threshold = 3;
        let points = secret_sharing(secret, num_of_shares, threshold);
        assert_eq!(points.len(), 7);
    }

    #[test]
    fn test_regenerate_secret() {
        let secret = 2390;
        let num_of_shares = 7;
        let threshold = 3;
        let points = secret_sharing(secret, num_of_shares, threshold);

        let x = [1, 2, 3, 4, 5, 6, 7];
        let y = [2641, 3068, 3671, 4450, 5405, 6536, 7843];
        let regenerated_secret = regenerate_secret(&x, &y, threshold);
        assert_eq!(regenerated_secret, 2390);
    }

    #[test]
    fn test_operation() {
        let secret = 2390;
        let num_of_shares = 7;
        let threshold = 3;
        let points = operation(secret, num_of_shares, threshold);
        assert_eq!(points, 2390);
    }
}

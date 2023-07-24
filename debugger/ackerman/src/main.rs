use num::{BigUint, FromPrimitive, One, Zero};

fn main() {
    ackerman(4, 4);
}

pub fn ackerman(m: usize, n: usize) -> BigUint {
    ackerman_rec(m, BigUint::from_usize(n).unwrap())
}

fn ackerman_rec(m: usize, n: BigUint) -> BigUint {
    let one: BigUint = One::one();
    let zero: BigUint = Zero::zero();
    if m == 0 {
        n + one
    } else if n == zero {
        ackerman_rec(m - 1, one)
    } else {
        ackerman_rec(m - 1, ackerman_rec(m, n - one))
    }
}

#[cfg(test)]
mod tests {
    use crate::ackerman;

    #[test]
    fn test_ackerman() {
        assert_eq!(ackerman(1, 1).to_string(), "3");
    }
}

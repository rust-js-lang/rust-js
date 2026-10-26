//! The library trait's method given as a value, `all(Zero::is_zero)`, as
//! num-bigint's conversion asks it: its impl's, as a call of it is.
use dep::Zero;

fn rest_zero(bytes: &[u8]) -> bool {
    bytes.iter().skip(1).all(Zero::is_zero)
}

fn any_zero(xs: &[u32]) -> bool {
    xs.iter().any(u32::is_zero)
}

pub fn main() {
    println!(
        "{} {} {}",
        rest_zero(&[1, 0, 0]),
        rest_zero(&[1, 0, 2]),
        any_zero(&[3, 0])
    );
}

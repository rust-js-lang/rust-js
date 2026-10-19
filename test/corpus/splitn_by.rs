// `s.splitn(n, p)` of a set of `char`s or a closure, as rust_decimal splits
// an exponent: its first pieces, and what's left whole.

fn main() {
    let mut parts = "1.5e-3".splitn(2, ['e', 'E']);
    println!("{:?} {:?} {:?}", parts.next(), parts.next(), parts.next());
    let pieces: Vec<&str> = "a1b2c3d".splitn(3, |c: char| c.is_ascii_digit()).collect();
    println!("{pieces:?}");
    println!("{:?}", "xyz".splitn(0, ['y']).collect::<Vec<_>>());
    println!("{:?}", "x-y_z".splitn(5, ['-', '_']).collect::<Vec<_>>());
}

// A `Peekable` of a range knows how many it has left, as the range does:
// `size_hint()` exact, and `len()`, before a peek and after one.
fn main() {
    let mut digits = (1..4).peekable();
    println!("{:?} {}", digits.size_hint(), digits.len());
    digits.next();
    println!("{:?} {}", digits.size_hint(), digits.len());
    digits.peek();
    println!("{:?} {}", digits.size_hint(), digits.len());
}

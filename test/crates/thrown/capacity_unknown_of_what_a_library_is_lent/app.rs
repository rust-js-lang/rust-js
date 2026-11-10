//! A capacity read of what its library grew of what it was lent.
pub fn main() {
    let mut lent = vec![9];
    dep::fill(&mut lent);
    println!("{}", lent.len());
    println!("{}", lent.capacity());
}

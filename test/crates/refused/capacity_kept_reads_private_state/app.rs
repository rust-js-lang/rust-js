//! A capacity read of what its library grew.
pub fn main() {
    let v = dep::made();
    println!("{} {}", v.len(), v.capacity());
}

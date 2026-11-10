//! A capacity read of what its library grew and gave back.
pub fn main() {
    let made = dep::made();
    println!("{}", made.items.len());
    println!("{}", made.items.capacity());
}

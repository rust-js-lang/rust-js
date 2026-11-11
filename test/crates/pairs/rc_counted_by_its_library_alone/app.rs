//! A consumer that clones and drops an `Rc` its library counts, reading no
//! count itself (ADR 0852).
pub fn main() {
    let a = dep::make();
    let b = a.clone();
    println!("{} {}", dep::count(&a), b.n);
    drop(b);
    println!("{}", dep::count(&a));
}

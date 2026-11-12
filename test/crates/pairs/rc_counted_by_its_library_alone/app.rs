//! A consumer that clones and drops an `Rc` its library counts, reading no
//! count itself, and one it counts none of (ADR 0852).
pub fn main() {
    let a = dep::make();
    let b = a.clone();
    println!("{} {}", dep::count(&a), b.n);
    drop(b);
    println!("{}", dep::count(&a));
    let n = dep::note();
    let m = n.clone();
    println!("{} {}", n.text, m.text);
}

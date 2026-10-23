// `slice::from_ref(&x)` and `array::from_ref(&x)`: `[x]`, a slice of the one
// item it shares, a cell too, which a write through it changes.
use std::cell::Cell;
fn sum(s: &[i32]) -> i32 {
    s.iter().sum()
}
fn names(s: &[String]) -> String {
    s.join(",")
}
fn main() {
    let x = 5;
    println!("{}", sum(std::slice::from_ref(&x)));
    let t = String::from("a");
    println!("{} {}", names(std::slice::from_ref(&t)), t);
    let c = Cell::new(1);
    let one = std::array::from_ref(&c);
    one[0].set(9);
    println!("{} {}", c.get(), one.len());
    let v = vec![3, 4];
    let first = std::slice::from_ref(&v[0]);
    println!("{:?}", first);
}

//! A consumer that picks a struct, whose copy is another object (#1233).
use dep::Pick;

#[derive(Clone, Copy, Debug)]
struct Point(u8, u8);

pub fn main() {
    let p = Point(1, 2);
    let mut q = dep::Same.pick(p);
    q.0 = 9;
    println!("{p:?} {q:?} {}", dep::own());
}

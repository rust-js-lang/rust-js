// `Pin::new(&mut b)` gives the `&mut` to generic code, `Pin<Ptr>`, whose
// `set` writes the struct whole in place through it: a `&mut` given where a
// function declares a bare type parameter counts as one generic code writes,
// so `let mut b = a` copies `a`, which keeps its fields (#1665).
use std::pin::Pin;

#[derive(Clone, Copy, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

fn main() {
    let a = Pair { a: 1, b: 2 };
    let mut b = a;
    let mut p = Pin::new(&mut b);
    p.set(Pair { a: 5, b: 6 });
    println!("{a:?} {b:?}");
}

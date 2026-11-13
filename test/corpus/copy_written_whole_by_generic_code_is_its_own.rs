// A copy of a `Copy` struct, `let mut b = a`, written whole by generic code
// through a `&mut T`, `*r = T::default()`: an in-place cell of `b` given to
// it, so `b` is a copy of `a`, not `a` itself, as the type is one generic
// code writes in place (#1665).
#[derive(Clone, Copy, Default, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

fn reset<T: Default>(r: &mut T) {
    *r = T::default()
}

fn main() {
    let a = Pair { a: 1, b: 2 };
    let mut b = a;
    reset(&mut b);
    println!("{a:?} {b:?}");
}

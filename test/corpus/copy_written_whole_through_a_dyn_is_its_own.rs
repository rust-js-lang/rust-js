//@ library-differs
// A copy of a `Copy` struct, `let mut b = a`, given as a `&mut dyn GetMut<T>`
// whose blanket impl hands back `self`, then written whole, `*r =
// T::default()`: a `&mut` made a `&mut dyn` erases its type as generic code
// does, and its pair writes the struct in place, so `b` is a copy of `a`, not
// `a` itself (#1665, found by the metamorphic net's `dyn-ref` rewrite).
#[derive(Clone, Copy, Default, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

trait GetMut<T: ?Sized> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T: ?Sized> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn reset<T: Default>(r: &mut dyn GetMut<T>) {
    let r: &mut T = r.get_mut();
    *r = T::default()
}

fn main() {
    let a = Pair { a: 1, b: 2 };
    let mut b = a;
    reset(&mut b);
    println!("{a:?} {b:?}");
}

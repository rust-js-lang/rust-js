// Generic `mem::swap` and `mem::replace` write a struct in place through
// generic code's box, so `let mut b = a` and `let mut e = d` copy, and the
// originals keep their fields (#1665).
#[derive(Clone, Copy, Default, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

fn swap_them<T>(x: &mut T, y: &mut T) {
    std::mem::swap(x, y)
}

fn put<T>(r: &mut T, v: T) -> T {
    std::mem::replace(r, v)
}

fn main() {
    let a = Pair { a: 1, b: 2 };
    let mut b = a;
    let mut c = Pair { a: 7, b: 8 };
    swap_them(&mut b, &mut c);
    println!("{a:?} {b:?} {c:?}");
    let d = Pair { a: 3, b: 4 };
    let mut e = d;
    let old = put(&mut e, Pair { a: 5, b: 6 });
    println!("{d:?} {e:?} {old:?}");
}

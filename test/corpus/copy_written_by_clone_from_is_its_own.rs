// `clone_from` through a `&mut` parameter writes the struct in place, `$take`,
// so `let mut b = a` copies `a` and `a` keeps its fields (#1665).
#[derive(Clone, Copy, Default, Debug)]
struct Pair {
    a: i32,
    b: i32,
}

fn copy_into(dst: &mut Pair, src: &Pair) {
    dst.clone_from(src)
}

fn main() {
    let a = Pair { a: 1, b: 2 };
    let mut b = a;
    copy_into(&mut b, &Pair { a: 9, b: 9 });
    println!("{a:?} {b:?}");
}

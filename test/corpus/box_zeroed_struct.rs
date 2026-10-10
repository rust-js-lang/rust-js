//@ compile-fail: a zeroed `Point`, which may be no value of it
// A zeroed struct is all zero bytes, which JS has no value of where its
// fields aren't numbers: refused (ADR 0332).

#[derive(Debug)]
struct Point {
    x: i32,
    name: String,
}

fn main() {
    let point = unsafe { Box::<Point>::new_zeroed().assume_init() };
    println!("{point:?}");
}

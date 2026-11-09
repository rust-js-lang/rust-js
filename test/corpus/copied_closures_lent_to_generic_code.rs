//@ run-pass
// A closure that generic code makes a `&dyn Fn` of, as a type parameter's
// value, whose address it reads: each copy of any capturing `Copy` closure
// is a function of its own, as generic code may be given any (#1090, the
// user's decision).
use std::ptr;

fn eq_of<F: Fn() -> i32>(a: &F, b: &F) -> bool {
    ptr::eq(a as &dyn Fn() -> i32, b as &dyn Fn() -> i32)
}

fn given<F: Fn() -> i32 + Copy>(f: F, original: &F) -> bool {
    eq_of(&f, original)
}

fn main() {
    let m = 7;
    let c3 = move || m * 2;
    let c4 = c3;
    println!("{} {} {}", eq_of(&c3, &c4), eq_of(&c3, &c3), c3() + c4());
    println!("{}", given(c3, &c3));
}

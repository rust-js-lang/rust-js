//@ run-pass
// A copy of a `Copy` closure that captures something is a place of its
// own, so a pointer to it isn't a pointer to the closure it was copied
// from, where the crate reads a `dyn Fn`'s address: each copy is a
// function of its own, `c1.bind()` (#1090, the user's decision). One that
// captures nothing has no size, and its address is the function's.
use std::ptr;

struct Holder<F> {
    f: F,
}

fn same(a: &dyn Fn() -> i32, b: &dyn Fn() -> i32) -> bool {
    ptr::eq(a, b)
}

fn main() {
    let n = 3;
    let c1 = move || n + 1;
    let c2 = c1;
    println!("{} {} {}", same(&c1, &c2), same(&c1, &c1), c1() + c2());
    // Copied into a struct's field.
    let held = Holder { f: c1 };
    println!("{} {}", same(&held.f, &c1), (held.f)());
    // Copies in an array, each its own.
    let items = [c1, c1];
    println!("{} {}", same(&items[0], &items[1]), same(&items[0], &items[0]));
    // `[c1; 3]`'s items, a copy of the array, and a tuple's copy.
    let rep = [c1; 3];
    let again = rep;
    println!("{} {}", same(&rep[0], &rep[1]), same(&rep[2], &again[2]));
    let pair = (c1, 5);
    let other = pair;
    println!("{} {}", same(&pair.0, &other.0), (other.0)() + other.1);
    // A copy changed by nothing still runs as the original.
    let total = items.iter().map(|f| f()).sum::<i32>();
    println!("{total}");
}

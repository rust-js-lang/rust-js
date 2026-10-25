// A `const { RefCell::new(..) }` thread-local of a struct is the struct, as
// one made at run time is (ADR 0863): made by its initializer, which knows
// it's the object, and borrowed in place.

use std::cell::RefCell;

#[derive(Debug)]
struct Pair {
    a: u32,
    b: u32,
}

thread_local! {
    static TOTALS: RefCell<Pair> = const { RefCell::new(Pair { a: 0, b: 0 }) };
}

fn add(n: u32) {
    TOTALS.with_borrow_mut(|totals| {
        totals.a += n;
        totals.b += 1;
    });
}

fn main() {
    add(2);
    add(3);
    TOTALS.with_borrow(|totals| println!("{} {}", totals.a, totals.b));
    TOTALS.with(|totals| println!("{:?}", totals.borrow()));
}

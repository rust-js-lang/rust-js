// A function compared by `std::ptr::eq` with a `&dyn Fn` is the JS
// function it is, `handler === noop`: a function's, given by reference,
// `&noop`, and a closure's that captures nothing, as one that does is,
// each a value of no size whose pointer is no address but the function
// (ADR 0562, amended). As React compares `node.onclick === noop`.

use std::cell::Cell;

fn noop(_value: &u32) {}

fn is_noop(handler: &dyn Fn(&u32)) -> bool {
    std::ptr::eq(handler, &noop)
}

thread_local! {
    static HELD: Cell<Option<&'static dyn Fn(&u32)>> = const { Cell::new(None) };
}

fn holds_noop() -> bool {
    HELD.get().is_some_and(|held| std::ptr::eq(held, &noop))
}

fn main() {
    let loud = |value: &u32| println!("loud {value}");
    let quiet = |_: &u32| {};
    let total = Cell::new(0);
    let counts = |value: &u32| total.set(total.get() + value);
    let handlers: [&dyn Fn(&u32); 4] = [&noop, &loud, &quiet, &counts];
    for handler in handlers {
        handler(&1);
        println!(
            "{} {} {} {}",
            is_noop(handler),
            std::ptr::eq(handler, &loud),
            std::ptr::eq(handler, &quiet),
            std::ptr::eq(handler, &counts),
        );
        let raw: *const dyn Fn(&u32) = &noop;
        println!("{}", std::ptr::eq(handler, raw));
    }
    println!("{} {}", total.get(), holds_noop());
    HELD.set(Some(&noop));
    println!("{}", holds_noop());
    HELD.set(Some(&|_| {}));
    println!("{}", holds_noop());
}

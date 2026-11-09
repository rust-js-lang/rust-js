//@ run-pass
// A closure that captures another by reference reads the one it captured,
// though a local of its own body holds a reference to it: only its own
// environment, `_1`, is read through its captures, never another local of
// a closure's type (#1077). So `&loud` in a closure compares equal to the
// handler made of it, and a closure called through its reference there
// runs.
use std::cell::Cell;

fn main() {
    let loud = |value: &u32| println!("loud {value}");
    let handler: &dyn Fn(&u32) = &loud;
    handler(&1);
    let check = || std::ptr::eq(handler, &loud);
    println!("{}", check());

    let quiet = |_: &u32| {};
    let handlers: [&dyn Fn(&u32); 2] = [&loud, &quiet];
    handlers.into_iter().for_each(|handler| {
        handler(&2);
        println!("{} {}", std::ptr::eq(handler, &loud), std::ptr::eq(handler, &quiet));
    });

    let total = Cell::new(0);
    let counts = |value: u32| total.set(total.get() + value);
    let twice = |value: u32| {
        let counted = &counts;
        counted(value);
        (*counted)(value);
    };
    twice(3);
    println!("{}", total.get());
}

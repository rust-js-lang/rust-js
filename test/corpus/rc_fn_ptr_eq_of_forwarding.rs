// `Rc::ptr_eq` of `Rc<dyn Fn()>`s each made of a closure that only calls a
// function of the crate's, uncounted (rd313, ADR 0959): where the crate
// compares an `Rc<dyn Fn>` it doesn't count, each such closure is a
// function of its own, so the two are two, as they're two `Rc`s to Rust.
// Where it compares none, the closure is the function it calls.
use std::rc::Rc;

fn ping() {
    println!("ping");
}

fn main() {
    let first: Rc<dyn Fn()> = Rc::new(|| ping());
    let second: Rc<dyn Fn()> = Rc::new(|| ping());
    first();
    second();
    let again = Rc::clone(&first);
    println!("{} {}", Rc::ptr_eq(&first, &second), Rc::ptr_eq(&first, &again));
}

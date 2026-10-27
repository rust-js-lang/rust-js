// `Rc::ptr_eq` of `Rc<dyn Fn()>`s made of a type parameter's function
// (ADR 0959): a function item given twice is one JS function, which two
// `Rc`s are made of, so they're counted, `{ value, strong, weak }`,
// compared by their boxes.
use std::rc::Rc;

fn hello() -> u32 {
    1
}

fn shared<F: Fn() -> u32 + 'static>(f: F) -> Rc<dyn Fn() -> u32> {
    Rc::new(f)
}

fn main() {
    let a = shared(hello);
    let b = shared(hello);
    println!("{} {} {}", Rc::ptr_eq(&a, &b), Rc::ptr_eq(&a, &a.clone()), a() + b());
}

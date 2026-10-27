// `Rc::ptr_eq` of `Rc<dyn Fn()>`s made of a function item (ADR 0959): the
// JS function is the item's, which another `Rc` may be made of too, so its
// `===` would be `true` where Rust's two `Rc`s are apart. They're counted,
// `{ value, strong, weak }`, compared by their boxes.
use std::rc::Rc;

fn hello() -> u32 {
    1
}

fn main() {
    let a: Rc<dyn Fn() -> u32> = Rc::new(hello);
    let b: Rc<dyn Fn() -> u32> = Rc::new(hello);
    println!("{} {} {}", Rc::ptr_eq(&a, &b), Rc::ptr_eq(&a, &a.clone()), a() + b());
}

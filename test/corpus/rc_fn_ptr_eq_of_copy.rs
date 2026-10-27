// `Rc::ptr_eq` of `Rc<dyn Fn(u32)>`s made of a closure copied, given
// `Rc::new` by its variable (ADR 0959): one JS function, which two `Rc`s
// are made of, so they're counted, `{ value, strong, weak }`, compared by
// their boxes.
use std::rc::Rc;

fn main() {
    let add = |x: u32| x + 1;
    let a: Rc<dyn Fn(u32) -> u32> = Rc::new(add);
    let b: Rc<dyn Fn(u32) -> u32> = Rc::new(add);
    println!("{} {} {}", Rc::ptr_eq(&a, &b), Rc::ptr_eq(&b, &b.clone()), a(1) + b(2));
}

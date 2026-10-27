//! Two `Rc`s of one function item, which the library compares: apart, as
//! two `Rc::new`s are (ADR 0959).
use std::rc::Rc;

fn log(n: u32) {
    println!("{n}");
}

pub fn main() {
    let a: Rc<dyn Fn(u32)> = Rc::new(log);
    let b: Rc<dyn Fn(u32)> = Rc::new(log);
    let c = dep::hook::<u32>();
    println!(
        "{} {} {}",
        dep::same(&a, &b),
        dep::same(&a, &a.clone()),
        dep::same(&c, &dep::hook())
    );
    a(1);
    c(2);
}

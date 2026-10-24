//! A crate given a library's uncounted `Rc`s, which it counts none of
//! either, and that gives the library's generic code its own (ADR 0844).
use std::rc::{Rc, Weak};

pub fn main() {
    let context = dep::create_context(1);
    let again = dep::bump(&context);
    println!("{} {}", context.value.get(), again.value.get());
    let consumer = context.consumer.take().expect("a consumer");
    context.consumer.set(Some(Rc::clone(&consumer)));
    let provider = context.provider.take();
    println!("{}", Weak::ptr_eq(&provider, &consumer.context));
    println!("{}", Rc::ptr_eq(&context, &again));
    let cell = dep::shared_cell(4.5);
    let alias = Rc::clone(&cell);
    alias.set(5.5);
    println!("{}", cell.get());
    let (a, b) = dep::pair(Rc::new(String::from("shared")));
    println!("{} {}", a, b);
}

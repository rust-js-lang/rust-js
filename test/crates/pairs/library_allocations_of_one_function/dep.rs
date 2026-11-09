//! Allocations made a `dyn Fn` of a function another may hold too, by a
//! library that reads no `dyn Fn`'s address itself (#1053): each a function
//! of its own to a consumer that does, as Rust's two allocations are apart.
use std::rc::Rc;

pub fn boxed(f: fn(u32) -> u32) -> Box<dyn Fn(u32) -> u32> {
    Box::new(f)
}

pub fn boxed_generic<F: Fn(u32) -> u32 + 'static>(f: F) -> Box<dyn Fn(u32) -> u32> {
    Box::new(f)
}

pub fn shared(f: fn(u32) -> u32) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(f)
}

pub fn double(x: u32) -> u32 {
    x * 2
}

/// Two boxes of one closure, cloned.
pub fn cloned(label: String) -> (Box<dyn Fn(u32) -> u32>, Box<dyn Fn(u32) -> u32>) {
    let measure = move |x: u32| x + label.len() as u32;
    (Box::new(measure.clone()), Box::new(measure))
}

/// Two `Rc`s of one `Copy` closure.
pub fn copied(k: u32) -> (Rc<dyn Fn(u32) -> u32>, Rc<dyn Fn(u32) -> u32>) {
    let add = move |x: u32| x + k;
    (Rc::new(add), Rc::new(add))
}

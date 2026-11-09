//! A `dyn Trait` whose method makes two `Rc<dyn Fn>`s of one closure, its
//! impl private, handed as a `Box<dyn Trait>` (#1055): its consumer calls
//! the trait's own method, which resolves to no impl it sees.
use std::rc::Rc;

pub trait Pair {
    fn pair(&self) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>);
}

struct Twice(u32);

impl Pair for Twice {
    fn pair(&self) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
        let n = self.0;
        let read = move || n;
        (Rc::new(read), Rc::new(read))
    }
}

pub fn made(n: u32) -> Box<dyn Pair> {
    Box::new(Twice(n))
}

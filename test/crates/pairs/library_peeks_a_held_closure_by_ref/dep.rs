//! A trait whose impl for a struct holding a function calls it by `&`,
//! and generic code given the struct by reference (#2609).
pub trait Peek {
    fn peek(&self) -> u32;
}

pub struct Held<F>(pub F);

impl<F: Fn() -> u32> Peek for Held<F> {
    fn peek(&self) -> u32 {
        (self.0)()
    }
}

pub fn peek_twice<R: Peek>(r: &R) -> u32 {
    r.peek() + r.peek()
}

pub trait Consume {
    fn consume(self) -> u32;
}

impl<P: Peek> Consume for &P {
    fn consume(self) -> u32 {
        self.peek()
    }
}

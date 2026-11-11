//! A library that counts the `Rc`s of a type it reads a count of, made and
//! read by its own functions, none generic, which its consumer calls
//! through its JS: the consumer, which reads no count and compiles none of
//! them, counts every `Rc` of it it clones and drops, as its library hands
//! it counted (ADR 0852), or the library reads one count fewer (#1332).
use std::rc::Rc;

pub struct Data {
    pub n: u32,
}

pub fn make() -> Rc<Data> {
    Rc::new(Data { n: 1 })
}

pub fn count(data: &Rc<Data>) -> usize {
    Rc::strong_count(data)
}

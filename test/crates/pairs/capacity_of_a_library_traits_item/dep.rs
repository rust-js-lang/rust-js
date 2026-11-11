//! A library's generic growth of a `Vec` of its trait's associated type's
//! items, `S::Item`, which reads no capacity, so its consumer that does
//! compiles its own copy, given the size its trait's dictionaries carry
//! (#1260).
pub trait Source {
    type Item;
    fn items(&self) -> Vec<Self::Item>;
}

pub struct Bytes;

impl Source for Bytes {
    type Item = u8;
    fn items(&self) -> Vec<u8> {
        vec![1, 2, 3]
    }
}

pub struct Wide;

impl Source for Wide {
    type Item = [u64; 4];
    fn items(&self) -> Vec<[u64; 4]> {
        vec![[0; 4]]
    }
}

pub fn gathered<S: Source>(s: &S) -> Vec<S::Item> {
    let mut all: Vec<S::Item> = Vec::new();
    for _ in 0..3 {
        all.extend(s.items());
    }
    all.push(s.items().pop().unwrap());
    all
}

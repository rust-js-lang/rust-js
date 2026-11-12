//! quickcheck 0.9's shrinkers (#1420): a library's trait whose method
//! boxes an iterator, and an impl boxing a `filter_map` of another's, the
//! `char` shrinker's `(*self as u32).shrink().filter_map(char::from_u32)`.
//! The boxed `u32` shrinker holds what drops, which drops where the outer
//! box does, or where a consumer ends it.
pub struct Noisy(pub u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub trait Arbitrary: Clone + 'static {
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(std::iter::empty())
    }
}

pub struct UnsignedShrinker {
    x: u32,
    i: u32,
    noisy: Noisy,
}

impl UnsignedShrinker {
    pub fn new(x: u32) -> Box<dyn Iterator<Item = u32>> {
        if x == 0 {
            Box::new(std::iter::empty())
        } else {
            Box::new(vec![0].into_iter().chain(UnsignedShrinker { x, i: x / 2, noisy: Noisy(x) }))
        }
    }
}

impl Iterator for UnsignedShrinker {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.x - self.i < self.x {
            let result = Some(self.x - self.i + self.noisy.0 - self.x);
            self.i /= 2;
            result
        } else {
            None
        }
    }
}

impl Arbitrary for u32 {
    fn shrink(&self) -> Box<dyn Iterator<Item = u32>> {
        UnsignedShrinker::new(*self)
    }
}

impl Arbitrary for char {
    fn shrink(&self) -> Box<dyn Iterator<Item = char>> {
        Box::new((*self as u32).shrink().filter_map(char::from_u32))
    }
}

pub fn shrunk<A: Arbitrary>(a: &A) -> Vec<A> {
    a.shrink().collect()
}

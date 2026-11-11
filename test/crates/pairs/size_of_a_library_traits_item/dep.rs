//! A trait whose associated type the library never asks the size of, rand_core
//! 0.5's `BlockRngCore`, and an impl of it: its consumers may ask it.
pub trait BlockCore {
    type Item;
    type Results: AsRef<[Self::Item]> + Default;
    fn generate(&mut self, results: &mut Self::Results);
}

pub struct Words;

impl BlockCore for Words {
    type Item = u32;
    type Results = [u32; 4];
    fn generate(&mut self, results: &mut [u32; 4]) {
        for (i, r) in results.iter_mut().enumerate() {
            *r = i as u32;
        }
    }
}

/// A trait whose associated types have no one size, which its dictionaries
/// carry none of: a `?Sized` one, a generic one and a method's `impl Trait`.
pub trait Shapes {
    type Unsized: ?Sized + std::fmt::Display;
    type Made<T>;
    fn name(&self) -> &Self::Unsized;
    fn made<T: Clone>(&self, item: T) -> Self::Made<T>;
    fn count<T>(made: &Self::Made<T>) -> usize;
    fn evens(&self) -> impl Iterator<Item = u32>;
}

pub struct Named(pub String);

impl Shapes for Named {
    type Unsized = str;
    type Made<T> = Vec<T>;
    fn name(&self) -> &str {
        &self.0
    }
    fn made<T: Clone>(&self, item: T) -> Vec<T> {
        vec![item; self.0.len()]
    }
    fn count<T>(made: &Vec<T>) -> usize {
        made.len()
    }
    fn evens(&self) -> impl Iterator<Item = u32> {
        (0..4).map(|n| n * 2)
    }
}

/// A trait one of whose impls' types the library's contract can't say, a
/// `dyn`'s beside its parameter: its dictionaries carry no size of it, so
/// it compiles as it did.
pub trait Erased {
    type Held;
    fn held(self) -> Self::Held;
}

pub struct Wrap<T>(pub T);

impl<T> Erased for Wrap<T> {
    type Held = (T, Box<dyn Fn(&T) -> usize>);
    fn held(self) -> Self::Held {
        (self.0, Box::new(|_| 1))
    }
}

/// And one whose impl's type is a std associated type of its parameter,
/// which nothing gives it the layout of.
pub trait Summed {
    type Sum;
    fn summed(self) -> Self::Sum;
}

impl<T: std::ops::Add<u8>> Summed for Wrap<T> {
    type Sum = <T as std::ops::Add<u8>>::Output;
    fn summed(self) -> Self::Sum {
        self.0 + 1
    }
}

/// And one whose type is another trait's associated type whose size that
/// trait's dictionaries don't carry.
pub trait Relay {
    type Out;
    fn relayed(self) -> Self::Out;
}

pub struct Twice<E>(pub E);

impl<E: Erased> Relay for Twice<E> {
    type Out = E::Held;
    fn relayed(self) -> E::Held {
        self.0.held()
    }
}

//! `size_of::<Self::Item>()` of an associated type of a library's trait, in
//! generic code, rand 0.7's `ReseedingCore::generate` (#1420): of the
//! library's impl and of this crate's own.
use dep::{BlockCore, Shapes};
use std::mem::size_of;

struct Longs;

impl BlockCore for Longs {
    type Item = u64;
    type Results = [u64; 2];
    fn generate(&mut self, results: &mut [u64; 2]) {
        results[0] = 9;
    }
}

struct Counted<R> {
    inner: R,
    bytes: i64,
}

impl<R: BlockCore> BlockCore for Counted<R> {
    type Item = <R as BlockCore>::Item;
    type Results = <R as BlockCore>::Results;

    fn generate(&mut self, results: &mut Self::Results) {
        let num_bytes = results.as_ref().len() * size_of::<Self::Item>();
        self.bytes -= num_bytes as i64;
        self.inner.generate(results);
    }
}

fn run<R: BlockCore>(inner: R) -> i64 {
    let mut c = Counted { inner, bytes: 100 };
    let mut results = R::Results::default();
    c.generate(&mut results);
    c.generate(&mut results);
    c.bytes + size_of::<R::Item>() as i64
}

fn shown<S: Shapes>(s: &S) -> String {
    let made = s.made(size_of::<R>() as u8);
    format!("{} {} {}", s.name(), S::count(&made), s.evens().sum::<u32>())
}

type R = <dep::Words as BlockCore>::Results;

pub fn main() {
    println!("{}", shown(&dep::Named("abc".to_string())));
    println!("{}", run(dep::Words));
    println!("{}", run(Longs));
    println!("{}", run(Counted { inner: Longs, bytes: 0 }));
}

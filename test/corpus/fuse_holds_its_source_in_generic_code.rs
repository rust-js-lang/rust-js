// An iterator moved into std's compiled struct in generic code is that
// struct's to drop (#2039): a `fuse()` of a `FusedIterator` keeps its source
// past its end, so a `map`'s closure, and what it captured, drops with the
// `Fuse`, not as the source is stepped to its end; as do a `map`'s, a
// `take`'s and a `zip`'s. A source that isn't a `FusedIterator`, the crate's
// own or a boxed `dyn`, `fuse()` drops at its first `None`, as std's does.
// A `Fuse` dropped before its source's end drops it then, with what it
// holds; and a local holding one, in code that isn't generic, holds it
// alike, and one in an array holds its source as a `Fuse` held alone
// does. One generic code gives back, collected by its caller or left part
// way by a loop's `break`, drops its source with it. A panic part way drops
// what's held as Rust unwinds.
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counter {
    left: u32,
    _d: D,
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        Some(self.left)
    }
}

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn fused<I: Iterator<Item = u32>>(what: &str, it: I) {
    let mut fused = it.fuse();
    while fused.next().is_some() {}
    println!("{what}: stepped to its end");
    println!("{what}: again {:?}", fused.next());
}

fn fused_in_an_array<I: Iterator<Item = u32>>(what: &str, it: I) {
    let mut fused = [it.fuse()];
    while fused[0].next().is_some() {}
    println!("{what}: stepped to its end");
    println!("{what}: again {:?}", fused[0].next());
}

fn mapped<I: Iterator<Item = u32>>(what: &str, it: I) {
    let mut mapped = it.map(|x| x * 2);
    while mapped.next().is_some() {}
    println!("{what}: stepped to its end");
}

fn taken<I: Iterator<Item = u32>>(what: &str, it: I) {
    let mut taken = it.take(5);
    while taken.next().is_some() {}
    println!("{what}: stepped to its end");
}

fn zipped<I: Iterator<Item = u32>>(what: &str, it: I) {
    let mut zipped = it.zip(10..20);
    while zipped.next().is_some() {}
    println!("{what}: stepped to its end");
}

fn first_of<I: Iterator<Item = u32>>(what: &str, it: I) -> Option<u32> {
    let mut fused = it.fuse();
    let first = fused.next();
    println!("{what}: took {first:?}");
    first
}

fn fused_back<I: Iterator<Item = u32>>(it: I) -> std::iter::Fuse<I> {
    it.fuse()
}

fn panics_at_its_end<I: Iterator<Item = u32>>(it: I) {
    let mut fused = it.fuse();
    while fused.next().is_some() {}
    panic!("at its end");
}

fn holding(n: u32) -> impl Iterator<Item = u32> {
    let d = D(n);
    vec![1, 2].into_iter().map(move |x| {
        let d = &d;
        x + d.0
    })
}

fn main() {
    fused("fused map", holding(1));
    let d = D(13);
    fused(
        "fused map made here",
        vec![1, 2].into_iter().map(move |x| {
            let d = &d;
            x + d.0
        }),
    );
    let d = D(14);
    fused_in_an_array(
        "in an array, made here",
        vec![1, 2].into_iter().map(move |x| {
            let d = &d;
            x + d.0
        }),
    );
    fused("crate's own", Counter { left: 2, _d: D(2) });
    fused("boxed", Box::new(holding(3)) as Box<dyn Iterator<Item = u32>>);
    mapped("mapped map", holding(4));
    taken("taken map", holding(5));
    zipped("zipped map", holding(6));
    println!("returned {:?}", first_of("first of a map", holding(8)));
    {
        let mut fused = holding(9).fuse();
        while fused.next().is_some() {}
        println!("a local: stepped to its end");
        println!("a local: again {:?}", fused.next());
    }
    {
        let mut fused = holding(10).fuse();
        println!("a local's first: {:?}", fused.next());
    }
    println!("locals gone");
    let collected: Vec<u32> = fused_back(holding(11)).collect();
    println!("collected {collected:?}");
    for x in fused_back(holding(12)) {
        println!("broke at {x}");
        break;
    }
    println!("after the loop");
    let unwound = panic::catch_unwind(AssertUnwindSafe(|| panics_at_its_end(holding(7))));
    println!("unwound: {}", message(unwound.unwrap_err()));
}

//@ run-fail: capacity overflow
// Generic code's `collect()` into a `Vec`, or `Vec::from_iter`, of what its
// caller gives it collects it as std's `SpecFromIter` collects the type its
// caller knows (#1239): in the buffer a `vec::IntoIter` holds, by its
// `try_fold`, asking nothing of its hint; a `TrustedLen`'s by its fold,
// room for its hint's upper bound first, or "capacity overflow" where it
// has none, stepping nothing; another's by `next`, its first item, then
// room for its hint as the `Vec` grows. How is its caller's, `IVecCollect`,
// which a generic caller passes on as it's given it, and a
// `B: FromIterator`'s entry takes. Of items that drop, a panic drops what
// it collected and what's left as each way does. The capacity each gives
// is #1263's.
use std::panic::{AssertUnwindSafe, catch_unwind};

/// A crate's iterator of three items, whose `size_hint` prints.
struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        println!("next at {}", self.0);
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.0);
        (1, Some(4))
    }
}

/// An item that drops.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn built<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn all<I: Iterator>(it: I) -> Vec<I::Item> {
    it.collect()
}

/// Of what a type parameter's `into_iter` gives.
fn from_items<T: IntoIterator>(items: T) -> Vec<T::Item> {
    Vec::from_iter(items)
}

/// Of an adapter generic code makes of what it's given, whose answer is
/// made of its caller's: in place, `TrustedLen` or not.
fn mapped<J: Iterator<Item = u32>>(it: J) -> Vec<u32> {
    all(it.map(|x| {
        println!("stepped {x}");
        x + 1
    }))
}

/// Of a filter generic code makes of what it's given, into a `B`, which
/// panics at its third: in place, what it collected dropped first, else
/// what's left.
fn kept<J: Iterator<Item = Loud>>(it: J) -> usize {
    let v: Vec<Loud> = built(it.filter(|l| {
        assert!(l.0 != 3, "at {}", l.0);
        true
    }));
    v.len()
}

/// A generic caller's, which gives its own caller's answer on.
fn relayed<J: Iterator>(it: J) -> Vec<J::Item> {
    all(it)
}

fn shown(name: &str, v: &[u32]) {
    println!("{name}: {v:?}");
}

fn main() {
    // In place: the source's buffer.
    let source = vec![1u32, 2, 3];
    let v = all(source.into_iter().map(|x| x * 2));
    shown("in place", &v);
    let v = from_items(vec![1u32, 2]);
    shown("in place, from its items", &v);
    // By its fold: room for its hint's upper bound first.
    let v = all((0..5u32).map(|x| x + 1));
    shown("by fold", &v);
    let v = relayed((0..6u32).rev());
    shown("by fold, relayed", &v);
    let v = from_items((0..4u32).chain(7..9));
    shown("by fold, from its items", &v);
    // By `next`: its first item, then its hint.
    let v = all(Own(0));
    shown("by next", &v);
    let v = all((0..9u32).filter(|x| x % 2 == 0));
    shown("by next, filtered", &v);
    let v = relayed(Own(0).chain(4..6));
    shown("by next, relayed", &v);
    // Into a `B` of the caller's, a `Vec`.
    let v: Vec<u32> = built((0..3u32).map(|x| x * 3));
    shown("into a B", &v);
    // Of items that drop: what it collected, dropped as a panic unwinds.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<Loud> = built((1..5).map(|n| {
            assert!(n < 3, "at {n}");
            Loud(n)
        }));
        v.len()
    }));
    println!("of items that drop: {}", r.is_err());
    // And by each way: a `TrustedLen` zip by its fold, what's left first.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut count = 0;
        all((0..4u32).zip(10..14u32).map(|(a, b)| {
            count += 1;
            println!("made {count}");
            assert!(count < 3, "at {count}");
            Loud(a + b)
        }))
        .len()
    }));
    println!("zipped, of items that drop: {}", r.is_err());
    // By its fold, of a `Vec`'s items made bigger, not in place: what's
    // left of the source dropped before what it collected.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let source: Vec<Loud> = (1..5).map(Loud).collect();
        all(source.into_iter().zip(0..4u64).map(|(l, n)| {
            assert!(l.0 < 3, "at {}", l.0);
            (Loud(l.0 * 10), n)
        }))
        .len()
    }));
    println!("zipped bigger, of items that drop: {}", r.is_err());
    // In place, the source's items left dropped after what it collected.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let source: Vec<Loud> = (1..5).map(Loud).collect();
        all(source.into_iter().map(|l| {
            assert!(l.0 < 3, "at {}", l.0);
            Loud(l.0 * 10)
        }))
        .len()
    }));
    println!("in place, of items that drop: {}", r.is_err());
    // Endless, stepped once: nothing more is made before the panic.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let mut endless = std::iter::repeat(1u32).map(|n| {
            println!("made");
            Loud(n)
        });
        endless.next();
        relayed(endless).len()
    }));
    println!("endless, of items that drop: {}", r.is_err());
    // Made of a caller's: in place, by fold, by `next`.
    shown("mapped, in place", &mapped(vec![1, 2].into_iter()));
    shown("mapped, by fold", &mapped(0..2));
    shown("mapped, by next", &mapped((0..5).filter(|x| x % 2 == 0)));
    let r = catch_unwind(AssertUnwindSafe(|| kept((1..5).map(Loud))));
    println!("kept, by next: {}", r.is_err());
    let source: Vec<Loud> = (1..5).map(Loud).collect();
    let r = catch_unwind(AssertUnwindSafe(|| kept(source.into_iter())));
    println!("kept, in place: {}", r.is_err());
    // Endless and `TrustedLen`, stepped once first: nothing more is stepped.
    let mut endless = std::iter::repeat(7u32);
    endless.next();
    let v = mapped(endless);
    shown("endless", &v);
}

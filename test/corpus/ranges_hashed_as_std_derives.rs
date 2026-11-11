//@ library-differs
// std's ranges hash as their derived `Hash` writes them: each bound in its
// field's order, then of `a..=b` whether it's exhausted (#1361). As keys of
// a hash map or set, a generic `T: Hash`'s, inside a key whose `Hash` is
// derived, of bounds whose own `Hash` prints, and hashed by a std hasher,
// whose `finish()` is std's; never a slice's, whose length is a `usize`,
// wasm32's 4 bytes where the host's native run writes 8.
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

fn hashed<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

fn distinct<T: Hash + Eq>(items: Vec<T>) -> usize {
    items.into_iter().collect::<HashSet<_>>().len()
}

#[derive(PartialEq, Eq, Hash, Debug)]
struct Span {
    file: &'static str,
    lines: Range<u32>,
}

/// A bound whose `Hash` prints what it writes.
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy)]
struct Line(u32);

impl Hash for Line {
    fn hash<H: Hasher>(&self, state: &mut H) {
        println!("hash line {}", self.0);
        self.0.hash(state);
    }
}

fn main() {
    let set: HashSet<Range<i32>> = [0..3, 1..4, 0..3].into_iter().collect();
    println!("{}", set.len());
    let mut counts: HashMap<Range<u8>, u32> = HashMap::new();
    for range in [2..5, 1..2, 2..5] {
        *counts.entry(range).or_insert(0) += 1;
    }
    println!("{:?} {:?}", counts.get(&(2..5)), counts.get(&(1..3)));
    println!("{}", distinct(vec![1..2, 1..2, 3..4]));
    println!("{}", distinct(vec![1..=2, 1..=2, 1..=3]));
    println!("{}", distinct(vec![1.., 2.., 1..]));
    println!("{}", distinct(vec![..1, ..1]));
    println!("{}", distinct(vec![.., ..]));

    // Each kind's hash, std's.
    println!("{}", hashed(&(1..5)));
    println!("{}", hashed(&(1..=5)));
    println!("{}", hashed(&(1u8..)));
    println!("{}", hashed(&(..5i64)));
    println!("{}", hashed(&(..=5u16)));
    println!("{}", hashed(&(..)));
    println!("{}", hashed(&('a'..='z')));
    println!("{}", hashed(&(String::from("a")..String::from("b"))));

    // `a..=b` stepped to its end is exhausted, and hashes and equals as
    // such, where a fresh one of the same bounds doesn't.
    let mut stepped = 1u8..=2;
    for n in stepped.by_ref() {
        println!("stepped {n}");
    }
    println!("{} {}", stepped == (1..=2), hashed(&stepped) == hashed(&(1u8..=2)));
    println!("{}", hashed(&stepped));
    println!("{}", distinct(vec![stepped, 1..=2, 1..=2]));
    let mut last = 254u8..=255;
    last.next();
    last.next();
    println!("{}", hashed(&last));

    // Inside a key whose `Hash` is derived.
    let spans: HashSet<Span> = [
        Span {
            file: "a.rs",
            lines: 1..4,
        },
        Span {
            file: "a.rs",
            lines: 1..4,
        },
        Span {
            file: "a.rs",
            lines: 1..5,
        },
    ]
    .into_iter()
    .collect();
    println!("{}", spans.len());

    // Of bounds whose own `Hash` prints: start, then end.
    let mut lines = HashSet::new();
    lines.insert(Line(1)..Line(3));
    println!("{}", lines.contains(&(Line(1)..Line(3))));
    println!("{}", hashed(&(Line(7)..=Line(9))));
}

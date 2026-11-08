// A type's `PartialOrd` and its `Ord` are each what rustc's impls say, never
// one taken for the other (#937): a derived `partial_cmp` beside a
// hand-written `cmp` compares the fields; a derived `cmp` beside a
// hand-written `partial_cmp` compares the fields too; a derive of both in one
// `#[derive]` writes `partial_cmp` as `Some(Ord::cmp(self, other))`, a
// derive apart its fields' `partial_cmp`s. std's sorts, `max`, `min`,
// `clamp` and heaps ask `<` and `<=`, `PartialOrd`'s, as an impl that prints
// shows; `Iterator::max` and `binary_search` ask `cmp`.
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A derived `PartialOrd` beside an `Ord` that orders the other way.
#[derive(PartialEq, Eq, PartialOrd, Debug, Clone, Copy)]
struct Down {
    a: u32,
    b: u32,
}

impl Ord for Down {
    fn cmp(&self, other: &Self) -> Ordering {
        other.a.cmp(&self.a).then(other.b.cmp(&self.b))
    }
}

/// A derived `Ord` beside a `PartialOrd` that orders the other way.
#[derive(PartialEq, Eq, Ord, Debug, Clone, Copy)]
struct Up(u32);

impl PartialOrd for Up {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.0.cmp(&self.0))
    }
}

/// The usual hand-written order, each method saying it's asked.
#[derive(Debug, Clone, Copy)]
struct Seen(u32);

impl PartialEq for Seen {
    fn eq(&self, other: &Self) -> bool {
        println!("  eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl Eq for Seen {}

impl PartialOrd for Seen {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        println!("  partial_cmp {} {}", self.0, other.0);
        Some(self.cmp(other))
    }
}

impl Ord for Seen {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("  cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

/// Derived in one `#[derive]`: its `partial_cmp` is its `cmp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Together(Seen);

/// Derived apart: its `partial_cmp` is its field's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[derive(Ord)]
struct Apart(Seen);

/// A `PartialOrd` that writes its own `<`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnLess(u32);

impl PartialOrd for OwnLess {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        println!("  lt {} {}", self.0, other.0);
        self.0 < other.0
    }
}

impl Ord for OwnLess {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// A quiet `PartialOrd` whose own `<` orders the other way, as a sort,
/// which asks `<`, shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Backwards(u32);

impl PartialOrd for Backwards {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        self.0 > other.0
    }
}

impl Ord for Backwards {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

fn main() {
    let (x, y) = (Down { a: 1, b: 5 }, Down { a: 2, b: 0 });
    println!("{} {:?} {:?}", x < y, x.partial_cmp(&y), x.cmp(&y));
    let mut downs = vec![y, x, Down { a: 1, b: 9 }];
    downs.sort();
    println!("sorted {downs:?}");
    downs.sort_unstable();
    println!("unstably {downs:?}");
    println!("max {:?}, by cmp {:?}", x.max(y), downs.iter().max());
    println!("clamp {:?}", Down { a: 9, b: 9 }.clamp(x, y));
    let heap: BinaryHeap<Down> = downs.iter().copied().collect();
    println!("heap {:?}", heap.into_sorted_vec());

    let (p, q) = (Up(1), Up(2));
    println!("{} {:?} {:?}", p < q, p.partial_cmp(&q), p.cmp(&q));
    let mut ups = vec![q, p, Up(0)];
    ups.sort();
    println!("sorted {ups:?}, by cmp {:?}", ups.iter().max());
    println!("max {:?} {:?}", p.max(q), std::cmp::min(p, q));
    let mut keyed = vec![(2, q), (1, p)];
    keyed.sort_by_key(|&(_, up)| up);
    println!("by key {keyed:?}");

    let seen = [Seen(3), Seen(1), Seen(2)];
    println!("sort");
    let mut v = seen.to_vec();
    v.sort();
    println!("sort_unstable");
    let mut u = seen.to_vec();
    u.sort_unstable();
    println!("max");
    println!("{:?}", Seen(1).max(Seen(2)));
    println!("min");
    println!("{:?}", Seen(1).min(Seen(2)));
    println!("Iterator::max");
    println!("{:?}", seen.iter().max());
    println!("clamp");
    println!("{:?}", Seen(5).clamp(Seen(1), Seen(3)));
    println!("heap");
    let mut heap = BinaryHeap::new();
    for s in seen {
        heap.push(s);
    }
    println!("{:?}", heap.pop());
    println!("binary_search");
    println!("{:?}", v.binary_search(&Seen(2)));
    println!("together <");
    println!("{}", Together(Seen(1)) < Together(Seen(2)));
    println!("together partial_cmp");
    println!("{:?}", Together(Seen(1)).partial_cmp(&Together(Seen(2))));
    println!("together sort");
    let mut t = vec![Together(Seen(2)), Together(Seen(1))];
    t.sort();
    println!("apart <");
    println!("{}", Apart(Seen(1)) < Apart(Seen(2)));
    println!("apart cmp");
    println!("{:?}", Apart(Seen(1)).cmp(&Apart(Seen(2))));
    println!("apart sort");
    let mut a = vec![Apart(Seen(2)), Apart(Seen(1))];
    a.sort();

    println!("own lt: sort");
    let mut own = vec![OwnLess(2), OwnLess(3), OwnLess(1)];
    own.sort();
    println!("{own:?}");
    println!("own lt: max");
    println!("{:?}", OwnLess(4).max(OwnLess(2)));
    let mut backwards = vec![Backwards(2), Backwards(3), Backwards(1)];
    backwards.sort();
    println!("backwards sorted {backwards:?}");
}

// Generic code's `==`, `<` and `partial_cmp` of a `T: Ord` are `T`'s
// `PartialEq` and `PartialOrd`, as rustc's supertraits have them, never its
// `cmp` (#944): a derived `==` beside a hand-written `cmp`, a `partial_cmp`
// that prints, an impl's own `lt`, and a derived `PartialOrd` beside a
// hand-written `Ord`, through generic functions, std's generic sorts and
// heaps, and `max` and `clamp` of a type parameter.
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// The usual hand-written order, each method saying it's asked.
#[derive(Debug, Clone, Copy)]
struct Loud(u32);

impl PartialEq for Loud {
    fn eq(&self, other: &Self) -> bool {
        println!("  eq {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl Eq for Loud {}

impl PartialOrd for Loud {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        println!("  partial_cmp {} {}", self.0, other.0);
        Some(self.cmp(other))
    }
}

impl Ord for Loud {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("  cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

/// A derived `==`, by parity, beside an `Ord` by value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Parity(bool, u32);

impl PartialOrd for Parity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Parity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

/// A `PartialOrd` that writes its own `<`, `<=`, `>` and `>=`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnOps(u32);

impl PartialOrd for OwnOps {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        println!("  lt {} {}", self.0, other.0);
        self.0 < other.0
    }

    fn le(&self, other: &Self) -> bool {
        println!("  le {} {}", self.0, other.0);
        self.0 <= other.0
    }

    fn gt(&self, other: &Self) -> bool {
        println!("  gt {} {}", self.0, other.0);
        self.0 > other.0
    }

    fn ge(&self, other: &Self) -> bool {
        println!("  ge {} {}", self.0, other.0);
        self.0 >= other.0
    }
}

impl Ord for OwnOps {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// A derived `PartialOrd` beside an `Ord` that orders the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
struct Down(u32);

impl Ord for Down {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.cmp(&self.0)
    }
}

/// A derived `Ord` beside a `PartialOrd` that orders the other way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
struct Up(u32);

impl PartialOrd for Up {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.0.cmp(&self.0))
    }
}

/// Derived whole, its dictionaries std's order of its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pair(u32, u32);

fn same<T: Ord>(a: &T, b: &T) -> bool {
    a == b
}

fn less<T: Ord>(a: &T, b: &T) -> bool {
    a < b
}

fn at_most<T: PartialOrd>(a: &T, b: &T) -> bool {
    a <= b
}

fn order<T: Ord>(a: &T, b: &T) -> Option<Ordering> {
    a.partial_cmp(b)
}

fn sorted<T: Ord + Clone>(items: &[T]) -> Vec<T> {
    let mut v = items.to_vec();
    v.sort();
    v
}

fn sorted_unstably<T: Ord + Clone>(items: &[T]) -> Vec<T> {
    let mut v = items.to_vec();
    v.sort_unstable();
    v
}

fn larger<T: Ord>(a: T, b: T) -> T {
    a.max(b)
}

fn clamped<T: Ord>(x: T, low: T, high: T) -> T {
    x.clamp(low, high)
}

fn heaped<T: Ord>(items: Vec<T>) -> Vec<T> {
    let mut heap = BinaryHeap::new();
    for item in items {
        heap.push(item);
    }
    heap.into_sorted_vec()
}

fn main() {
    println!("same loud");
    println!("{}", same(&Loud(1), &Loud(2)));
    println!("less loud");
    println!("{}", less(&Loud(1), &Loud(2)));
    println!("at_most loud");
    println!("{}", at_most(&Loud(1), &Loud(2)));
    println!("order loud");
    println!("{:?}", order(&Loud(1), &Loud(2)));
    println!("sorted loud");
    println!("{:?}", sorted(&[Loud(3), Loud(1), Loud(2)]));
    println!("sorted_unstably loud");
    println!("{:?}", sorted_unstably(&[Loud(3), Loud(1), Loud(2)]));
    println!("larger loud");
    println!("{:?}", larger(Loud(1), Loud(2)));
    println!("clamped loud");
    println!("{:?}", clamped(Loud(5), Loud(1), Loud(3)));
    println!("heaped loud");
    println!("{:?}", heaped(vec![Loud(2), Loud(3), Loud(1)]));

    println!("parity: {}", same(&Parity(true, 1), &Parity(true, 3)));

    println!("less own");
    println!("{}", less(&OwnOps(1), &OwnOps(2)));
    println!("at_most own");
    println!("{}", at_most(&OwnOps(1), &OwnOps(2)));
    println!("sorted own");
    println!("{:?}", sorted(&[OwnOps(3), OwnOps(1), OwnOps(2)]));
    println!("larger own");
    println!("{:?}", larger(OwnOps(1), OwnOps(2)));
    println!("clamped own");
    println!("{:?}", clamped(OwnOps(5), OwnOps(1), OwnOps(3)));
    println!("heaped own");
    println!("{:?}", heaped(vec![OwnOps(2), OwnOps(5), OwnOps(3), OwnOps(1), OwnOps(4)]));
    println!("concrete heap own");
    let mut heap = BinaryHeap::new();
    heap.push(OwnOps(1));
    heap.push(OwnOps(4));
    println!("{:?}", heap.pop());
    println!("concrete clamp own");
    println!("{:?}", OwnOps(0).clamp(OwnOps(1), OwnOps(3)));

    let (x, y) = (Down(1), Down(2));
    println!(
        "down: {} {:?} {:?} {:?} {:?}",
        less(&x, &y),
        order(&x, &y),
        sorted(&[y, x]),
        larger(x, y),
        heaped(vec![x, y])
    );
    let (p, q) = (Up(1), Up(2));
    println!(
        "up: {} {:?} {:?} {:?} {:?}",
        less(&p, &q),
        order(&p, &q),
        sorted(&[p, q]),
        larger(p, q),
        heaped(vec![p, q])
    );
    let (r, s) = (Pair(1, 2), Pair(1, 3));
    println!(
        "pair: {} {} {:?} {:?} {:?}",
        same(&r, &s),
        less(&r, &s),
        sorted(&[s, r]),
        clamped(Pair(0, 0), r, s),
        heaped(vec![s, r])
    );
    println!("numbers: {} {} {:?}", same(&1, &1), less(&"a", &"b"), sorted(&[3, 1, 2]));
}

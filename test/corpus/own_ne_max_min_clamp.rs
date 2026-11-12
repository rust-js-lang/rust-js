//@ run-fail: min > max. min = 3, max = 1
// An impl's own `ne`, `max`, `min` and `clamp` are what `!=` and the
// others run, as rustc calls them (#962): of generic code, through std's
// impls that ask their parts' `!=` (a reference's, a `Box`'s, an `Rc`'s, a
// tuple's, a `Pin`'s target's, of a `Box` or of the crate's own pointer,
// #1446), std's slice `==`, which asks each two `!=`, a B-tree's bulk
// build, which asks `!=` of each two in a row, and `cmp::max` and
// `cmp::min`. An integer's `clamp` is its own too, whose panic shows its
// bounds.
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, LinkedList, VecDeque};
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;

/// A pointer of the crate's own to an `Ne`, never equal itself.
struct Holder(Box<Ne>);

impl PartialEq for Holder {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Deref for Holder {
    type Target = Ne;
    fn deref(&self) -> &Ne {
        &self.0
    }
}

/// A `PartialEq` that writes its own `!=`, saying so.
#[derive(Debug, Clone, Copy)]
struct Ne(u32);

impl PartialEq for Ne {
    fn eq(&self, other: &Self) -> bool {
        println!("  eq {} {}", self.0, other.0);
        self.0 == other.0
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &Self) -> bool {
        println!("  ne {} {}", self.0, other.0);
        self.0 != other.0
    }
}

impl Eq for Ne {}

impl PartialOrd for Ne {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ne {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

/// A `PartialEq<u32>` that writes its own `!=`.
#[derive(Clone, Copy)]
struct Cents(u32);

impl PartialEq<u32> for Cents {
    fn eq(&self, other: &u32) -> bool {
        println!("  cents eq");
        self.0 == *other
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &u32) -> bool {
        println!("  cents ne");
        self.0 != *other
    }
}

/// A numbered enum whose `==` is its own, by its number's parity, beside a
/// derived order: its dictionaries aren't std's shared quiet ones.
#[derive(Debug, Clone, Copy, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Level {
    Low = 1,
    High = 3,
}

impl PartialEq for Level {
    fn eq(&self, other: &Self) -> bool {
        println!("  level eq");
        *self as u8 % 2 == *other as u8 % 2
    }
}

/// Its order prints, its `==` derived: a B-tree of them is std's nodes,
/// whose search asks what a sorted array's wouldn't.
#[derive(Debug, PartialEq, Eq)]
struct Counted(u32);

impl PartialOrd for Counted {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Counted {
    fn cmp(&self, other: &Self) -> Ordering {
        println!("  counted cmp {} {}", self.0, other.0);
        self.0.cmp(&other.0)
    }
}

/// Derived, of a sequence of them: its `!=` is `!(a == b)`, whose `==`
/// asks the sequence's, which asks each two `!=`.
#[derive(Debug, PartialEq)]
struct Bag {
    items: Vec<Ne>,
}

/// An `Ord` that writes its own `max`, `min` and `clamp`, unlike std's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
struct Picky(u32, char);

impl Ord for Picky {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }

    fn max(self, other: Self) -> Self {
        println!("  own max");
        if self.0 >= other.0 { self } else { other }
    }

    fn min(self, other: Self) -> Self {
        println!("  own min");
        if self.0 <= other.0 { self } else { other }
    }

    fn clamp(self, low: Self, high: Self) -> Self {
        println!("  own clamp");
        if self.0 < low.0 { high } else { low }
    }
}

/// One with a destructor, whose `max` is std's: the one it doesn't give
/// back is dropped.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("  drop {}", self.0);
    }
}

/// One with a destructor whose `max` is its own.
#[derive(Debug, PartialEq, Eq, PartialOrd)]
struct Keeper(u32);

impl Ord for Keeper {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }

    fn max(self, other: Self) -> Self {
        println!("  own max, keeping the first");
        drop(other);
        self
    }
}

impl Drop for Keeper {
    fn drop(&mut self) {
        println!("  drop keeper {}", self.0);
    }
}

fn differ<T: PartialEq>(a: &T, b: &T) -> bool {
    a != b
}

fn nonzero<T: PartialEq<u32>>(x: &T) -> bool {
    *x != 0
}

fn same<T: Ord>(a: &T, b: &T) -> bool {
    a == b
}

fn differ_pairs<T: PartialEq>(a: (T, u8), b: (T, u8)) -> bool {
    a != b
}

fn same_items<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a == b
}

fn biggest<T: Ord>(a: T, b: T) -> T {
    a.max(b)
}

fn least<T: Ord>(a: T, b: T) -> T {
    std::cmp::min(a, b)
}

fn clamped<T: Ord>(x: T, low: T, high: T) -> T {
    x.clamp(low, high)
}

fn main() {
    println!("generic ne");
    println!("{}", differ(&Ne(1), &Ne(2)));
    println!("concrete ne");
    println!("{}", Ne(1) != Ne(1));
    println!("ref ne");
    let (a, b) = (&Ne(3), &Ne(4));
    println!("{}", a != b);
    println!("tuple ne");
    println!("{}", (Ne(1), 2) != (Ne(1), 3));
    println!("generic ne of another type");
    println!("{}", nonzero(&Cents(0)));
    println!("generic ne of a tuple");
    println!("{}", differ(&(Ne(1), 2), &(Ne(1), 3)));
    println!("generic tuple ne");
    println!("{}", differ_pairs((Ne(5), 1), (Ne(5), 1)));
    println!("box ne");
    println!("{}", Box::new(Ne(1)) != Box::new(Ne(2)));
    println!("pin ne");
    println!("{}", Box::pin(Ne(1)) != Box::pin(Ne(2)));
    let (h, k) = (Pin::new(Holder(Box::new(Ne(6)))), Pin::new(Holder(Box::new(Ne(6)))));
    println!("{} {}", h != k, h == k);
    println!("rc ne");
    println!("{}", Rc::new(Ne(1)) != Rc::new(Ne(1)));
    println!("rc ne of one allocation");
    let shared = Rc::new(Ne(1));
    println!("{}", shared != Rc::clone(&shared));
    println!("option ne");
    println!("{}", Some(Ne(1)) != Some(Ne(2)));
    println!("vec ne");
    println!("{}", vec![Ne(1)] != vec![Ne(2)]);
    println!("vec eq");
    println!("{}", vec![Ne(1), Ne(2), Ne(3)] == vec![Ne(1), Ne(5), Ne(3)]);
    println!("vec eq of other lengths");
    println!("{}", vec![Ne(1)] == vec![Ne(1), Ne(2)]);
    println!("array eq");
    println!("{}", [Ne(1), Ne(2)] == [Ne(1), Ne(2)]);
    println!("deque eq");
    println!("{}", VecDeque::from([Ne(4)]) == VecDeque::from([Ne(4)]));
    println!("list eq");
    println!("{}", LinkedList::from([Ne(6)]) == LinkedList::from([Ne(6)]));
    println!("generic slice eq");
    println!("{}", same_items(&[Ne(7), Ne(8)], &[Ne(7), Ne(9)]));
    println!("starts with");
    println!("{}", [Ne(1), Ne(2), Ne(3)].starts_with(&[Ne(1), Ne(2)]));
    println!("strip prefix");
    println!("{:?}", [Ne(1), Ne(2)].strip_prefix(&[Ne(1)]));
    println!("derived of a vec ne");
    println!("{}", Bag { items: vec![Ne(1)] } != Bag { items: vec![Ne(1)] });
    println!("btree set from");
    let set: BTreeSet<Ne> = [Ne(2), Ne(1), Ne(2)].into_iter().collect();
    println!("{set:?}");
    println!("btree map from");
    let map = BTreeMap::from([(Ne(7), 'a'), (Ne(7), 'b')]);
    println!("{map:?}");

    println!("generic eq of an own eq beside a derived order");
    println!("{}", same(&Level::Low, &Level::High));
    println!("generic ne of an impl that writes no ne");
    println!("{}", differ(&Level::Low, &Level::High));
    println!("btree of a seen order");
    let counted: BTreeSet<Counted> = (1..=6).rev().map(Counted).collect();
    println!("{}", counted.contains(&Counted(6)));

    println!("generic max of an impl that writes no max");
    println!("{:?}", biggest(Ne(1), Ne(2)));
    println!("generic max");
    println!("{:?}", biggest(Picky(1, 'a'), Picky(1, 'b')));
    println!("concrete max");
    println!("{:?}", Picky(1, 'a').max(Picky(1, 'b')));
    println!("std max");
    println!("{:?}", std::cmp::max(Picky(1, 'a'), Picky(1, 'b')));
    println!("generic min");
    println!("{:?}", least(Picky(1, 'a'), Picky(1, 'b')));
    println!("generic clamp");
    println!("{:?}", clamped(Picky(5, 'x'), Picky(1, 'l'), Picky(3, 'h')));
    println!("iter max");
    println!("{:?}", [Picky(1, 'a'), Picky(1, 'b')].into_iter().max());
    println!("generic max of a destructor");
    println!("{:?}", biggest(Noisy(1), Noisy(2)));
    println!("generic min of a destructor");
    println!("{:?}", least(Noisy(3), Noisy(3)));
    println!("generic clamp of a destructor");
    println!("{:?}", clamped(Noisy(9), Noisy(4), Noisy(6)));
    println!("generic own max of a destructor");
    println!("{:?}", biggest(Keeper(1), Keeper(2)));
    println!("generic clamp of a char");
    println!("{:?}", clamped('z', 'a', 'c'));
    println!("generic clamp of text");
    println!("{:?}", clamped("m", "a", "c"));
    println!("generic clamp of a u32 the wrong way round");
    println!("{}", clamped(5u32, 3, 1));
}

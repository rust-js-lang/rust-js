//@ library-differs
// A hash table keyed by one of std's types around a key whose `Hash` is its
// own: each hashed as std's impl writes it, which runs the key's `hash`. A
// `Reverse` or a `Wrapping` its field's, as derived; a `ManuallyDrop` its
// value's; a `Pin` its target's; a `Cow` what it derefs to's; a `Bound`, a
// `Poll` or a `ControlFlow` its discriminant, then its variant's fields, as
// derived, so `Bound::Excluded(v)` hashes as `Some(v)` does. Each was refused
// (#1289).
use std::borrow::Cow;
use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem::ManuallyDrop;
use std::num::Wrapping;
use std::ops::{Bound, ControlFlow};
use std::pin::Pin;
use std::task::Poll;

thread_local! {
    static HASHED: Cell<u32> = const { Cell::new(0) };
}

/// A key whose `Hash` counts its calls.
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
struct Version(u32);

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        HASHED.set(HASHED.get() + 1);
        self.0.hash(state);
    }
}

/// A key whose `Hash` says when it runs.
struct Loud(u32);

impl Hash for Loud {
    fn hash<H: Hasher>(&self, state: &mut H) {
        println!("hash {}", self.0);
        self.0.hash(state);
    }
}

fn made(n: u32) -> u32 {
    println!("made {n}");
    n
}

fn hashed<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

fn hashed_since(before: u32) -> u32 {
    HASHED.get() - before
}

fn main() {
    let before = HASHED.get();
    let newest: HashSet<Reverse<Version>> = [Reverse(Version(1)), Reverse(Version(1)), Reverse(Version(2))]
        .into_iter()
        .collect();
    let found = newest.contains(&Reverse(Version(2)));
    println!("{} {found} hashed {}", newest.len(), hashed_since(before));
    println!("{}", hashed(&Reverse(Version(3))) == hashed(&Version(3)));
    println!("{}", hashed(&Wrapping(7u8)) == hashed(&7u8));
    // Each part made before any is hashed, as std makes the tuple first.
    let mut state = DefaultHasher::new();
    Reverse((Loud(made(1)), Loud(made(2)))).hash(&mut state);
    (Loud(made(3)), Loud(made(4))).hash(&mut state);

    let before = HASHED.get();
    let held: HashMap<ManuallyDrop<Version>, &str> = [(ManuallyDrop::new(Version(4)), "four")].into_iter().collect();
    let four = held.get(&ManuallyDrop::new(Version(4)));
    println!("{four:?} hashed {}", hashed_since(before));

    let pinned: HashSet<Pin<Box<Version>>> = [Box::pin(Version(5)), Box::pin(Version(5))].into_iter().collect();
    println!(
        "{} {}",
        pinned.len(),
        hashed(&Box::pin(Version(5))) == hashed(&Version(5))
    );

    let one = Version(6);
    let cows: HashSet<Cow<'_, Version>> = [Cow::Borrowed(&one), Cow::Owned(Version(6)), Cow::Owned(Version(7))]
        .into_iter()
        .collect();
    let owned: Cow<'_, [Version]> = Cow::Owned(vec![Version(8)]);
    println!("{} {}", cows.len(), hashed(&owned) == hashed(&[Version(8)][..]));

    let before = HASHED.get();
    let bounds: HashSet<Bound<Version>> = [
        Bound::Included(Version(1)),
        Bound::Excluded(Version(1)),
        Bound::Unbounded,
    ]
    .into_iter()
    .collect();
    let excluded = bounds.contains(&Bound::Excluded(Version(1)));
    println!("{} {excluded} hashed {}", bounds.len(), hashed_since(before));
    println!(
        "{} {}",
        hashed(&Bound::Excluded(Version(9))) == hashed(&Some(Version(9))),
        hashed(&Bound::Included(Version(9))) == hashed(&Some(Version(9)))
    );

    let polls: HashMap<Poll<Option<Reverse<Version>>>, u8> = [
        (Poll::Ready(Some(Reverse(Version(2)))), 1),
        (Poll::Ready(None), 2),
        (Poll::Pending, 3),
    ]
    .into_iter()
    .collect();
    let ready = polls.get(&Poll::Ready(Some(Reverse(Version(2)))));
    println!("{ready:?} {:?}", polls.get(&Poll::Pending));

    let flows: HashSet<ControlFlow<Version, u8>> = [ControlFlow::Break(Version(1)), ControlFlow::Continue(1)]
        .into_iter()
        .collect();
    println!("{} {}", flows.len(), flows.contains(&ControlFlow::Break(Version(1))));
}

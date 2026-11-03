// One iterator type, one representation, wherever its values meet (#537):
// a `map()` made where it's given to generic code, to `mem::replace`, to
// `Vec::push` or to a struct's field is std's struct, as the one a
// variable holds is, so code that steps one steps the other. A `T` of
// `mem::replace` or `Vec::push` holds what it's given; a generic function's
// `&mut I` and another `I` meet, so it's given as it is and written in
// place. What's moved out of one is apart from what's written there, its
// steps its own, and what becomes it knows its back where it did. The
// function runs as each item is stepped, as Rust's does.
use std::mem;

fn tens(x: u32) -> u32 {
    println!("tens {x}");
    x * 10
}

fn odd(x: &u32) -> bool {
    println!("odd {x}");
    x % 2 == 1
}

fn replace_in<I: Iterator<Item = u32>>(it: &mut I, next: I) {
    *it = next;
}

fn swap_in<I: Iterator<Item = u32>>(a: &mut I, b: &mut I) {
    mem::swap(a, b);
}

fn replaced<I: Iterator<Item = u32>>(it: &mut I, next: I) -> I {
    mem::replace(it, next)
}

fn set_get<T>(place: &mut T, value: T) -> &mut T {
    *place = value;
    place
}

struct Holder<I> {
    items: I,
}

impl<I: Iterator<Item = u32>> Holder<I> {
    fn refill(&mut self, items: I) -> I {
        mem::replace(&mut self.items, items)
    }
}

fn main() {
    // The issue's: generic code writes one made in place over a variable's.
    let mut it = (0..3).map(tens);
    println!("{:?}", it.next());
    replace_in(&mut it, (5..7).map(tens));
    println!("{:?}", it.size_hint());
    println!("{:?}", it.collect::<Vec<_>>());

    // Moved out apart: the old one steps its own items.
    let mut it = (0..3).map(tens);
    println!("{:?}", it.next());
    let old = replaced(&mut it, (5..7).map(tens));
    println!("{:?}", it.collect::<Vec<_>>());
    println!("{:?}", old.collect::<Vec<_>>());

    // Swapped by generic code, each stepped from its back after.
    let mut a = (0..3).map(tens);
    let mut b = (5..8).map(tens);
    println!("{:?}", a.next());
    swap_in(&mut a, &mut b);
    println!("{:?} {:?}", a.next_back(), a.len());
    println!("{:?} {:?}", b.next_back(), b.len());
    println!("{:?} {:?}", a.collect::<Vec<_>>(), b.collect::<Vec<_>>());

    // A `T` bound by nothing holds it, the same type stepped from its back.
    let mut it = (0..3).map(tens);
    println!("{:?}", it.next());
    set_get(&mut it, (5..8).map(tens));
    println!("{:?} {}", it.size_hint(), it.len());
    println!("{:?}", it.next_back());
    println!("{:?}", it.rev().collect::<Vec<_>>());

    // `mem::replace` and `mem::swap` in place.
    let mut it = (0..3).map(tens);
    println!("{:?}", it.next());
    let old = mem::replace(&mut it, (5..8).map(tens));
    println!("{:?} {:?}", it.size_hint(), old.size_hint());
    println!("{:?} {:?}", it.next_back(), old.len());
    let mut other = (10..12).map(tens);
    mem::swap(&mut it, &mut other);
    println!("{:?} {:?}", it.collect::<Vec<_>>(), other.collect::<Vec<_>>());

    // A `Vec` of them, one pushed made in place.
    let mut maps = vec![(0..2).map(tens)];
    maps.push((3..5).map(tens));
    for map in &mut maps {
        println!("{:?} {:?}", map.size_hint(), map.next_back());
    }
    println!("{:?}", maps.into_iter().flatten().collect::<Vec<_>>());

    // A struct's field, refilled by one made in place.
    let mut holder = Holder {
        items: (0..3).map(tens),
    };
    println!("{:?}", holder.items.next());
    let old = holder.refill((7..9).map(tens));
    println!("{:?} {:?}", holder.items.len(), old.len());
    println!("{:?}", holder.items.next_back());
    println!("{:?} {:?}", holder.items.collect::<Vec<_>>(), old.collect::<Vec<_>>());

    // Other adapters: a filter, a chain, a rev, a take and a skip.
    let mut filtered = (0..6).filter(odd);
    println!("{:?}", filtered.next());
    let old = replaced(&mut filtered, (6..10).filter(odd));
    println!("{:?} {:?}", filtered.next_back(), old.collect::<Vec<_>>());
    println!("{:?}", filtered.collect::<Vec<_>>());

    let mut chained = (0..2).chain(5..7);
    println!("{:?}", chained.next());
    let old = replaced(&mut chained, (10..11).chain(20..22));
    println!("{:?} {:?}", chained.size_hint(), old.size_hint());
    println!("{:?} {:?}", chained.collect::<Vec<_>>(), old.collect::<Vec<_>>());

    let mut reversed = (0..4).rev();
    println!("{:?}", reversed.next());
    let old = mem::replace(&mut reversed, (10..13).rev());
    println!("{:?} {:?}", reversed.len(), old.len());
    println!("{:?} {:?}", reversed.collect::<Vec<_>>(), old.collect::<Vec<_>>());

    let mut taken = (0..10).take(3);
    let mut skipped = (0..10).skip(7);
    println!("{:?} {:?}", taken.next(), skipped.next());
    let old_taken = replaced(&mut taken, (20..30).take(2));
    let old_skipped = mem::replace(&mut skipped, (20..25).skip(3));
    println!("{:?} {:?}", taken.size_hint(), skipped.size_hint());
    println!("{:?} {:?}", taken.next_back(), skipped.next_back());
    println!("{:?} {:?}", taken.collect::<Vec<_>>(), skipped.collect::<Vec<_>>());
    println!(
        "{:?} {:?}",
        old_taken.collect::<Vec<_>>(),
        old_skipped.collect::<Vec<_>>()
    );
}

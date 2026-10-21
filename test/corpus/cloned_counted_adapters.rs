// An `enumerate`, a `take` and a `skip` that keep their count explicit (ADR
// 0542): cloned where they've stepped, each a clone of what it steps at the
// count it's at; and held by a struct, stepped from either end, its length
// asked, and shown as std shows it.
use std::iter::{Skip, Take};

#[derive(Debug)]
struct Window {
    rows: Take<std::vec::IntoIter<u32>>,
    rest: Skip<std::ops::Range<u32>>,
}

fn main() {
    let mut numbered = "abcde".chars().enumerate();
    numbered.next();
    let copy = numbered.clone();
    println!("{} {:?}", numbered.count(), copy.collect::<Vec<_>>());

    let mut first = (10..20).take(4);
    first.next();
    let rest = first.clone();
    println!("{:?} {}", first.collect::<Vec<_>>(), rest.sum::<i32>());

    let tail = (1..6).skip(2);
    let fresh = tail.clone();
    let mut stepped = tail;
    stepped.next();
    let after = stepped.clone();
    println!(
        "{:?} {:?} {:?}",
        fresh.collect::<Vec<_>>(),
        stepped.collect::<Vec<_>>(),
        after.collect::<Vec<_>>()
    );

    let mut window = Window {
        rows: vec![1, 2, 3, 4, 5].into_iter().take(3),
        rest: (0..6).skip(2),
    };
    println!("{:?} {} {}", window, window.rows.len(), window.rest.len());
    println!("{:?} {:?} {}", window.rows.next_back(), window.rows.next(), window.rows.len());
    println!("{:?} {:?} {}", window.rest.next_back(), window.rest.next(), window.rest.len());
    println!("{window:?}");
}

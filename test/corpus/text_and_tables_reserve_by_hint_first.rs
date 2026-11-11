// A `String` collected or extended from `char`s, and a hash table from
// what it's given, reserves room for the hint's lower bound before it
// steps it, as std's `Extend<char>` and hashbrown's `extend` do: where that
// bound passes the room there is, an endless `repeat`, a range past
// `isize::MAX` bytes, the panic is `capacity overflow`, or hashbrown's
// "Hash table capacity overflow", before an item is taken; where it fits,
// a short range, a string's own `chars()`, nothing changes (#1358).
use std::collections::{HashMap, HashSet};
use std::iter::repeat;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn check(name: &str, f: impl FnOnce() -> usize) {
    let r = catch_unwind(AssertUnwindSafe(f));
    match r {
        Ok(n) => println!("{name}: {n}"),
        Err(e) => println!(
            "{name}: panicked {:?}",
            e.downcast_ref::<String>()
                .map(String::as_str)
                .or(e.downcast_ref::<&str>().copied())
        ),
    }
}

fn made(c: char) -> char {
    println!("made {c}");
    c
}

fn main() {
    check("string of repeat", || repeat('x').collect::<String>().len());
    check("string of a long range", || {
        (0..usize::MAX / 2 + 1).map(|_| 'y').collect::<String>().len()
    });
    check("string of a cycle", || {
        vec!['a', 'b'].into_iter().cycle().collect::<String>().len()
    });
    check("string of an option's cycle", || {
        Some('q').into_iter().cycle().collect::<String>().len()
    });
    check("string of a filtered cycle", || {
        vec!['a', 'b']
            .into_iter()
            .cycle()
            .filter(|_| true)
            .take(3)
            .collect::<String>()
            .len()
    });
    check("string extended", || {
        let mut s = String::from("a");
        s.extend(repeat('z'));
        s.len()
    });
    check("string extended by refs", || {
        let mut s = String::new();
        s.extend(repeat(&'r'));
        s.len()
    });
    check("string of chars", || {
        "héllo".chars().map(made).collect::<String>().len()
    });
    check("string of a short range", || {
        (0..3).map(|i| char::from(b'a' + i)).collect::<String>().len()
    });
    check("map of repeat", || {
        repeat((1u8, 2u8)).collect::<HashMap<u8, u8>>().len()
    });
    check("set extended", || {
        let mut s = HashSet::from([1u32]);
        s.extend(repeat(7u32).map(|n| n + 1));
        s.len()
    });
    check("map of a short range", || {
        (0..3u8).map(|i| (i, i)).collect::<HashMap<u8, u8>>().len()
    });
}

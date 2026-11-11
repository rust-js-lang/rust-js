// A `cycle()` of a `map` of a string's `chars()`, collected into a `String` or
// a hash table or extending one: std reserves room for its hint first,
// `usize::MAX` where it has items, and panics `capacity overflow` before it
// steps it. The map is held, std's struct, whose hint asks the `chars()`'
// own, where JS's own `map` would know none and the cycle would step
// forever; an empty one, a taken one and a plain map are as before (#1367).
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn check(name: &str, f: impl FnOnce() -> usize) {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(n) => println!("{name}: {n}"),
        Err(e) => println!(
            "{name}: panicked {:?}",
            e.downcast_ref::<&str>()
                .copied()
                .or(e.downcast_ref::<String>().map(String::as_str))
        ),
    }
}

fn made(c: char) -> char {
    println!("made {c}");
    c
}

fn main() {
    check("string of a cycle", || {
        "ab".chars().map(made).cycle().collect::<String>().len()
    });
    check("string of indices' cycle", || {
        "ab".char_indices()
            .map(|(_, c)| made(c))
            .cycle()
            .collect::<String>()
            .len()
    });
    check("string extended by a cycle", || {
        let mut s = String::new();
        s.extend("xy".chars().map(made).cycle());
        s.len()
    });
    check("set of a cycle", || {
        "ab".chars().map(made).cycle().collect::<HashSet<char>>().len()
    });
    check("empty cycle", || "".chars().map(made).cycle().collect::<String>().len());
    check("cycle taken", || {
        "ab".chars().map(made).cycle().take(5).collect::<String>().len()
    });
    check("plain map", || "ab".chars().map(made).collect::<String>().len());
}

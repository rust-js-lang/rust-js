//! A consumer collecting a `Vec`'s own iterator, and an adapter of it,
//! through the library's defaults, which std collects in place, its
//! buffer reused, where the library's JS takes nothing for the items it
//! drops or the `Ord` its default's where clause asks, so the crate
//! compiles the defaults itself (#1370): of numbers, of items with a
//! destructor, sorted, and mapped by a closure that panics part-way, what
//! it took and what's left each dropped as std's in-place collect drops
//! them.
use dep::Tools;
use std::panic::{self, AssertUnwindSafe};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let v = vec![3, 1, 2].into_iter().collect_vec();
    println!("{v:?}");
    let doubled = vec![3u32, 1, 2].into_iter().map(|n| n * 2).collect_vec();
    println!("{doubled:?}");
    let sorted: Vec<u8> = vec![3, 1, 2].into_iter().sorted().collect();
    println!("{sorted:?}");
    let louds = vec![Loud(1), Loud(2)].into_iter().collect_vec();
    println!("{}", louds.len());
    drop(louds);
    let made = panic::catch_unwind(AssertUnwindSafe(|| {
        vec![Loud(10), Loud(11), Loud(12), Loud(13)]
            .into_iter()
            .map(|loud| {
                if loud.0 == 12 {
                    panic!("at 12");
                }
                Loud(loud.0 + 100)
            })
            .collect_vec()
    }));
    println!("{}", made.is_err());
}

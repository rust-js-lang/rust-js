// A `zip` whose second side's `next` panics drops the item it took from
// its first side as it unwinds, as std's `Zip::next` holds it (#695):
// collected, `last()`ed and folded, then what each side has left.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Panicky {
    items: std::vec::IntoIter<D>,
    at: u32,
}

impl Iterator for Panicky {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        let next = self.items.next();
        if next.as_ref().is_some_and(|d| d.0 == self.at) {
            panic!("boom");
        }
        next
    }
}

fn ds(r: std::ops::Range<u32>) -> Vec<D> {
    r.map(D).collect()
}

fn panicky(r: std::ops::Range<u32>, at: u32) -> Panicky {
    Panicky {
        items: ds(r).into_iter(),
        at,
    }
}

fn run(name: &str, f: impl FnOnce()) {
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {}", r.is_err());
}

fn main() {
    run("last", || {
        let x = ds(30..35).into_iter().zip(panicky(40..45, 43)).last();
        println!("{}", x.is_some());
    });
    run("collect", || {
        let x: Vec<(D, D)> = ds(50..54).into_iter().zip(panicky(60..64, 62)).collect();
        println!("{}", x.len());
    });
    run("fold", || {
        let n = ds(70..74)
            .into_iter()
            .zip(panicky(80..84, 81))
            .fold(0, |n, (a, b)| n + a.0 + b.0);
        println!("{n}");
    });
    run("next", || {
        let mut zipped = ds(90..93).into_iter().zip(panicky(100..103, 100));
        println!("{}", zipped.next().is_some());
    });
}

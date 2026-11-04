// A `collect()` of the crate's iterator with a `Drop`, whose `next`
// panics part-way: std's collect frame holds the iterator inside the one
// that holds what it collected, so the iterator drops first, then what it
// collected, into a `Vec` and a `BTreeSet`; and once, at its
// end, where none panics (#655).
use std::collections::BTreeSet;
use std::panic::catch_unwind;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Own(u32, u32);

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        if self.0 == self.1 {
            panic!("next");
        }
        (self.0 < 5).then(|| D(self.0))
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own");
    }
}

fn main() {
    let r = catch_unwind(|| Own(0, 3).collect::<Vec<_>>().len());
    println!("vec {}", r.is_err());
    let r = catch_unwind(|| Own(0, 3).collect::<BTreeSet<_>>().len());
    println!("btree set {}", r.is_err());
    let all: Vec<D> = Own(0, 9).collect();
    println!("collected {}", all.len());
}

// The crate's own `count` and `last`, asked through a `chain`, a `skip`
// and an `enumerate`, take what they're given and drop it, as they unwind
// too: what holds them drops it never again, though it's closed after, as
// the chain is as it unwinds (#870).
use std::panic;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Counts(Vec<D>, bool);

impl Iterator for Counts {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn count(self) -> usize {
        println!("own count of {}", self.0.len());
        if self.1 {
            panic!("count panics");
        }
        self.0.len()
    }

    fn last(self) -> Option<D> {
        println!("own last of {}", self.0.len());
        if self.1 {
            panic!("last panics");
        }
        let mut items = self.0;
        items.pop()
    }
}

fn counts(first: u32, panics: bool) -> Counts {
    Counts(vec![D(first), D(first + 1)], panics)
}

fn main() {
    let counted = panic::catch_unwind(|| counts(1, true).chain(counts(3, false)).count());
    println!("chain count panicked: {}", counted.is_err());
    let counted = panic::catch_unwind(|| counts(5, false).chain(counts(7, true)).count());
    println!("chain count panicked: {}", counted.is_err());
    let counted = panic::catch_unwind(|| counts(11, true).skip(1).count());
    println!("skip count panicked: {}", counted.is_err());
    let counted = panic::catch_unwind(|| counts(13, true).enumerate().count());
    println!("enumerate count panicked: {}", counted.is_err());
    let last = panic::catch_unwind(|| counts(21, true).chain(counts(23, false)).last().map(|d| d.0));
    println!("chain last panicked: {}", last.is_err());
    let last = panic::catch_unwind(|| counts(25, false).chain(counts(27, true)).last().map(|d| d.0));
    println!("chain last panicked: {}", last.is_err());
    let last = panic::catch_unwind(|| counts(31, true).skip(1).last().map(|d| d.0));
    println!("skip last panicked: {}", last.is_err());
    println!("{}", counts(41, false).chain(counts(43, false)).count());
    println!("{:?}", counts(45, false).chain(counts(47, false)).last().map(|d| d.0));
    println!("{:?}", counts(51, false).skip(1).last().map(|d| d.0));
}

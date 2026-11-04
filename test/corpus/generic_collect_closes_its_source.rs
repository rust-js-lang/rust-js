// Generic code's `src.collect()` of a type parameter's iterator closes it,
// as it does the crate's own (#655): a panic in its `next` drops it before
// what it collected, as std's collect holds it inside the frame that holds
// its `Vec`; at its end, before what comes after (#640).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// Counts up to 5, panicking at `boom`, and prints as it's dropped.
struct Every {
    at: u32,
    boom: u32,
}

impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.at += 1;
        if self.at == self.boom {
            panic!("next");
        }
        (self.at <= 5).then(|| D(self.at))
    }
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop src at {}", self.at);
    }
}

fn collected<I: Iterator<Item = D>>(src: I) -> usize {
    let v: Vec<D> = src.collect();
    println!("collected {}", v.len());
    v.len()
}

fn tried<I: Iterator<Item = D>>(src: I) -> usize {
    let v: Option<Vec<D>> = src.map(Some).collect();
    v.map_or(0, |v| v.len())
}

fn main() {
    println!("{}", catch_unwind(|| collected(Every { at: 0, boom: 3 })).is_err());
    println!("{}", catch_unwind(|| collected(Every { at: 0, boom: 0 })).is_err());
    println!("{}", catch_unwind(|| collected(vec![D(7), D(8)].into_iter())).is_err());
    println!("{}", catch_unwind(|| tried(Every { at: 0, boom: 3 })).is_err());
}

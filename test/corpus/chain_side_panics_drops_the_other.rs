// A `chain` whose first side's `next` or own `last` panics, consumed by
// `count()` or `last()`, drops its second side as it unwinds, as Rust
// drops the `Chain`; and one whose second side's panics, by `last()`,
// drops the first side's last item it kept (#647); of one made eager,
// its sides spread, too (#667).
use std::panic::catch_unwind;
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Own(u32);
impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 += 1;
        if self.0 == 2 {
            panic!("next");
        }
        Some(D(self.0))
    }
}
/// Counts up as it's stepped, and prints as it's dropped.
struct Counter(u32);
impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        Some(self.0).filter(|&n| n < 3)
    }
}
impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop counter {}", self.0);
    }
}
/// Counts up, panicking at its second; its own `last` panics.
struct Plain(u32);
impl Iterator for Plain {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        if self.0 == 2 {
            panic!("plain");
        }
        Some(self.0)
    }
    fn last(self) -> Option<u32> {
        panic!("plain last");
    }
}
/// Counts up, panicking at its second, with std's `last`.
struct Spent(u32);
impl Iterator for Spent {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        if self.0 == 2 {
            panic!("spent");
        }
        Some(self.0)
    }
}
fn main() {
    let r = catch_unwind(|| Spent(0).chain(Counter(0)).last());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Spent(0).chain(Counter(0)).skip(1).last());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Spent(0).chain(Counter(0)).collect::<Vec<u32>>().len());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Plain(0).chain(Counter(0)).last());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Plain(0).chain(Counter(0)).count());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Own(0).chain(vec![D(6), D(7)]).take(3).count());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Own(0).chain(vec![D(6), D(7)]).count());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Own(0).chain(vec![D(6), D(7)]).last().map(|d| d.0));
    println!("{}", r.is_err());
    let r = catch_unwind(|| vec![D(8), D(9)].into_iter().chain(Own(0)).last().map(|d| d.0));
    println!("{}", r.is_err());
    let r = catch_unwind(|| vec![D(10), D(11)].into_iter().chain(Own(0)).count());
    println!("{}", r.is_err());
}

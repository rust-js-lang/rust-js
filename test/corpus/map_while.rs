// `map_while` steps while its function gives `Some`, and ends at the first
// `None`, as Rust's does: over an array, a range, the crate's iterator, and
// what's left of an iterator after it stops.
struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
}

fn main() {
    let words = ["3", "14", "x", "15"];
    let numbers: Vec<u32> = words.iter().map_while(|w| w.parse().ok()).collect();
    println!("{numbers:?}");
    let halves: Vec<u32> = (0..10).map_while(|n| (n < 4).then_some(n / 2)).collect();
    println!("{halves:?}");
    let total: u32 = Countdown(6).map_while(|n| n.checked_sub(2)).sum();
    println!("{total}");
    let mut it = [1, 2, 0, 4, 5].iter();
    let before: Vec<i32> = it.by_ref().map_while(|&n| (n != 0).then_some(n * 10)).collect();
    let after: Vec<&i32> = it.collect();
    println!("{before:?} {after:?}");
}

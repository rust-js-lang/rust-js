// A `step_by(n)`'s `nth` asks its source's `next` for its first item and
// stops there where that's `None`, as std's `StepBy::nth` does: a source
// that isn't fused, a chain whose second side gives `None` once, then its
// items, is asked nothing more, so `nth` is `None` (#2124).

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Gives `None` once, then its items.
struct Flaky(bool, Vec<u32>);

impl Iterator for Flaky {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if !self.0 {
            self.0 = true;
            println!("flaky none");
            return None;
        }
        if self.1.is_empty() {
            None
        } else {
            Some(self.1.remove(0))
        }
    }
}

fn main() {
    let louds: Vec<Loud> = Vec::new();
    let n = louds.into_iter().map(|l| l.0).chain(Flaky(false, vec![7, 8, 9])).step_by(2).nth(1);
    println!("past an empty first {n:?}");
    let louds = vec![Loud(1), Loud(2)];
    let n = louds.into_iter().map(|l| l.0).chain(Flaky(true, vec![7, 8, 9])).step_by(2).nth(1);
    println!("of five {n:?}");
}

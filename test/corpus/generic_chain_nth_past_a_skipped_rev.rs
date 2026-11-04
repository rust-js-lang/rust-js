// Generic code's `a.chain(b).nth(3)` of a `skip()` of a `rev()` of a
// `Vec`'s items with a destructor: the chain asks its first side's own
// `advance_by`, std's `Skip`'s, which is its source's of what it skips and
// more, a `Vec`'s `advance_back_by`, so the `Vec`'s items are dropped as
// one slice, front to back, 1 before 4, not each as it's stepped (#286).
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn chained_nth<I: Iterator<Item = Noisy>>(a: I, b: I) -> Option<u32> {
    a.chain(b).nth(3).map(|n| n.0)
}

fn noisy(n: u32) -> Vec<Noisy> {
    (1..=n).map(Noisy).collect()
}

fn main() {
    let skips = |n| noisy(n).into_iter().rev().skip(1);
    println!("{:?}", chained_nth(skips(4), skips(3)));
    let first = noisy(5).into_iter().rev().skip(2);
    let second = noisy(2).into_iter().rev().skip(0);
    println!("{:?}", chained_nth(first, second));
}

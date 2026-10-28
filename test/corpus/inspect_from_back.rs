// An `inspect(f)` stepped from its back, `rev()`'d, of the crate's own
// iterator and of a `Vec`'s: `f` sees each item as it's given, from the
// back, its source's own `next_back`, as Rust's `Inspect` calls it.
struct Down {
    n: u32,
}

impl Iterator for Down {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            self.n
        })
    }
}

impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        (self.n > 0).then(|| {
            self.n -= 1;
            100 + self.n
        })
    }
}

fn main() {
    let mut seen = Vec::new();
    let r: Vec<u32> = Down { n: 3 }.inspect(|x| seen.push(*x)).rev().collect();
    println!("{r:?} {seen:?}");
    let mut seen = Vec::new();
    let r: Vec<u32> = vec![1u32, 2, 3]
        .into_iter()
        .inspect(|x| seen.push(*x * 10))
        .rev()
        .take(2)
        .collect();
    println!("{r:?} {seen:?}");
    let mut seen = Vec::new();
    let last = (1..5u32).inspect(|x| seen.push(*x)).next_back();
    println!("{last:?} {seen:?}");
}

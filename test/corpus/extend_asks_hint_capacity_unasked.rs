// An `extend` that asks the crate's own `size_hint` as a `Vec` fills, in a
// crate that never asks a capacity: the `Vec`'s is native Rust's anyway,
// `with_capacity(n)`'s and as it grows, so the hint is asked when std asks
// it (#849).
struct Loud {
    n: u32,
}

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n == 0 {
            return None;
        }
        self.n -= 1;
        Some(self.n)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.n);
        (0, None)
    }
}

fn main() {
    let mut v: Vec<u32> = Vec::with_capacity(3);
    v.extend(Loud { n: 7 });
    v.reserve(1);
    v.extend(Loud { n: 6 });
    println!("{}", v.len());
}

// An `extend` that asks the crate's own `size_hint` as a `Vec` fills, of a
// `Vec` pushed first, collected of what isn't `TrustedLen`, a caller's, or
// a field's by an iterator generic code is given:
// its capacity std's wherever it was made, as the crate keeps each `Vec`'s
// as std grows it, so the hint is asked when std asks it (#849, #1180).
struct Loud(u32);

impl Iterator for Loud {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        print!("[hint {}] ", self.0);
        (0, None)
    }
}

fn pushed_first() -> usize {
    let mut v = Vec::new();
    v.push(1);
    v.extend(Loud(6));
    v.capacity()
}

fn collected_first(xs: &[u32]) -> usize {
    let mut v: Vec<u32> = xs.iter().copied().filter(|x| *x > 1).collect();
    v.extend(Loud(5));
    v.capacity()
}

fn given(mut v: Vec<u32>) -> usize {
    v.extend(Loud(4));
    let n = v.capacity();
    v = Vec::new();
    n + v.len()
}

fn lent(v: &mut Vec<u32>) {
    v.extend(Loud(7));
}

struct Buf {
    v: Vec<u32>,
}

impl Buf {
    fn fill<I: Iterator<Item = u32>>(&mut self, items: I) {
        self.v.extend(items);
    }
}

fn main() {
    println!("{}", pushed_first());
    println!("{}", collected_first(&[1, 2, 3, 4, 5]));
    let mut v = vec![1u32, 2];
    v.push(3);
    println!("{}", given(v));
    let mut w = Vec::with_capacity(1);
    w.push(0u32);
    lent(&mut w);
    println!("{}", w.capacity());
    let mut b = Buf { v: Vec::new() };
    b.fill(0..3);
    b.fill(Loud(3));
    println!("{}", b.v.capacity());
}

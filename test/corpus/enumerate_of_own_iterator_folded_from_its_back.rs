// An `enumerate()` folded from its back, `rfold`, of the crate's own
// double-ended iterator of items with a destructor under a `rev()`, a
// `skip(n)`, a `take(n)` or a `step_by(n)`, and of a reversed `Vec`'s
// `step_by(n)`: made knowing its back and its length where it's made, as
// the crate steps it from its back, so each item is given its index from
// the back and dropped where Rust drops it (#383, #823).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Every {
    at: u32,
    end: u32,
}

impl Every {
    fn new() -> Self {
        Every { at: 1, end: 6 }
    }
}

impl Drop for Every {
    fn drop(&mut self) {
        println!("drop src");
    }
}

impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.at += 1;
        Some(D(self.at - 1))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.end - self.at) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        if self.at == self.end {
            return None;
        }
        self.end -= 1;
        Some(D(self.end))
    }
}

impl ExactSizeIterator for Every {}

fn folded(i: usize, d: D) -> u32 {
    i as u32 * 100 + d.0
}

fn main() {
    let r = Every::new()
        .rev()
        .enumerate()
        .rfold(0, |n, (i, d)| n * 10 + folded(i, d));
    println!("rev {r}");
    let s = Every::new().skip(1).enumerate().rfold(0, |n, (i, d)| n + folded(i, d));
    println!("skip {s}");
    let t = Every::new().take(3).enumerate().rfold(0, |n, (i, d)| n + folded(i, d));
    println!("take {t}");
    let b = Every::new()
        .step_by(2)
        .enumerate()
        .rfold(0, |n, (i, d)| n + folded(i, d));
    println!("step_by {b}");
    let mut seen = Vec::new();
    let mut it = Every::new().rev().enumerate();
    if let Some((i, d)) = it.next() {
        seen.push((i, d.0));
    }
    let rest = it.rfold(0, |n, (i, d)| n + folded(i, d));
    println!("{seen:?} then {rest}");
    let v: Vec<D> = (1..=5).map(D).collect();
    let w = v
        .into_iter()
        .rev()
        .step_by(2)
        .enumerate()
        .rfold(0, |n, (i, d)| n + folded(i, d));
    println!("vec {w}");
    // A step past two or more, which a `Vec` drops as one slice, front to
    // back, the crate's iterator each as its `next_back` gives it.
    let v: Vec<D> = (1..=8).map(D).collect();
    println!("{:?}", v.into_iter().rev().step_by(3).nth_back(1).map(|d| d.0));
    let v: Vec<D> = (1..=8).map(D).collect();
    println!("{:?}", v.into_iter().step_by(3).nth_back(1).map(|d| d.0));
    let mut own = Every::new().step_by(3);
    println!("{:?}", own.next_back().map(|d| d.0));
    println!("end");
}

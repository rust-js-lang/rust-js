// A blanket impl of a trait over every iterator, `impl<I: Iterator> Trait
// for I`, whose `&mut self` method steps it: its dictionary's entry takes
// the box generic code gives a `&mut Self`, and gives the method what's in
// it, the iterator, as an object's impl's entry does (ADR 0099), which its
// `Iterator` dictionary steps (ADR 0411). A range is stepped in place, as
// Rust's is.
trait Pull {
    type Out;
    fn pull(&mut self) -> Option<Self::Out>;
    fn pull_two(&mut self) -> (Option<Self::Out>, Option<Self::Out>) {
        (self.pull(), self.pull())
    }
}

impl<I: Iterator> Pull for I {
    type Out = I::Item;
    fn pull(&mut self) -> Option<I::Item> {
        self.next()
    }
}

struct Countdown(u32);

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(self.0 + 1)
    }
}

struct Enumerated<P> {
    inner: P,
    count: usize,
}

impl<P: Pull> Pull for Enumerated<P> {
    type Out = (usize, P::Out);
    fn pull(&mut self) -> Option<(usize, P::Out)> {
        let out = self.inner.pull()?;
        self.count += 1;
        Some((self.count - 1, out))
    }
}

fn drain<P: Pull>(p: &mut P) -> Vec<P::Out> {
    let mut out = Vec::new();
    while let Some(x) = p.pull() {
        out.push(x);
    }
    out
}

fn main() {
    let mut range = 0..5;
    println!("{:?} {:?}", range.pull(), range.pull_two());
    println!("{:?}", range);
    println!("{:?}", drain(&mut range));
    let mut down = Countdown(3);
    println!("{:?} {}", down.pull_two(), down.0);
    let v = vec!["a", "b", "c"];
    let mut e = Enumerated { inner: v.iter(), count: 0 };
    println!("{:?}", e.pull());
    println!("{:?}", drain(&mut e));
    let mut r = Enumerated { inner: 10..13, count: 0 };
    println!("{:?} {:?}", drain(&mut r), r.inner);
}

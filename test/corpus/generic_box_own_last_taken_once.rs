// Generic code's sized `Box<I>` passes `last` and `nth` on to `I`'s own,
// as std's `Box<I>` does, behind std's struct that holds the box too: a
// `skip`'s, a `fuse`'s, a `peekable`'s and a chain's `last` reach the
// crate's own `last`, which takes `I` and drops it, once, where the struct
// that held the box dropped it again (#2225).
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
        Every { at: 1, end: 5 }
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

    fn last(mut self) -> Option<D> {
        println!("own last");
        let mut last = None;
        while let Some(x) = self.next() {
            last = Some(x);
        }
        last
    }

    fn nth(&mut self, n: usize) -> Option<D> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

fn skipped<I: Iterator<Item = D>>(src: Box<I>) {
    let l = src.skip(1).last();
    println!("skip last {:?}", l.as_ref().map(|d| d.0));
}

fn fused<I: Iterator<Item = D>>(src: Box<I>) {
    let l = src.fuse().fuse().last();
    println!("fuse last {:?}", l.as_ref().map(|d| d.0));
}

fn peeked<I: Iterator<Item = D>>(src: Box<I>) {
    let mut it = src.peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    let l = it.last();
    println!("peekable last {:?}", l.as_ref().map(|d| d.0));
}

fn chained<I: Iterator<Item = D>>(src: Box<I>) {
    let l = src.chain(vec![D(6), D(7)]).last();
    println!("chain last {:?}", l.as_ref().map(|d| d.0));
}

fn nth<I: Iterator<Item = D>>(src: Box<I>) {
    let mut src = src;
    let x = src.nth(1);
    println!("nth {:?}", x.as_ref().map(|d| d.0));
}

fn main() {
    skipped(Box::new(Every::new()));
    fused(Box::new(Every::new()));
    peeked(Box::new(Every::new()));
    chained(Box::new(Every::new()));
    nth(Box::new(Every::new()));
}

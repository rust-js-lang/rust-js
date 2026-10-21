// Generic code given a type with a destructor where it takes none, a
// `peekable` of an iterator of it that it gives back: std's own `Peekable`,
// compiled (ADR 0561), drops what it holds as Rust does,
// its source and what it peeked at, when the caller drops it.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn peeked<I: Iterator<Item = D>>(it: I) -> std::iter::Peekable<I> {
    it.peekable()
}

// And one generic code consumes, `sum()`'s, dropped after, by the drops
// it's given, as Rust drops it at the method's end: its source, a counter
// with a destructor of its own.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("counter dropped at {}", self.0);
    }
}

fn consumed<I: Iterator<Item = u32>>(it: I) -> u32 {
    let p = it.peekable();
    p.sum()
}

fn main() {
    println!("{}", peeked(vec![D(1), D(2)].into_iter()).count());
    println!("{}", consumed(Counter(0)));
    let mut p = peeked(vec![D(3), D(4), D(5)].into_iter());
    println!("{}", p.peek().map(|d| d.0).unwrap_or(0));
    println!("{}", p.next().map(|d| d.0).unwrap_or(0));
    println!("end");
}

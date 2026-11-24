//@ library-differs
// An iterator of the crate's whose own `collect` asks the size of what it's
// asked for: generic code's call, which knows it, gives it its override as
// Rust's dispatch runs it, and a caller whose own `B` it is gives it that
// call in turn; through a `map` of it, std's runs, which asks none (#2449).
// Of collections whose size is the same on every target, so native's and
// wasm32's agree. An override of a method of no type parameters of its
// own, a `max`, of an impl of none either, is read as its impl's type gives
// it, never as what its instance's arguments may not hold.
#[derive(Debug)]
struct Pair([u8; 2]);

impl FromIterator<u8> for Pair {
    fn from_iter<I: IntoIterator<Item = u8>>(items: I) -> Self {
        let mut pair = [0; 2];
        for (at, item) in items.into_iter().take(2).enumerate() {
            pair[at] = item;
        }
        Pair(pair)
    }
}

#[derive(Debug)]
struct Trio(u8, u8, u8);

impl FromIterator<u8> for Trio {
    fn from_iter<I: IntoIterator<Item = u8>>(items: I) -> Self {
        let mut items = items.into_iter();
        let mut next = || items.next().unwrap_or(0);
        Trio(next(), next(), next())
    }
}

struct Measured(u8);

impl Iterator for Measured {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        (self.0 <= 2).then_some(self.0)
    }
    fn collect<B: FromIterator<u8>>(self) -> B {
        println!("own collect into {} bytes", std::mem::size_of::<B>());
        std::iter::once(7).collect()
    }
}

struct Plain(u8);

impl Iterator for Plain {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn max(self) -> Option<u8> {
        println!("own max");
        Some(99)
    }
}

fn largest<I: Iterator<Item = u8>>(it: I) -> Option<u8> {
    it.max()
}

fn gathered_pair<I: Iterator<Item = u8>>(it: I) -> Pair {
    it.collect()
}

fn gathered_trio<I: Iterator<Item = u8>>(it: I) -> Trio {
    it.collect()
}

fn gathered_into<B: FromIterator<u8>, I: Iterator<Item = u8>>(it: I) -> B {
    it.collect()
}

fn main() {
    println!("{:?}", gathered_pair(Measured(0)));
    println!("{:?}", gathered_trio(Measured(0)));
    println!("{:?}", gathered_pair(Measured(0).map(|n| n + 1)));
    let into: Pair = gathered_into(Measured(0));
    println!("{into:?}");
    let into: Trio = gathered_into(Measured(0));
    println!("{into:?}");
    let into: Trio = gathered_into(Measured(0).map(|n| n + 1));
    println!("{into:?}");
    println!("{:?}", largest(Plain(0)));
    println!("{:?}", largest(Plain(0).map(|n| n * 2)));
}

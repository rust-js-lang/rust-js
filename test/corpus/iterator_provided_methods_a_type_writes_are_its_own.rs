// An iterator of the crate's whose impl writes its own of `Iterator`'s
// provided methods, `partition` and `peekable`, runs them where generic
// code calls them, as Rust's dispatch picks;
// through a `&mut` or a `Box` of it, or an adapter of it, std's default
// runs, as std's impls for those don't pass them on (#2414). Given a
// dictionary, `I` of an `impl Into2<I>`, by its entry; dropped once, by its
// own `partition`, which takes it.
struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn partition<B: Default + Extend<u32>, F: FnMut(&u32) -> bool>(self, _f: F) -> (B, B) {
        println!("own partition");
        (B::default(), B::default())
    }
    fn peekable(self) -> std::iter::Peekable<Self> {
        panic!("Counter is never peeked")
    }
}

impl Drop for Counter {
    fn drop(&mut self) {
        println!("drop Counter at {}", self.0);
    }
}

trait Into2<T> {
    fn into2(self) -> T;
}

impl<T> Into2<T> for T {
    fn into2(self) -> T {
        self
    }
}

fn given_evens<I: Iterator<Item = u32>>(it: impl Into2<I>) -> (Vec<u32>, Vec<u32>) {
    it.into2().partition(|n| n % 2 == 0)
}

fn given_peeked<I: Iterator<Item = u32>>(it: impl Into2<I>) -> Option<u32> {
    it.into2().peekable().peek().copied()
}

fn evens<I: Iterator<Item = u32>>(it: I) -> (Vec<u32>, Vec<u32>) {
    it.partition(|n| n % 2 == 0)
}

fn evens_lent<I: Iterator<Item = u32>>(it: &mut I) -> (Vec<u32>, Vec<u32>) {
    it.partition(|n| n % 2 == 1)
}

fn first_peeked<I: Iterator<Item = u32>>(it: I) -> Option<u32> {
    let mut p = it.peekable();
    p.peek().copied()
}

fn main() {
    println!("{:?}", given_evens(Counter(0)));
    println!("{}", std::panic::catch_unwind(|| given_peeked(Counter(0))).is_err());
    println!("{:?}", given_evens(Box::new(Counter(0))));
    println!("{:?}", evens(Counter(0)));
    println!("{}", std::panic::catch_unwind(|| first_peeked(Counter(0))).is_err());
    let mut lent = Counter(0);
    println!("{:?}", evens(&mut lent));
    println!("{:?}", evens_lent(&mut Counter(0)));
    println!("{:?}", evens(Box::new(Counter(0))));
    println!("{:?}", first_peeked(Box::new(Counter(0))));
    println!("{:?}", evens(Counter(0).filter(|_| true)));
}

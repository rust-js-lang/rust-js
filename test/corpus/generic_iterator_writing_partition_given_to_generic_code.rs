//@ library-differs
// A generic iterator of the crate's whose impl writes its own `partition`,
// `W<L>`'s, which takes its impl's evidence, `L`'s dictionary, before the
// method's own: its own runs where generic code calls it, given what it
// takes, of a `W<Counter>` and a `W<W<Counter>>`, each forwarding to the
// own of what it wraps, as Rust's dispatch picks; given to generic code
// that never calls it, it steps as it is. And an impl whose `partition`
// asks fewer bounds than the trait's, `B: Default` alone, is given what it
// asks; one that spells its bound as the trait does, `Extend<Self::Item>`,
// either's, over an `impl Iterator`, its caller's (#2414, #2421).
struct W<L>(L);

impl<L: Iterator<Item = u32>> Iterator for W<L> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
    fn partition<B: Default + Extend<u32>, F: FnMut(&u32) -> bool>(self, f: F) -> (B, B) {
        println!("own partition of W");
        self.0.partition(f)
    }
}

struct Spelled<L>(L);

impl<L: Iterator<Item = u32>> Iterator for Spelled<L> {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0.next()
    }
    fn partition<B, F>(self, f: F) -> (B, B)
    where
        B: Default + Extend<Self::Item>,
        F: FnMut(&Self::Item) -> bool,
    {
        println!("own partition of Spelled");
        self.0.partition(f)
    }
}

fn odds() -> Spelled<impl Iterator<Item = u32>> {
    Spelled((1..6).filter(|n| n % 2 == 1))
}

struct Counter(u32);

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 3).then_some(self.0)
    }
    fn partition<B: Default + Extend<u32>, F: FnMut(&u32) -> bool>(self, _f: F) -> (B, B) {
        println!("own partition of Counter");
        (B::default(), B::default())
    }
}

struct Loose(u32);

impl Iterator for Loose {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 <= 2).then_some(self.0)
    }
    fn partition<B: Default, F>(self, _f: F) -> (B, B) {
        println!("own partition of Loose at {}", self.0);
        (B::default(), B::default())
    }
}

fn evens<I: Iterator<Item = u32>>(it: I) -> (Vec<u32>, Vec<u32>) {
    it.partition(|n| n % 2 == 0)
}

fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.sum()
}

fn main() {
    println!("{}", total(W(1..4)));
    println!("{:?}", evens(W(1..5)));
    println!("{:?}", evens(W(Counter(0))));
    println!("{:?}", evens(W(W(Counter(0)))));
    println!("{:?}", evens(Loose(0)));
    println!("{:?}", evens(odds()));
    println!("{:?}", odds().collect::<Vec<_>>());
    let (even, odd): (Vec<u32>, Vec<u32>) = W(1..5).partition(|n| n % 2 == 0);
    println!("{even:?} {odd:?}");
}

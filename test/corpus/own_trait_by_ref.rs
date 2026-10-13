// A method of the crate's own trait named as std's iterator method is,
// `by_ref`, of the crate's own iterator: the crate's, called, not std's
// `Iterator::by_ref`, which is the iterator itself.
struct Counter {
    n: u32,
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        (self.n <= 3).then_some(self.n)
    }
}

trait Doubled {
    fn by_ref(&mut self) -> u32;
}

impl Doubled for Counter {
    fn by_ref(&mut self) -> u32 {
        self.n * 2
    }
}

fn main() {
    let mut counter = Counter { n: 1 };
    let doubled = Doubled::by_ref(&mut counter);
    let rest: Vec<u32> = Iterator::by_ref(&mut counter).collect();
    println!("{doubled} {rest:?}");
}

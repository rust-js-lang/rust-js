// A `copied()` of `Copy` structs, each its own object in JS, asks the
// crate's iterator what std's `Copied` does: its own `nth`, `last` and
// `count`, which `Copied` passes on, each item it gives copied, in
// generic code too (#2122).
#[derive(Clone, Copy, Debug)]
struct P {
    x: u32,
}
struct Refs<'a>(&'a [P]);
impl<'a> Iterator for Refs<'a> {
    type Item = &'a P;
    fn next(&mut self) -> Option<&'a P> {
        println!("next");
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
    fn count(self) -> usize {
        println!("own count");
        self.0.len()
    }
    fn last(self) -> Option<&'a P> {
        println!("own last");
        self.0.last()
    }
    fn nth(&mut self, n: usize) -> Option<&'a P> {
        println!("own nth {n}");
        let item = self.0.get(n);
        self.0 = self.0.get(n + 1..).unwrap_or(&[]);
        item
    }
}
fn generic<'a, I: Iterator<Item = &'a P>>(i: I) -> usize {
    i.copied().skip(1).count()
}
fn main() {
    let items = [P { x: 1 }, P { x: 2 }, P { x: 3 }, P { x: 4 }];
    println!("{}", Refs(&items).copied().count());
    println!("{:?}", Refs(&items).copied().last());
    println!("{:?}", Refs(&items).copied().nth(1));
    println!("{:?}", Refs(&items).copied().skip(1).count());
    let mut c = Refs(&items).copied();
    let mut first = c.next().unwrap();
    first.x += 10;
    println!("{:?} {:?} {:?}", first, c.next(), c.nth(0));
    println!("{}", generic(Refs(&items)));
    let v: Vec<P> = Refs(&items).copied().collect();
    println!("{:?}", v);
}

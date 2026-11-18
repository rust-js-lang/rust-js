// A `copied()` of `Copy` structs, each its own object in JS, stepped from
// its back through `rev()` where the crate's iterator writes its own `nth`,
// `last` and `count`: its source made knowing its back under the copy, as
// a map's is, where it read the copy as the crate's iterator (#2130). And
// one the crate clones, made of the crate's iterator itself, which its
// clone clones, where it was made of the JS iterator made of it.
#[derive(Clone, Copy, Debug)]
struct P {
    x: u32,
}
#[derive(Clone)]
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
impl<'a> DoubleEndedIterator for Refs<'a> {
    fn next_back(&mut self) -> Option<&'a P> {
        println!("back");
        let (last, rest) = self.0.split_last()?;
        self.0 = rest;
        Some(last)
    }
}
fn main() {
    let items = [P { x: 1 }, P { x: 2 }, P { x: 3 }, P { x: 4 }];
    println!("{:?}", Refs(&items).copied().rev().nth(1));
    println!("{:?}", Refs(&items).copied().rev().count());
    println!("{:?}", Refs(&items).copied().rev().last());
    let mut c = Refs(&items).copied();
    let mut first = c.next().unwrap();
    first.x += 10;
    println!("{:?} {:?} {:?}", first, c.next_back(), c.nth(0));
    let v: Vec<P> = Refs(&items).copied().rev().collect();
    println!("{:?}", v);
    let mut e = Refs(&items).copied();
    let mut f = e.clone();
    let mut g = e.next().unwrap();
    g.x += 10;
    println!("{:?} {:?} {:?} {:?}", g, f.next(), e.next_back(), f.next_back());
}

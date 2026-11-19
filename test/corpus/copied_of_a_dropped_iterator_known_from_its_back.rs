// A `copied()` of the crate's iterator with a `Drop`, of a type taken as
// stepped from its back, where the crate writes a `DoubleEndedIterator`
// for an `Own<I>` that holds it: its source made knowing its back, closed
// to drop it as std's `Copied` drops what it holds, where it's dropped,
// stepped, collected by generic code or stepped from its back, where it
// was never dropped (#2152, ADR 0545).
struct Loud<'a>(&'static str, std::slice::Iter<'a, u32>);

impl<'a> Iterator for Loud<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        self.1.next()
    }
}

impl<'a> DoubleEndedIterator for Loud<'a> {
    fn next_back(&mut self) -> Option<&'a u32> {
        self.1.next_back()
    }
}

impl Drop for Loud<'_> {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub struct Own<I>(pub I);

impl<I: Iterator> Iterator for Own<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I: DoubleEndedIterator> DoubleEndedIterator for Own<I> {
    fn next_back(&mut self) -> Option<I::Item> {
        self.0.next_back()
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let v = [1, 2, 3];
    drop(Own(Loud("dropped", v.iter()).copied()));
    let mut stepped = Own(Loud("stepped", v.iter()).copied());
    println!("{:?}", stepped.next());
    drop(stepped);
    let all: Vec<u32> = collected(Own(Loud("collected", v.iter()).copied()));
    println!("{all:?}");
    let back: Vec<u32> = Loud("reversed", v.iter()).copied().rev().collect();
    println!("{back:?}");
    let mut ends = Loud("from its back", v.iter()).copied();
    println!("{:?} {:?}", ends.next_back(), ends.next());
    drop(ends);
    println!("end");
}

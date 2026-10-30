//@ library-differs
// A `Cycle` a struct holds, as itertools' `CircularTupleWindows` does: the
// one it started with and the one it steps kept, so its `Debug`, its clone
// and an empty one's size hint are Rust's (ADR 0544).
use std::iter::Cycle;

#[derive(Debug, Clone)]
struct Circular<I: Clone> {
    iter: Cycle<I>,
    len: usize,
}

fn circular<I: Iterator + Clone>(iter: I, len: usize) -> Circular<I> {
    Circular { iter: iter.cycle(), len }
}

impl<I: Iterator + Clone> Iterator for Circular<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        self.iter.next()
    }
}

// One of an array's iterator, which shows both it holds.
#[derive(Debug)]
struct Turns {
    order: Cycle<std::vec::IntoIter<char>>,
}

fn main() {
    let mut round = circular(vec![1, 2].into_iter(), 3);
    println!("{round:?}");
    println!("{:?} {:?}", round.next(), round.next());
    println!("{round:?}");
    let copy = round.clone();
    println!("{:?} {:#?}", round.collect::<Vec<_>>(), copy);
    let mut turns = Turns { order: vec!['a', 'b'].into_iter().cycle() };
    turns.order.next();
    println!("{turns:?} {:?}", turns.order.nth(2));
    let none = circular(std::iter::empty::<u8>(), 2);
    println!("{none:?} {:?}", none.iter.size_hint());
}

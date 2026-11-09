//! A consumer's impls of the library's `Tools`, of a known item and of a
//! type parameter's, whose dictionaries carry their items' layouts, which
//! the library's defaults read (#990).
use dep::{Gather, Measure, Tools};

struct Countdown(u8);

impl Iterator for Countdown {
    type Item = (u16, u8);

    fn next(&mut self) -> Option<(u16, u8)> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some((u16::from(self.0) * 300, self.0))
    }
}

impl Tools for Countdown {}

struct Wrap<I>(I);

impl<I: Iterator> Iterator for Wrap<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

impl<I: Iterator> Tools for Wrap<I> {}

impl Gather for Countdown {}

impl Measure for Countdown {}

impl<I: Iterator> Measure for Wrap<I> {}

impl<I: Iterator> Gather for Wrap<I> {}

fn gathered<T: Gather>(items: T) -> Vec<T::Item> {
    items.gather()
}

fn layout<T: Tools>(tools: &T) -> (usize, usize) {
    tools.item_layout()
}

pub fn main() {
    println!("{:?}", Countdown(3).collect_vec());
    println!("{:?}", layout(&Countdown(1)));
    println!("{:?}", layout(&Wrap([1u64, 2].into_iter())));
    println!("{:?}", layout(&Wrap(0..3u16)));
    println!("{:?}", Wrap("ab".chars()).collect_vec());
    println!("{:?}", layout(&Wrap(Countdown(2))));
    println!("{:?}", layout(&Wrap(Wrap(0..1u32))));
    println!("{:?}", (0..2u8).fuse().item_layout());
    println!("{:?}", Countdown(2).gather());
    println!("{:?}", gathered(Wrap(Countdown(1))));
    println!("{:?}", gathered(Wrap("hi".chars())));
    println!("{}", Countdown(1).item_size());
    println!("{}", Wrap(0..2u64).item_size());
}

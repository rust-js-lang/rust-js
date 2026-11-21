// An adapter over a sized `Box` of the crate's iterator with a `Drop` is
// closed as an adapter over the iterator itself is, which drops what it
// holds once (#2219). Consumed whole, by `count`, `fold`, `last`, `max` or
// `sum`, std's compiled body takes it and drops the box there, so nothing
// drops it again after; dropped part-way, closing it drops the iterator as
// it was left. A box of a box, a generic `Box<I>`, a zip and a chain of
// boxes alike.
use std::panic::catch_unwind;

struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        if self.0 == 77 {
            panic!("next");
        }
        Some(self.0)
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own {}", self.0);
    }
}

fn skipped_count<I: Iterator<Item = u32>>(it: Box<I>) -> usize {
    it.skip(1).count()
}

fn consumed() {
    println!("a {}", Box::new(Own(3)).skip(1).count());
    println!("b {}", Box::new(Own(3)).skip(1).fold(0, |n, x| n + x));
    println!("c {:?}", Box::new(Own(3)).skip(1).last());
    println!("d {}", Box::new(Own(4)).take(2).count());
    println!("e {:?}", Box::new(Own(5)).map(|x| x + 1).max());
    println!("f {}", Box::new(Own(4)).skip(1).sum::<u32>());
    println!("g {}", Box::new(Own(3)).zip(Box::new(Own(5))).count());
    println!("h {}", Box::new(Own(3)).chain(Own(2)).count());
    let boxed: Box<Box<Own>> = Box::new(Box::new(Own(3)));
    println!("i {}", boxed.skip(1).count());
    println!("j {}", skipped_count(Box::new(Own(3))));
}

fn part_way() {
    let mut skip = Box::new(Own(4)).skip(1);
    println!("k {:?}", skip.next());
    drop(skip);
    let mut skip = Box::new(Own(8)).skip(2);
    println!("l {:?}", skip.nth(1));
    println!("l {:?}", skip.last());
    let left = Box::new(Own(6)).skip(1);
    println!("m");
    drop(left);
}

fn unwound() {
    let r = catch_unwind(|| Box::new(Own(80)).skip(1).count());
    println!("n {}", r.is_err());
}

fn main() {
    consumed();
    part_way();
    unwound();
}

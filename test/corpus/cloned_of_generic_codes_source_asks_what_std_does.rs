// A `cloned()` in generic code of what its caller gives, an iterator that
// writes `nth`, `last` and `count`, asks it what std's `Cloned` does: none
// of them, as `Cloned` leaves each to its `fold`, the crate's `next`
// here, through the adapters after it too; of a slice's iterator, read by
// index where a zip reads it so (#2131, #2122).
struct Refs<'a>(&'a [u32]);
impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        println!("next");
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
    fn count(self) -> usize {
        println!("own count");
        self.0.len()
    }
    fn last(self) -> Option<&'a u32> {
        println!("own last");
        self.0.last()
    }
    fn nth(&mut self, n: usize) -> Option<&'a u32> {
        println!("own nth {n}");
        let item = self.0.get(n);
        self.0 = self.0.get(n + 1..).unwrap_or(&[]);
        item
    }
}
fn skip_count<'a, I: Iterator<Item = &'a u32>>(i: I) -> usize {
    i.cloned().skip(1).count()
}
fn fuse_count<'a, I: Iterator<Item = &'a u32>>(i: I) -> usize {
    i.cloned().fuse().count()
}
fn fuse_last<'a, I: Iterator<Item = &'a u32>>(i: I) -> Option<u32> {
    i.cloned().fuse().last()
}
fn skip_last<'a, I: Iterator<Item = &'a u32>>(i: I) -> Option<u32> {
    i.cloned().skip(1).last()
}
fn chain_count<'a, I: Iterator<Item = &'a u32>>(i: I) -> usize {
    i.cloned().chain(4..6).count()
}
fn take_nth<'a, I: Iterator<Item = &'a u32>>(i: I) -> Option<u32> {
    i.cloned().take(3).nth(1)
}
fn peek_count<'a, I: Iterator<Item = &'a u32>>(i: I) -> usize {
    i.cloned().peekable().count()
}
fn zipped<'a, I: Iterator<Item = &'a u32>>(i: I) -> Vec<(u32, u32)> {
    i.cloned().zip(10..13u32).collect()
}
fn pass<I>(it: I) -> I {
    it
}
fn zipped_by_index<'a, J: DoubleEndedIterator<Item = &'a u32> + ExactSizeIterator>(i: J) -> Option<(u32, u32)> {
    let a = i.cloned().zip(10..13u32);
    let mut b = pass(a);
    b.nth(1)
}
fn main() {
    let items = [1, 2, 3];
    println!("skip count {}", skip_count(Refs(&items)));
    println!("fuse count {}", fuse_count(Refs(&items)));
    println!("fuse last {:?}", fuse_last(Refs(&items)));
    println!("skip last {:?}", skip_last(Refs(&items)));
    println!("chain count {}", chain_count(Refs(&items)));
    println!("take nth {:?}", take_nth(Refs(&items)));
    println!("peek count {}", peek_count(Refs(&items)));
    println!("zipped {:?}", zipped(Refs(&items)));
    println!("zipped slice {:?}", zipped(items.iter()));
    println!("zipped by index {:?}", zipped_by_index(items.iter()));
    println!("slice skip count {}", skip_count(items.iter()));
}

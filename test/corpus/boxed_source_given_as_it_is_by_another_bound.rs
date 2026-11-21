// A `Box<I>` whose `I` another bound gives as it is, `impl Is<Box<I>>`'s,
// is stepped through `I`'s dictionary, as std's `Box<I>` passes it on
// (#1611): the caller's own value, not a JS iterator of it. Its `count`,
// `last`, `nth`, a `skip`'s, and its `len()` of an exact-size one each
// ask the crate's iterator, which drops once, as native Rust does (#2223).

struct Own(u32);

impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl ExactSizeIterator for Own {}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own {}", self.0);
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn skipped_count<I: Iterator<Item = u32>>(it: impl Is<Box<I>>) -> usize {
    let it = Is::is(it);
    it.skip(0).count()
}

fn skipped_last<I: Iterator<Item = u32>>(it: impl Is<Box<I>>) -> Option<u32> {
    Is::is(it).skip(1).last()
}

fn second<I: Iterator<Item = u32>>(it: impl Is<Box<I>>) -> Option<u32> {
    let mut it = Is::is(it);
    it.nth(1)
}

fn length<I: ExactSizeIterator<Item = u32>>(it: impl Is<Box<I>>) -> usize {
    Is::is(it).len()
}

fn main() {
    println!("count {}", skipped_count(Box::new(Own(3))));
    println!("last {:?}", skipped_last(Box::new(Own(3))));
    println!("nth {:?}", second(Box::new(Own(3))));
    println!("len {}", length(Box::new(Own(4))));
}

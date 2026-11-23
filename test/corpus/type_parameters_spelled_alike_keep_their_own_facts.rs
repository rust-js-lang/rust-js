//@ library-differs
// Two functions whose type parameters are spelled alike, `I` or `T`, have
// types that look alike, `Enumerate<Box<I>>` or `Skip<IntoIter<T>>`, but
// are not the same type. What one function's body asks of its type is that
// function's own, keyed where the parameters are declared (`typed_key`).
// Here, `held` keeping its `Enumerate<Box<I>>` as std's struct no longer
// makes `folded`'s `enumerate().rfold(..)` std's struct too, which threw
// `enumerated.iter.back is not a function` where Rust steps `I`'s own
// `next_back`. `zipped_by_index` asking `Cloned<I>`'s length no longer
// refuses `zipped`, whose `I` has none. `boxed`, whose `dyn Iterator`'s
// hint is asked, makes its `Skip<Iter<T>>` knowing it, and `skipped`'s
// is JS's own (#2133).

struct D(u32);

struct Every(u32);

impl Iterator for Every {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        println!("own next");
        self.0 = self.0.checked_sub(1)?;
        Some(D(self.0))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}

impl DoubleEndedIterator for Every {
    fn next_back(&mut self) -> Option<D> {
        println!("own next_back");
        self.next()
    }
}

impl ExactSizeIterator for Every {}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn held<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) -> usize {
    let src: Box<I> = Is::is(src);
    let it = Box::new(src.enumerate());
    it.count()
}

fn folded<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) -> u32 {
    let src: Box<I> = Is::is(src);
    src.enumerate().rfold(0, |sum, (i, x)| sum * 10 + i as u32 + x.0)
}

fn pass<I>(it: I) -> I {
    it
}

fn zipped<'a, I: Iterator<Item = &'a u32>>(i: I) -> Vec<(u32, u32)> {
    i.cloned().zip(10..13u32).collect()
}

fn zipped_by_index<'a, I: DoubleEndedIterator<Item = &'a u32> + ExactSizeIterator>(i: I) -> Option<(u32, u32)> {
    let a = i.cloned().zip(10..13u32);
    let mut b = pass(a);
    b.nth(1)
}

fn boxed<T>(v: &[T]) -> Box<dyn Iterator<Item = &T> + '_> {
    Box::new(v.iter().skip(1))
}

fn count_of<J: Iterator>(it: J) -> usize {
    it.count()
}

fn skipped<T>(v: &[T]) -> usize {
    count_of(v.iter().skip(1))
}

fn main() {
    println!("held {}", held(Box::new(Every(3))));
    for n in [0, 1, 3] {
        println!("folded {}", folded(Box::new(Every(n))));
    }
    let v = vec![1, 2, 3];
    println!("{:?}", zipped(v.iter()));
    println!("{:?}", zipped_by_index(v.iter()));
    let w = [4u8, 5, 6];
    println!("hint {:?}", boxed(&w).size_hint());
    println!("skipped {}", skipped(&['a', 'b', 'c']));
}

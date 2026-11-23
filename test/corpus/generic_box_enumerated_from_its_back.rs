// A `Box<I>` generic code reads back through a trait, `Is<Box<I>>`, its
// `I` bound `DoubleEndedIterator + ExactSizeIterator`: its `enumerate()`
// stepped from its back asks `I`'s `next_back` and `len`, which `Box<I>`
// passes on, so the caller gives their dictionaries, and the JS iterator
// made of the box is given its back where it's made; one held, std's
// `Enumerate`, is given the box's dictionaries, `I`'s; and a filter of it,
// or an enumeration of a `&mut` of it, stepped from its back (#2369).

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

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

fn reversed<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) {
    let src: Box<I> = Is::is(src);
    for (i, x) in src.enumerate().rev() {
        println!("reversed {i} {}", x.0);
    }
}

fn skipped_back<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) {
    let src: Box<I> = Is::is(src);
    for (i, x) in src.enumerate().rev().skip(1) {
        println!("skipped_back {i} {}", x.0);
    }
}

fn last_of<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) {
    let src: Box<I> = Is::is(src);
    let mut it = src.enumerate();
    println!("len {}", it.len());
    if let Some((i, x)) = it.next_back() {
        println!("last {i} {}", x.0);
    }
    println!("left {}", it.len());
    for (i, x) in it {
        println!("then {i} {}", x.0);
    }
}

fn folded<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) -> u32 {
    let src: Box<I> = Is::is(src);
    src.enumerate().rfold(0, |sum, (i, x)| sum * 10 + i as u32 + x.0)
}

fn kept_back<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) {
    let src: Box<I> = Is::is(src);
    for x in src.filter(|x| x.0 % 2 == 0).rev() {
        println!("kept_back {}", x.0);
    }
}

fn lent_back<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(src: impl Is<Box<I>>) {
    let mut src: Box<I> = Is::is(src);
    for (i, x) in src.by_ref().enumerate().rev().take(1) {
        println!("lent_back {i} {}", x.0);
    }
    for x in src {
        println!("lent_rest {}", x.0);
    }
}

fn main() {
    for n in [0, 1, 3] {
        reversed(Box::new(Every(n)));
        skipped_back(Box::new(Every(n)));
        last_of(Box::new(Every(n)));
        println!("folded {}", folded(Box::new(Every(n))));
        kept_back(Box::new(Every(n)));
        lent_back(Box::new(Every(n)));
    }
}

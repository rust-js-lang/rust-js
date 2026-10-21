// A map the crate steps from its back, made knowing it where it's made: one
// rust-js holds before it's stepped, as moving what may drop does,
// itertools' `PadUsing::rfold`; and one a variable holds (ADR 0545).
use std::iter::Fuse;

struct PadUsing<I, F> {
    iter: Fuse<I>,
    min: usize,
    filler: F,
}

impl<I, F> PadUsing<I, F>
where
    I: DoubleEndedIterator + ExactSizeIterator,
    F: FnMut(usize) -> I::Item,
{
    fn rfold<B, G: FnMut(B, I::Item) -> B>(self, mut init: B, mut f: G) -> B {
        init = (self.iter.len()..self.min).map(self.filler).rfold(init, &mut f);
        self.iter.rfold(init, f)
    }
}

// A map of an iterator given as it is that knows its length, counted.
struct Counted<I> {
    inner: I,
}

impl<I: ExactSizeIterator<Item = u32> + Clone> Counted<I> {
    fn doubled_len(&self) -> usize {
        self.inner.clone().map(|x| x * 2).len()
    }
}

// What may drop, which the crate has.
struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {}
}

fn last_square(n: u32) -> Option<u32> {
    let mut squares = (1..n).map(|x| x * x);
    squares.next_back()
}

fn main() {
    let _noisy = Noisy;
    let pad = PadUsing { iter: vec![1u32, 2].into_iter().fuse(), min: 4, filler: |i| i as u32 * 10 };
    println!("{:?}", pad.rfold(Vec::new(), |mut all, x| {
        all.push(x);
        all
    }));
    println!("{:?} {:?}", last_square(5), last_square(1));
    println!("{} {}", Counted { inner: vec![1, 2, 3].into_iter() }.doubled_len(), Counted { inner: 2..7 }.doubled_len());
}

//@ library-differs
// Generic code over `I: DoubleEndedIterator` asks the `nth_back` of what
// its caller gives it, the crate's own where it writes one, as std's does:
// through `by_ref()`, a `&mut I`, which passes `nth_back` on to `I`'s,
// and a `rev()`, whose `nth` is its source's `nth_back`, in both builds
// (#2134). And where `I` is reached only through another trait's method,
// by value or by `&mut`, as `impl Is<I>` or `impl GetMut<I>` give it: its
// own `nth_back` and, by value, its own `rfold`, where a `&mut I` folds by
// its `try_rfold`, the trait's default over `next_back` (#2139, #2147).
struct Refs<'a>(&'a [u32]);
impl<'a> Iterator for Refs<'a> {
    type Item = &'a u32;
    fn next(&mut self) -> Option<&'a u32> {
        println!("next");
        let (first, rest) = self.0.split_first()?;
        self.0 = rest;
        Some(first)
    }
}
impl<'a> DoubleEndedIterator for Refs<'a> {
    fn next_back(&mut self) -> Option<&'a u32> {
        println!("back");
        let (last, rest) = self.0.split_last()?;
        self.0 = rest;
        Some(last)
    }
    fn nth_back(&mut self, n: usize) -> Option<&'a u32> {
        println!("own nth_back {n}");
        let len = self.0.len();
        let item = len.checked_sub(n + 1).map(|i| &self.0[i]);
        self.0 = &self.0[..len.saturating_sub(n + 1)];
        item
    }
    fn rfold<B, F: FnMut(B, &'a u32) -> B>(self, init: B, _f: F) -> B {
        println!("own rfold");
        init
    }
}

trait GetMut<T> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
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

fn direct<'a, I: DoubleEndedIterator<Item = &'a u32>>(mut i: I) -> Option<&'a u32> {
    i.nth_back(1)
}

fn rev_nth<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Option<&'a u32> {
    i.rev().nth(1)
}

fn rev_skip<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Option<&'a u32> {
    i.rev().skip(1).next()
}

fn rev_step<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Vec<&'a u32> {
    i.rev().step_by(2).collect()
}

fn fuse_nth_back<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: I) -> Option<&'a u32> {
    i.fuse().nth_back(1)
}

fn by_ref_rev<'a, I: DoubleEndedIterator<Item = &'a u32>>(mut i: I) -> (Option<&'a u32>, Option<&'a u32>) {
    let first = i.by_ref().rev().nth(0);
    (first, i.next())
}

fn by_ref_nth_back<'a, I: DoubleEndedIterator<Item = &'a u32>>(mut i: I) -> Option<&'a u32> {
    i.by_ref().nth_back(1)
}

fn lent<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: &mut I) -> Option<&'a u32> {
    i.rev().nth(1)
}

fn lent_through<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: &mut impl GetMut<I>) -> Option<&'a u32> {
    let i = GetMut::get_mut(i);
    i.rev().nth(1)
}

fn lent_through_dyn<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: &mut dyn GetMut<I>) -> Option<&'a u32> {
    let i = GetMut::get_mut(i);
    i.rev().nth(1)
}

fn owned_through<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: impl Is<I>) -> Option<&'a u32> {
    let i = Is::is(i);
    i.rev().nth(1)
}

fn lent_through_folded<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: &mut impl GetMut<I>) -> u32 {
    let i = GetMut::get_mut(i);
    i.rev().fold(0, |a, b| a + b)
}

fn owned_through_folded<'a, I: DoubleEndedIterator<Item = &'a u32>>(i: impl Is<I>) -> u32 {
    let i = Is::is(i);
    i.rev().fold(0, |a, b| a + b)
}

fn main() {
    let items = [1, 2, 3, 4, 5];
    let mut r = Refs(&items);
    println!("{:?}", r.by_ref().rev().nth(0));
    println!("{:?}", direct(Refs(&items)));
    println!("{:?}", rev_nth(Refs(&items)));
    println!("{:?}", rev_skip(Refs(&items)));
    println!("{:?}", rev_step(Refs(&items)));
    println!("{:?}", fuse_nth_back(Refs(&items)));
    println!("{:?}", by_ref_rev(Refs(&items)));
    println!("{:?}", by_ref_nth_back(Refs(&items)));
    println!("{:?}", lent(&mut Refs(&items)));
    println!("{:?}", by_ref_rev(items.iter()));
    println!("{:?}", lent(&mut items.iter()));
    println!("{:?}", lent_through(&mut Refs(&items)));
    println!("{:?}", lent_through_dyn::<Refs>(&mut Refs(&items)));
    println!("{:?}", owned_through(Refs(&items)));
    println!("{:?}", lent_through_folded(&mut Refs(&items)));
    println!("{:?}", owned_through_folded(Refs(&items)));
    println!("{:?}", lent_through(&mut items.iter()));
    println!("{:?}", owned_through_folded(items.iter()));
}

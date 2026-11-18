//@ library-differs
// A `&mut` of the crate's iterator that writes its own `fold`, `count`
// and `rfold`, given to generic code as `&mut R` and passed on by value as
// its `J`, or lent by a type parameter's `by_ref()` to an adapter stepped
// from its back: each passes on what std's `&mut I` does (`PASSED_ON`), its
// folds by its `try_fold`s, here the trait's defaults over `next` and
// `next_back`, its `count` std's default, never the iterator's own (#2104).
// The iterator's own methods are its own where it's given by value. And
// a `&mut` given for a `&mut G` whose `G` isn't bound by `Iterator`, a
// blanket `GetMut<R>`'s, is the value it is, not lent.
struct Down(u32);

impl Iterator for Down {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, _f: F) -> B {
        println!("own fold");
        init
    }
    fn count(self) -> usize {
        println!("own count");
        99
    }
}

impl DoubleEndedIterator for Down {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }
    fn rfold<B, F: FnMut(B, u32) -> B>(self, init: B, _f: F) -> B {
        println!("own rfold");
        init
    }
}

fn rsum<J: DoubleEndedIterator<Item = u32>>(it: J) -> u32 {
    it.rfold(0, |a, b| a + b)
}

fn sum<J: Iterator<Item = u32>>(it: J) -> u32 {
    it.fold(0, |a, b| a + b)
}

fn counted<J: Iterator<Item = u32>>(it: J) -> usize {
    it.count()
}

fn through<R: DoubleEndedIterator<Item = u32>>(it: &mut R) -> (u32, u32, usize) {
    (rsum(&mut *it), sum(&mut *it), counted(it))
}

trait GetMut<T: ?Sized> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T: ?Sized> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn through_get<R: DoubleEndedIterator<Item = u32>, G: GetMut<R>>(g: &mut G) -> u32 {
    rsum(g.get_mut())
}

struct Held<R> {
    inner: R,
}

impl<R: DoubleEndedIterator<Item = u32>> Held<R> {
    fn total(&mut self) -> u32 {
        self.inner.by_ref().map(|x| x * 2).rfold(0, |a, b| a + b)
    }
    fn kept(&mut self) -> u32 {
        self.inner.by_ref().filter(|x| x % 2 == 0).rfold(0, |a, b| a + b)
    }
    fn counted(&mut self) -> usize {
        self.inner.by_ref().map(|x| x * 2).count()
    }
    fn rcounted(&mut self) -> usize {
        self.inner.by_ref().rev().count()
    }
    fn rfolded(&mut self) -> u32 {
        self.inner.by_ref().rev().fold(0, |a, b| a + b)
    }
    fn forward(&mut self) -> u32 {
        self.inner.by_ref().map(|x| x * 2).fold(0, |a, b| a + b)
    }
}

fn main() {
    println!("{:?}", through(&mut Down(4)));
    let mut d = Down(3);
    println!("{:?}", through(&mut d));
    println!("{}", Held { inner: Down(4) }.total());
    println!("{}", Held { inner: Down(4) }.forward());
    println!("{}", Held { inner: Down(5) }.kept());
    println!("{}", Held { inner: Down(4) }.counted());
    println!("{}", Held { inner: Down(4) }.rcounted());
    println!("{}", Held { inner: Down(4) }.rfolded());
    let mut c = Down(4);
    println!("{}", c.by_ref().map(|x| x * 2).rfold(0, |a, b| a + b));
    let mut e = Down(4);
    println!("{}", e.by_ref().rfold(0, |a, b| a + b));
    let mut f = Down(4);
    println!("{}", f.by_ref().fold(0, |a, b| a + b));
    let mut g = Down(4);
    println!("{}", g.by_ref().count());
    println!("{}", rsum(Down(2)));
    println!("{}", through_get::<Down, Down>(&mut Down(3)));
}

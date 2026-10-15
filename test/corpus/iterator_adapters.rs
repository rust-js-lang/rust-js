// A struct of a type parameter bound by `Iterator`, an adapter, steps it
// through the parameter's dictionary, `{ next }`, as it is: a range in
// place, the crate's own by its impl, a JS iterator by `$next` (ADR 0411).
struct Doubled<I> {
    inner: I,
}
impl<I: Iterator<Item = i32>> Iterator for Doubled<I> {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        self.inner.next().map(|x| x * 2)
    }
}
impl<I: Iterator<Item = i32>> Doubled<I> {
    fn new(inner: I) -> Self {
        Doubled { inner }
    }
    fn rest(self) -> i32 {
        self.inner.sum()
    }
}
fn doubled<I: Iterator<Item = i32>>(inner: I) -> Doubled<I> {
    Doubled { inner }
}
fn sum_doubled<J: Iterator<Item = i32>>(j: J) -> i32 {
    Doubled { inner: j }.sum()
}
// Given back, stepped through what holds it, and through itself.
fn skip_one<I: Iterator<Item = i32>>(mut d: Doubled<I>) -> Doubled<I> {
    d.next();
    d
}
fn take_two<I: Iterator<Item = i32>>(mut it: I) -> (i32, I) {
    let mut n = 0;
    for x in it.by_ref().take(2) {
        n += x;
    }
    (n, it)
}
// Of any items, one that looks like `None` too, boxed while it's stepped.
struct Pass<I>(I);
impl<I: Iterator> Pass<I> {
    fn all(self) -> Vec<I::Item> {
        self.0.collect()
    }
    fn shown(self) -> Vec<String>
    where
        I::Item: std::fmt::Debug,
    {
        let mut out = Vec::new();
        for x in self.0 {
            out.push(format!("{x:?}"));
        }
        out
    }
}
// A JS iterator where it's given only itself; as it is where it's in what's
// given.
struct Wrap<I>(I);
fn first<I: Iterator<Item = i32>>(i: I) -> Option<i32> {
    let mut w = Wrap(i);
    w.0.next()
}
fn second<I: Iterator<Item = i32>>(mut w: Wrap<I>) -> Option<i32> {
    w.0.next();
    w.0.next()
}
// Bound by a trait `Iterator` is a supertrait of, and given only itself: a
// JS iterator, whose dictionary is no `Iterator`'s.
pub trait Keyed: Iterator<Item = <Self as Keyed>::Key> {
    type Key;
}
impl Keyed for std::vec::IntoIter<i32> {
    type Key = i32;
}
fn sum_keyed<F: Keyed<Key = i32>>(f: F) -> i32 {
    f.fold(0, |a, b| a + b)
}
// Given on to generic code as a JS iterator, stepped by its impl's `next`.
fn sum_all<J: Iterator<Item = i32>>(j: J) -> i32 {
    j.sum()
}
fn total_of<I: Iterator<Item = i32>>(d: Doubled<I>, spare: I) -> (i32, I) {
    (sum_all(d), spare)
}
struct Countdown(i32);
impl Iterator for Countdown {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}
fn main() {
    let v: Vec<i32> = doubled(vec![1, 2, 3].into_iter()).collect();
    println!("{v:?}");
    let w: Vec<i32> = Doubled { inner: 0..4 }.collect();
    println!("{w:?}");
    let mut r = Doubled::new(5..9);
    println!("{:?} {} {}", r.next(), r.inner.start, r.inner.end);
    println!("{}", r.rest());
    let c: Vec<i32> = doubled(Countdown(3)).collect();
    println!("{c:?}");
    let nested: Vec<i32> = Doubled { inner: Doubled { inner: 1..3 } }.collect();
    println!("{nested:?}");
    println!("{}", sum_doubled(10..12));
    let mut back = doubled(Countdown(2));
    back.next();
    println!("{}", back.inner.0);
    let skipped = skip_one(Doubled { inner: 3..6 });
    let start = skipped.inner.start;
    println!("{} {:?}", start, skipped.collect::<Vec<_>>());
    let (n, rest) = take_two(10..15);
    println!("{} {} {}", n, rest.start, rest.end);
    let (m, left) = take_two(Countdown(4));
    println!("{} {}", m, left.0);
    println!("{:?}", Pass(vec![Some(1), None, Some(3)].into_iter()).all());
    println!("{:?}", Pass(vec![Some(1), None].into_iter()).shown());
    println!("{:?} {:?} {:?}", first(3..5), second(Wrap(7..10)), second(Wrap(Countdown(5))));
    println!("{}", sum_keyed(vec![11, 10, 1].into_iter()));
    let (t, spare) = total_of(Doubled { inner: 1..4 }, 7..8);
    println!("{} {}", t, spare.start);
}

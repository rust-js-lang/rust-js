// An iterator given to generic code, which takes a JS iterator where it
// steps one and its dictionary where it's given as it is (ADR 0411): a
// crate's iterator or a range adapted where it's given (`given_iterators`),
// alike by every route generic code is reached by, a call, a `fn` pointer,
// a function value given as an `F`, a `dyn Fn`, a closure, a trait's
// method through a `dyn` and a `Box<dyn Iterator>`; and one given as it
// is, returned or held in a struct, its caller's own (ADR 1115).
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Debug, Clone)]
struct Count {
    n: u32,
}
impl Iterator for Count {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }
}

fn id<I: Iterator<Item = u32>>(i: I) -> I {
    i
}
fn first<I: Iterator<Item = u32>>(mut i: I) -> (Option<u32>, I) {
    let f = i.next();
    (f, i)
}
struct Wrap<I> {
    inner: I,
}
fn wrapped<I: Iterator<Item = u32>>(w: Wrap<I>) -> u32 {
    w.inner.sum()
}
fn with<I: Iterator<Item = u32>, F: Fn(I) -> u32>(i: I, f: F) -> u32 {
    f(i)
}
trait Sum {
    fn sum_of(&self, it: Range<u32>) -> u32;
}
struct S;
impl Sum for S {
    fn sum_of(&self, it: Range<u32>) -> u32 {
        total(it)
    }
}
fn total<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.sum()
}
fn twice<I: Iterator<Item = u32> + Clone>(it: I) -> u32 {
    it.clone().sum::<u32>() + it.sum::<u32>()
}
fn boxed(it: Box<dyn Iterator<Item = u32>>) -> u32 {
    total(it)
}

fn by_shapes() {
    let r = id(1..4u32);
    println!("{:?} {} {}", r, r.start, r.len());
    let c = id(Count { n: 1 });
    println!("{:?}", c);
    let (f, rest) = first(5..8u32);
    println!("{f:?} {rest:?}");
    let (g, rest2) = first(Count { n: 0 });
    println!("{g:?} {rest2:?} {}", rest2.clone().count());
    println!(
        "{} {}",
        wrapped(Wrap { inner: 0..4u32 }),
        wrapped(Wrap { inner: Count { n: 0 } })
    );
    println!(
        "{} {}",
        with(0..4u32, |r: Range<u32>| r.len() as u32),
        with(Count { n: 0 }, |c: Count| c.n + 100)
    );
    let d: &dyn Sum = &S;
    println!("{}", d.sum_of(2..5));
    println!("{} {}", twice(0..3u32), twice(Count { n: 1 }));
    println!("{}", boxed(Box::new(0..4u32)));
    let p: fn(Range<u32>) -> Range<u32> = id::<Range<u32>>;
    let q = p(3..6);
    println!("{:?} {}", q, q.end);
}

fn summed<I: Iterator<Item = u32>>(it: I) -> u32 {
    it.sum()
}
fn listed<C: IntoIterator<Item = u32>>(c: C) -> Vec<u32> {
    c.into_iter().collect()
}
fn back<I: DoubleEndedIterator<Item = u32>>(mut it: I) -> Option<u32> {
    it.next_back()
}
fn sized<I: ExactSizeIterator<Item = u32>>(it: I) -> usize {
    it.len()
}
fn apply<A, F: Fn(A) -> u32>(f: F, a: A) -> u32 {
    f(a)
}

fn by_values() {
    // Direct calls.
    println!(
        "{} {} {:?} {:?} {}",
        summed(0..4u32),
        summed(Count { n: 0 }),
        listed(vec![5, 6]),
        back(1..4u32),
        sized(0..7u32)
    );
    // As fn pointers.
    let t: fn(Range<u32>) -> u32 = summed::<Range<u32>>;
    let c: fn(Count) -> u32 = summed::<Count>;
    let l: fn(Vec<u32>) -> Vec<u32> = listed::<Vec<u32>>;
    let s: fn(BTreeSet<u32>) -> Vec<u32> = listed::<BTreeSet<u32>>;
    let b: fn(Range<u32>) -> Option<u32> = back::<Range<u32>>;
    let z: fn(Range<u32>) -> usize = sized::<Range<u32>>;
    println!(
        "{} {} {:?} {:?} {:?} {}",
        t(0..4),
        c(Count { n: 0 }),
        l(vec![5, 6]),
        s([9, 8].into()),
        b(1..4),
        z(0..7)
    );
    // Given to generic code as an `F`.
    println!(
        "{} {} {}",
        apply(summed::<Range<u32>>, 0..4),
        apply(summed::<Count>, Count { n: 1 }),
        apply(|r: Range<u32>| sized(r) as u32, 2..5)
    );
    println!("{}", apply(|v: Vec<u32>| listed(v).len() as u32, vec![1, 2, 3]));
    // Through dyn.
    let d: &dyn Fn(Range<u32>) -> u32 = &summed::<Range<u32>>;
    println!("{}", d(0..5));
}

fn main() {
    by_shapes();
    by_values();
}

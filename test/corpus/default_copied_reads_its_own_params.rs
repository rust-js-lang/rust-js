//@ library-differs
// A trait's default copied into impls of no, one and two type parameters
// reads each fact of its own type parameters by the default's numbering,
// not the impl's terms its body is lowered in, which number them after the
// impl's own (#2273): `T`'s `Some(None)` stays boxed beside a `U` never
// `None`, a `U` given tuples is compared by its dictionary beside a `T`
// given numbers, and an `I: DoubleEndedIterator` or `ExactSizeIterator` is
// stepped from its back and asked its length, and a function JS calls too
// is called as its own parameters are given, as native Rust does.
use std::marker::PhantomData;

pub fn total<X: Iterator<Item = u32>>(items: X) -> u32 {
    items.fold(0, |sum, item| sum + item)
}

trait Probe {
    fn wrap<T, U>(&self, t: T, _u: U) -> Option<T> {
        Some(t)
    }

    fn both_rise<T: PartialOrd, U: PartialOrd>(&self, a: T, b: T, c: U, d: U) -> bool {
        a < b && c < d
    }

    fn last<U, I: DoubleEndedIterator<Item = u32>>(&self, _u: U, items: I) -> Option<u32> {
        items.rev().next()
    }

    fn left<U, I: ExactSizeIterator<Item = u32>>(&self, _u: U, items: I) -> usize {
        items.len()
    }

    fn sum_all<U, I: Iterator<Item = u32>>(&self, _u: U, items: I) -> u32 {
        total(items)
    }
}

struct Plain;
impl Probe for Plain {}

struct One<A>(PhantomData<A>);
impl<A> Probe for One<A> {}

struct Two<A, B>(PhantomData<(A, B)>);
impl<A, B> Probe for Two<A, B> {}

fn report(name: &str, probe: &impl Probe) {
    let got = probe.wrap(None::<u8>, 7u32);
    println!("{name}: {:?} {}", got, got.is_some());
    println!("{name}: {:?}", probe.wrap(Some(3u8), 'x'));
    println!("{name}: {}", probe.both_rise(1, 2, (1, 9), (1, 10)));
    println!("{name}: {}", probe.both_rise(2.5, 1.5, "a", "b"));
    println!("{name}: {:?}", probe.last((), vec![4u32, 5, 6].into_iter()));
    println!("{name}: {:?}", probe.last(0u8, [7u32, 8].into_iter().map(|x| x * 2)));
    println!("{name}: {}", probe.left("u", vec![1u32, 2, 3].into_iter().skip(1)));
    println!("{name}: {}", probe.sum_all(None::<u8>, 1..4u32));
}

fn main() {
    report("plain", &Plain);
    report("one", &One::<u8>(PhantomData));
    report("two", &Two::<u8, i16>(PhantomData));
}

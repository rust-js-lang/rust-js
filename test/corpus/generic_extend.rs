// Generic code extending a type parameter, `C: Extend<A>`, as either's
// Either does (ADR 0164).

use std::collections::BTreeSet;

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, A> Extend<A> for Either<L, R>
where
    L: Extend<A>,
    R: Extend<A>,
{
    fn extend<T: IntoIterator<Item = A>>(&mut self, iter: T) {
        match self {
            Either::Left(inner) => inner.extend(iter),
            Either::Right(inner) => inner.extend(iter),
        }
    }
}

fn fill<C: Extend<u32>>(c: &mut C, n: u32) {
    c.extend(0..n);
}

fn shout<C: Extend<char>>(c: &mut C) {
    c.extend("!?".chars());
}

fn main() {
    let mut a: Either<Vec<u32>, BTreeSet<u32>> = Either::Left(vec![9]);
    a.extend([1, 2, 3]);
    let mut b: Either<Vec<u32>, BTreeSet<u32>> = Either::Right(BTreeSet::new());
    b.extend(vec![5, 4, 5]);
    let mut s = String::from("x");
    fill(&mut Vec::new(), 2);
    let mut v = vec![7];
    fill(&mut v, 3);
    s.extend(['y', 'z']);
    shout(&mut s);
    if let Either::Left(l) = &a { println!("{l:?}"); }
    if let Either::Right(r) = &b { println!("{r:?}"); }
    println!("{v:?} {s}");
    // A `Vec` extended by another's items, copied as they're stepped.
    let mut sources = vec![1u32];
    let more = vec![2u32, 3];
    sources.extend(more.iter().copied());
    println!("{sources:?} {more:?}");
}

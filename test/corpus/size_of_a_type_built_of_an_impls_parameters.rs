//@ library-differs
// `size_of::<Self>()` and `align_of::<Self>()` of an impl over a type
// built of its parameters, whose layout depends on them, `Wrap<T>([T; 2])`,
// through a dictionary and called directly; a generic caller that relays
// it; a tuple of its parameters; and a generic function's `size_of` of a
// type built of its own: each its callers' layout, a built slot (#2287).
use std::mem::{align_of, size_of};

trait Sz {
    fn sz(&self) -> usize;
    fn al(&self) -> usize;
}

struct Wrap<T>([T; 2]);

impl<T> Sz for Wrap<T> {
    fn sz(&self) -> usize {
        size_of::<Self>()
    }
    fn al(&self) -> usize {
        align_of::<Self>()
    }
}

struct Pair<A, B>(A, B);

impl<A, B> Sz for Pair<A, B> {
    fn sz(&self) -> usize {
        size_of::<(A, [B; 3])>()
    }
    fn al(&self) -> usize {
        align_of::<Self>()
    }
}

struct G<T>(T);

impl<T> G<T> {
    fn me(&self) -> usize {
        size_of::<Self>()
    }
}

fn through<S: Sz>(s: &S) -> (usize, usize) {
    (s.sz(), s.al())
}

fn outer<U>(w: &Wrap<U>) -> (usize, usize) {
    through(w)
}

fn arr<T>() -> usize {
    size_of::<[T; 2]>() + align_of::<Option<T>>()
}

fn main() {
    println!("{} {}", through(&Wrap([1u32, 2])).0, Wrap([1u8, 2]).sz());
    println!("{:?} {:?}", through(&Wrap([1u32, 2])), outer(&Wrap([1u16, 2])));
    println!(
        "{:?} {:?}",
        through(&Pair(1u8, 2u64)),
        outer(&Wrap([(1u8, 2u32), (3, 4)]))
    );
    println!("{} {} {} {}", arr::<u64>(), arr::<u8>(), G(5u64).me(), G(1u8).me());
}

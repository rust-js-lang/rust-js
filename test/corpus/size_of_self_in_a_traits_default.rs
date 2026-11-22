//@ library-differs
// `size_of::<Self>()` and `align_of::<Self>()` in a trait's default, each
// impl's dictionary's: a constant of an impl over a concrete type, the
// layout its makers give of one over a type built of its parameters,
// `Wrap<T>([T; 2])`, through a dictionary and called directly, and a
// generic caller relaying it; and of a trait's own parameter, through a
// `dyn` and a generic caller, each impl's dictionary's, its callers given
// none of it (#2290).
use std::mem::{align_of, size_of};

trait Sz: Sized {
    fn sz(&self) -> usize;
    fn al(&self) -> usize {
        align_of::<Self>()
    }
    fn both(&self) -> (usize, usize) {
        (size_of::<Self>(), self.al() + self.sz())
    }
}

struct Wrap<T>([T; 2]);

impl<T> Sz for Wrap<T> {
    fn sz(&self) -> usize {
        size_of::<Self>()
    }
}

struct One(u16);

impl Sz for One {
    fn sz(&self) -> usize {
        1
    }
}

struct Pair<A, B>(A, B);

impl<A, B> Sz for Pair<A, B> {
    fn sz(&self) -> usize {
        0
    }
    fn al(&self) -> usize {
        align_of::<(A, B)>() * 10
    }
}

trait Conv<X> {
    fn tag(&self) -> u8;
    fn xs(&self) -> usize {
        size_of::<X>() * 10 + self.tag() as usize
    }
}

impl<T> Conv<u64> for Wrap<T> {
    fn tag(&self) -> u8 {
        1
    }
}

impl Conv<u64> for One {
    fn tag(&self) -> u8 {
        2
    }
}

impl Conv<(u8, u32)> for One {
    fn tag(&self) -> u8 {
        3
    }
}

fn via<X, C: Conv<X>>(c: &C) -> usize {
    c.xs()
}

fn dynamic(cs: &[&dyn Conv<u64>]) -> Vec<usize> {
    cs.iter().map(|c| c.xs()).collect()
}

fn through<S: Sz>(s: &S) -> (usize, (usize, usize)) {
    (s.al(), s.both())
}

fn outer<U>(w: &Wrap<U>) -> (usize, (usize, usize)) {
    through(w)
}

fn main() {
    println!("{:?} {:?}", through(&Wrap([1u32, 2])), Wrap([1u8, 2]).both());
    println!("{:?} {} {:?}", through(&One(2)), One(1).al(), outer(&Wrap([1u64, 2])));
    println!(
        "{:?} {:?}",
        through(&Pair(1u8, 2u32)),
        outer(&Wrap([(1u8, 2u16), (3, 4)]))
    );
    println!("{:?}", dynamic(&[&Wrap([1u8, 2]), &One(1)]));
    let boxed: Box<dyn Conv<(u8, u32)>> = Box::new(One(2));
    println!(
        "{} {} {} {}",
        via::<u64, _>(&Wrap([1u16, 2])),
        via::<(u8, u32), _>(&One(3)),
        via::<u64, _>(&One(4)),
        boxed.xs()
    );
}

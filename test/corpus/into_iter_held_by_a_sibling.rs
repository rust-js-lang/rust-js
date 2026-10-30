//@ library-differs
// An impl's `IntoIterator` bound that some of its methods' results hold as
// it is, `make`'s and `pair`'s `Pair<A::IntoIter>`, and others' don't,
// `first`'s, which gives `A` to `pair`: each is given `A` as its own
// signature holds it, and a trait's dictionary, `Make`'s, as its methods'
// (ADR 0552, #382).
pub struct Pair<T> {
    t: T,
}

pub trait Make {
    type Out;
    fn make(self) -> Self::Out;
}

pub struct Wrap<A>(A);

impl<A: IntoIterator> Make for Wrap<A> {
    type Out = Pair<A::IntoIter>;
    fn make(self) -> Pair<A::IntoIter> {
        Pair { t: self.0.into_iter() }
    }
}

impl<A: IntoIterator> Wrap<A> {
    pub fn pair(self) -> Pair<A::IntoIter> {
        Pair { t: self.0.into_iter() }
    }

    pub fn first(self) -> Option<A::Item> {
        self.pair().t.next()
    }
}

fn go<M: Make>(m: M) -> M::Out {
    m.make()
}

fn main() {
    let p = go(Wrap(vec![1u32, 2, 3]));
    println!("{:?}", p.t.collect::<Vec<_>>());
    let q = Wrap(vec![4u32, 5]).pair();
    println!("{:?}", q.t.collect::<Vec<_>>());
    println!("{:?}", Wrap(vec![6u32]).first());
    let r = Wrap(vec![7u32, 8]).make();
    println!("{:?}", r.t.collect::<Vec<_>>());
}

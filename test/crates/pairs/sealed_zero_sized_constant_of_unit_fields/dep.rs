//! A constant of a type no other crate can make, of no size, whose field
//! is a unit struct, `Pred(Eq)`, in a library's generic code a consumer
//! compiles from its optimized MIR (ADR 0914), as itertools'
//! `DedupPred2CoalescePred(DedupEq)` is: made of its fields' values and
//! branded as Rust makes it (#2403, ADR 1196).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eq;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pred<P>(P);

pub struct Pair<I> {
    iter: I,
    pred: Pred<Eq>,
}

pub fn pair<I: Iterator>(iter: I) -> Pair<I> {
    Pair { iter, pred: Pred(Eq) }
}

impl<I: Iterator> Pair<I> {
    pub fn shown(&self) -> String {
        format!("{:?} {}", self.pred, self.pred == Pred(Eq))
    }
}

impl<I: Iterator> Iterator for Pair<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

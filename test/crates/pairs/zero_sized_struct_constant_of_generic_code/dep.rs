//! A constant of a struct of no size, `Pred(Mode::Strict)`, in a library's
//! generic function a consumer compiles from its optimized MIR (ADR 0914):
//! its fields alone, as the same value made in code is, never a variant's
//! index before them, so `==` of the two is `true` (#2595).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pred<P>(P);

pub struct Pair<I> {
    iter: I,
    pred: Pred<Mode>,
}

pub fn pair<I: Iterator>(iter: I) -> Pair<I> {
    Pair {
        iter,
        pred: Pred(Mode::Strict),
    }
}

impl<I: Iterator> Pair<I> {
    pub fn shown(&self) -> String {
        let strict = match self.pred.0 {
            Mode::Strict => "strict",
        };
        format!("{:?} {} {}", self.pred, strict, self.pred == Pred(Mode::Strict))
    }
}

impl<I: Iterator> Iterator for Pair<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

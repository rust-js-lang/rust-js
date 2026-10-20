//! The API of a crate rustc builds that rust-js writes the JS of, as it
//! writes std's: memchr's searches (ADR 0529), over the byte arrays slices
//! are, whose indices are the slice's.

use super::recognition::Std;
use super::{Expr, FnCx};

/// One of memchr's searches: the first or last index of one of its needles,
/// or each, from either end.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MemchrOp {
    /// `memchr`, `memchr2` and `memchr3`, or of `back`, their `memrchr`s.
    Find { back: bool },
    /// Their iterators, `memchr_iter`, or of `back`, `memrchr_iter`, its
    /// `rev()`.
    Iter { back: bool },
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of memchr's: its needles, then the haystack, the runtime's
    /// helper given the haystack and the needles, `$memchr(text, [111])`.
    /// `None` if `known` is another.
    pub(in crate::lower) fn memchr_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Memchr(op) = known else {
            return None;
        };
        let mut given: Vec<Expr> = values.collect();
        let haystack = given.pop().expect("rustc checked the arguments");
        let needles = Expr::array(given);
        Some(match op {
            MemchrOp::Find { back: false } => Expr::call_helper("$memchr", vec![haystack, needles]),
            MemchrOp::Find { back: true } => Expr::call_helper("$memrchr", vec![haystack, needles]),
            MemchrOp::Iter { back } => {
                let it = Expr::call_helper("$memchrIter", vec![haystack, needles]);
                match back {
                    true => Expr::call_helper("$rev", vec![it]),
                    false => it,
                }
            }
        })
    }
}

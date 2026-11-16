//! What std's `&mut I` and `Box<I>` pass on to the iterator they hold:
//! the one catalog of it (#2000), which the lowering asks
//! (`Wrapper::passes`) and test/passed-on.test.ts holds to std's source in
//! rust-src and src/runtime/lent.js's `$passedOn` to.

use super::*;

/// A wrapper std's impls step the iterator it holds through: a `&mut I` of
/// a sized `I` or of an unsized one, a `dyn`'s, and a `Box<I>` of each.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Wrapper {
    MutRefSized,
    MutRefUnsized,
    BoxSized,
    BoxUnsized,
}

/// How a wrapper's method reaches its iterator's: as the iterator's own
/// method; by its `try_fold` or `try_rfold`, never stopped, a sized `&mut
/// I`'s `fold` and `rfold` (`IteratorRefSpec`); or not at all, the trait's
/// default over `next` and `next_back`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Passed {
    Own,
    ByTry,
    Default,
}

/// What each wrapper passes on, by std's method name, in `Wrapper`'s order
/// (`&mut I` of a sized and an unsized `I`, `Box<I>` of each): std's own
/// source, core/src/iter/traits/{iterator,double_ended,exact_size}.rs's
/// `&mut I` impls and their `RefSpec`s, and alloc/src/boxed/iter.rs's
/// `Box<I>` impls and `BoxIter`. The one catalog (#2000): the lowering asks
/// `Wrapper::passes`, and test/passed-on.test.ts holds this to rust-src and
/// src/runtime/lent.js's `$passedOn` to this.
const PASSED_ON: [(&str, [Passed; 4]); 15] = {
    use Passed::{ByTry, Default, Own};
    [
        ("next", [Own, Own, Own, Own]),
        ("size_hint", [Own, Own, Own, Own]),
        ("advance_by", [Own, Own, Default, Default]),
        ("nth", [Own, Own, Own, Own]),
        ("fold", [ByTry, Default, Default, Default]),
        ("try_fold", [Own, Default, Default, Default]),
        ("last", [Default, Default, Own, Default]),
        ("count", [Default, Default, Default, Default]),
        ("next_back", [Own, Own, Own, Own]),
        ("advance_back_by", [Own, Own, Default, Default]),
        ("nth_back", [Own, Own, Own, Own]),
        ("rfold", [ByTry, Default, Default, Default]),
        ("try_rfold", [Own, Default, Default, Default]),
        ("len", [Own, Own, Own, Own]),
        ("is_empty", [Own, Own, Own, Own]),
    ]
};

impl Wrapper {
    /// The wrapper `ty` is of the iterator it holds, a `&mut` or a `Box`, or
    /// none.
    pub(in crate::lower) fn of<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
    ) -> Option<Wrapper> {
        let (held, boxed) = match *ty.kind() {
            ty::Ref(_, to, ty::Mutability::Mut) => (to, false),
            _ => (ty.boxed_ty()?, true),
        };
        Some(match (boxed, held.is_sized(tcx, typing_env)) {
            (false, true) => Wrapper::MutRefSized,
            (false, false) => Wrapper::MutRefUnsized,
            (true, true) => Wrapper::BoxSized,
            (true, false) => Wrapper::BoxUnsized,
        })
    }

    /// How its `method`, std's name, reaches its iterator's: the trait's
    /// default where std's impl doesn't name it.
    pub(in crate::lower) fn passes(self, method: &str) -> Passed {
        PASSED_ON
            .iter()
            .find(|(name, _)| *name == method)
            .map_or(Passed::Default, |(_, of)| of[self as usize])
    }

    /// Whether it's a `Box`'s, which owns what it holds; a `&mut`'s lends it.
    pub(in crate::lower) fn owns(self) -> bool {
        matches!(self, Wrapper::BoxSized | Wrapper::BoxUnsized)
    }

    /// The methods it passes on to its iterator's own, std's names.
    pub(in crate::lower) fn own_methods(self) -> impl Iterator<Item = &'static str> {
        PASSED_ON
            .iter()
            .filter(move |(_, of)| of[self as usize] == Passed::Own)
            .map(|(name, _)| *name)
    }
}

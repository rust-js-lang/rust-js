//! What a slice's `position` shows of the index it finds (rd316): where
//! `xs.iter().position(f)` is `Some(i)`, `i` is an index of `xs`, below
//! its length, as it stays till `xs` changes. The `Option`'s range is its
//! index's, `Some`'s, so a variable that holds it, `let index =
//! xs.iter().position(f)`, holds it, and a pattern `Some(i)` of it binds
//! `i` to it: `xs.remove(i)` is then `xs.splice(i, 1)`.

use super::*;
use crate::lower::recognition::{is_iterator_position, is_slice_iter};

/// The slice whose index `e`, an `Option`, holds where it's `Some`: of
/// `xs.iter().position(f)`, the first item `f` finds, `xs`.
fn found_index_of(tcx: TyCtxt<'_>, thir: &Thir<'_>, e: ExprId) -> Option<ExprId> {
    let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
        return None;
    };
    let (def_id, _) = fn_def(thir[strip(thir, fun)].ty)?;
    if !is_iterator_position(tcx, def_id) {
        return None;
    }
    let ExprKind::Borrow { arg: it, .. } = thir[strip(thir, *args.first()?)].kind else {
        return None;
    };
    let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, it)].kind else {
        return None;
    };
    let (def_id, _) = fn_def(thir[strip(thir, fun)].ty)?;
    is_slice_iter(tcx, def_id).then_some(*args.first()?)
}

impl<'tcx> Walk<'_, 'tcx> {
    /// Of `e`, a slice's `position`, just walked: the range of the index
    /// it finds, from 0 to one below the slice's length, at most `u32`'s
    /// most, as a slice's length is (`length_most_of`).
    pub(super) fn found_index(&self, e: ExprId) -> Option<Range> {
        let slice = self.slice(found_index_of(self.tcx, self.thir, e)?)?;
        let below = Below::LENGTH.offset(-1);
        Some(Range {
            lo: 0,
            hi: below.most(u32::MAX as i128),
            below: Some((slice, below)),
        })
    }

    /// Of `pattern`, `Some(i)`, where what it tests, just walked, is an
    /// `Option` a slice's `position` gave, `found`, its index's range: `i`
    /// in it.
    pub(super) fn found_index_bound(&self, pattern: &Pat<'tcx>, found: Option<Range>, known: &mut Option<Known>) {
        let PatKind::Variant { ref subpatterns, .. } = pattern.kind else {
            return;
        };
        if let [index] = &subpatterns[..]
            && present::some_pattern(self.tcx, pattern)
            && let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, _),
                subpattern: None,
                ..
            } = index.pattern.kind
            && !self.volatile.contains(&var)
            && let (Some(found), Some(k)) = (found, known.as_mut())
        {
            k.vars.insert(var, found);
        }
    }
}

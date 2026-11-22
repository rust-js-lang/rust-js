//! What an iterator crossing the JS boundary is, the one rule its three
//! readers ask (#1993, #1994, #1999): a JS-facing function's type
//! parameter bound by `Iterator`, `DoubleEndedIterator` or
//! `ExactSizeIterator`, which a JS caller gives, and an `impl Trait` of
//! them such a function gives back. Of `Iterator` alone, any of JS's
//! iterables, `Iterable<T>`. Of `DoubleEndedIterator` or
//! `ExactSizeIterator`, a JS iterator that knows its back or its length,
//! `back()` and `len()` (ADRs 0429, 0456), or an array, which the
//! function makes its `$iter`: what it gives back is that iterator, so one
//! function's result is another's argument.

use super::*;
use crate::lower::recognition::{StdItem, is_double_ended, is_exact_size, is_std_def};
use rustc_middle::ty::TyCtxt;

/// An iterator at the JS boundary: what it gives, `item`, and whether it
/// knows its back, `back`, and its length, `len`.
// Flags, not a state: its back and its length are two facts of its bounds,
// `DoubleEndedIterator` and `ExactSizeIterator`, either with the other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::lower) struct BoundaryIterator<'tcx> {
    pub(in crate::lower) item: Ty<'tcx>,
    pub(in crate::lower) back: bool,
    pub(in crate::lower) len: bool,
}

/// Of `function`'s type parameter `param`, the iterator a JS caller gives
/// for it, as its bounds say; none of one generic code is given as it is
/// (ADR 0411), the crate's own value, which no JS caller has.
pub(in crate::lower) fn of_param<'tcx>(
    tcx: TyCtxt<'tcx>,
    function: DefId,
    param: ty::ParamTy,
) -> Option<BoundaryIterator<'tcx>> {
    if given_as_it_is(tcx, function, param) {
        return None;
    }
    let env = ty::TypingEnv::post_analysis(tcx, function);
    let of = |ty: Ty<'tcx>| crate::lower::generic_facts::type_parameter_of(ty) == Some(param.index);
    bounded(tcx, of, env.param_env.caller_bounds())
}

/// Of an `impl Trait` a function gives, `alias`, the iterator its JS gives
/// back, as its bounds say: `upto(n) -> impl DoubleEndedIterator`'s.
pub(in crate::lower) fn of_opaque<'tcx>(tcx: TyCtxt<'tcx>, alias: ty::AliasTy<'tcx>) -> Option<BoundaryIterator<'tcx>> {
    let ty::Opaque { def_id } = alias.kind else {
        return None;
    };
    let of = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Alias(_, a) if a.kind == ty::Opaque { def_id });
    let clauses = (tcx
        .explicit_item_bounds(def_id)
        .iter_instantiated_copied(tcx, alias.args))
    .map(|bound| bound.skip_normalization().0);
    bounded(tcx, of, clauses)
}

/// What a JS-facing function gives back of an `impl Iterator`: the
/// iterator at the boundary and the type its body reveals.
#[derive(Clone, Copy, Debug)]
pub(in crate::lower) struct Returned<'tcx> {
    pub(in crate::lower) boundary: BoundaryIterator<'tcx>,
    pub(in crate::lower) revealed: Ty<'tcx>,
}

impl<'tcx> Returned<'tcx> {
    /// Whether its JS isn't the boundary's as it is, so the function's own
    /// JS converts it and the crate's own Rust calls a copy that doesn't:
    /// one that knows its back or its length, or of a type whose value is
    /// no JS iterator, a range's `{ start, end }`; never std's compiled
    /// struct asked neither, a JS iterator already (#1999).
    pub(in crate::lower) fn converts(&self, tcx: TyCtxt<'tcx>) -> bool {
        self.boundary.back
            || self.boundary.len
            || !crate::lower::recognition::is_compiled_std_iterator(tcx, self.revealed)
    }
}

/// What `function` gives back, an `impl Iterator` or one of its
/// extensions, at the JS boundary (`Returned`); none of another.
pub(in crate::lower) fn returned<'tcx>(tcx: TyCtxt<'tcx>, function: DefId) -> Option<Returned<'tcx>> {
    // Its late-bound regions erased, `odd_squares(&self)`'s `'_`: no bound
    // variable escapes into what traits are asked of.
    let output = tcx
        .instantiate_bound_regions_with_erased(tcx.fn_sig(function).instantiate_identity().skip_normalization())
        .output();
    let ty::Alias(_, alias) = output.kind() else {
        return None;
    };
    let boundary = of_opaque(tcx, *alias)?;
    let env = ty::TypingEnv::post_analysis(tcx, function);
    let revealed = tcx
        .try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(output))
        .unwrap_or(output);
    Some(Returned { boundary, revealed })
}

/// Of what `of` holds of, what its `clauses` say: bound by `Iterator`, or
/// one of its extensions, each of which is one, and its `Item`.
fn bounded<'tcx>(
    tcx: TyCtxt<'tcx>,
    of: impl Fn(Ty<'tcx>) -> bool,
    clauses: impl Iterator<Item = ty::Clause<'tcx>>,
) -> Option<BoundaryIterator<'tcx>> {
    let (mut iterator, mut back, mut len, mut item) = (false, false, false, None);
    for clause in clauses {
        if let Some(bound) = clause.as_trait_clause()
            && of(bound.skip_binder().self_ty())
        {
            let id = bound.skip_binder().def_id();
            iterator |= is_std_def(tcx, id, StdItem::Iterator) || is_double_ended(tcx, id) || is_exact_size(tcx, id);
            back |= is_double_ended(tcx, id);
            len |= is_exact_size(tcx, id);
        }
        if let Some(projection) = clause.as_projection_clause()
            && of(projection.skip_binder().self_ty())
            && tcx
                .trait_of_assoc(projection.skip_binder().def_id())
                .is_some_and(|t| is_std_def(tcx, t, StdItem::Iterator))
            && let Some(given) = projection.skip_binder().term.as_type()
        {
            item = Some(given);
        }
    }
    iterator.then_some(BoundaryIterator { item: item?, back, len })
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// What the function being lowered, JS-facing and giving back an `impl
    /// Iterator`, gives JS of `value`, its revealed type's: the JS iterator
    /// a JS caller steps, knowing its back and its length where its bounds
    /// say, `$iterator(range, $rangeNext, .., $rangeNextBack, len)` of a
    /// range's `{ start, end }`; spread by `[...it]`, its `back()` and
    /// `len()` public API (#1999).
    pub fn returned_to_js(&mut self, value: Expr, span: Span) -> R<Expr> {
        let Some(Returned { boundary, revealed }) = returned(self.tcx, self.item) else {
            return Ok(value);
        };
        match (boundary.back, boundary.len) {
            (false, false) => self.as_js_iterator(value, revealed, span),
            known => self.as_knowing(value, revealed, known, span),
        }
    }
}

//! What a caller gives generic code that only the caller knows: a clone,
//! a drop of a type parameter, a boxed iterator's steps, a derived
//! constructor's first field, an `Error` impl's `TypeId`, and whether a
//! source is fused or `TrustedLen`. Split from generic_facts.rs by
//! concept, as a file is at most 1500 lines.

use super::super::FnCx;
use rust_js_program::js::Expr;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `source`, an iterator only a caller knows, is a
    /// `FusedIterator`, as its callers say: of each type parameter what it
    /// is fused by is of, `I`'s of a `Map<I, F>` (`held_facts`), its caller's
    /// `IFused`, each together; `None` where one isn't given, a trait's
    /// method's or a library's, which only its callers could say (#2242).
    pub(crate) fn callers_fused(&self, source: Ty<'tcx>) -> Option<Expr> {
        let held = crate::recognition::held_facts(self.tcx, self.typing_env, source, crate::TypeFact::Fused);
        let mut fused: Option<Expr> = None;
        for (of, _) in held {
            let ty::Param(param) = of.kind() else {
                return None;
            };
            let given = self.fused_fact(param.index)?;
            fused = Some(match fused {
                Some(both) => Expr::bin(rust_js_program::js::Op::And, both, given),
                None => given,
            });
        }
        fused
    }
}

/// Whether `imp`, an `Error` impl, gives its dictionary its type's
/// `TypeId`: one of a type of no type parameter. A generic one's, whose key
/// its caller's types would make, gives none, and a downcast to a type it's
/// of is refused (ADR 0331, amended).
pub(crate) fn error_impl_has_type_id(tcx: rustc_middle::ty::TyCtxt<'_>, imp: DefId) -> bool {
    !tcx.type_of(imp).instantiate_identity().skip_normalization().has_param()
}

/// Whether `ty`, what a `Box<I>` holds, is stepped by a dictionary its
/// caller gives, which may carry the crate's own steps, `fold` or `count`,
/// that std's `Box<I>` never calls: a type only its bounds know, a type
/// parameter or an associated type of one (#1611). No default of the box
/// passes on to it (`passed_through_box`).
pub(crate) fn boxed_steps_given<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
) -> bool {
    crate::recognition::is_unknown_where(tcx, ty, typing_env)
}

/// Whether a clone of `ty` is what its caller gives: of a type parameter,
/// a `dyn`'s or an opaque type's, which may run any code, so a clone of it
/// shows (`clone_shows`, #1630).
pub(crate) fn clone_given_by_caller(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Param(_) | ty::Dynamic(..) | ty::Alias(..))
}

/// Whether a caller gives function `DefId`'s type parameter of an index
/// one whose drop runs, `drop_params`' answer (ADR 0098).
pub(crate) type GivenADrop<'a> = dyn Fn(DefId, u32) -> bool + 'a;

/// Whether `item` is a type parameter of `owner`'s function that a caller
/// gives a value with a destructor, so a drop (`drop_params`', ADR 0098):
/// `given` answers it by the function and the parameter's index. Of a
/// library's, whose callers it can't see, `given` is `None`, and any may
/// be, where `params_may_drop` (ADR 0521, amended by #580).
pub(crate) fn given_a_drop_of<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    owner: DefId,
    item: Ty<'tcx>,
    (params_may_drop, given): (bool, Option<&GivenADrop<'_>>),
) -> bool {
    params_may_drop
        && matches!(item.kind(), ty::Param(param)
            if given.is_none_or(|given| given(tcx.typeck_root_def_id(owner), param.index)))
}

/// Whether a value of `ty`, a struct or an enum, has a variant with a field
/// of a type parameter's before another field, which a derived `Clone` or
/// `Default` makes first, and drops as a later one panics (#1517).
pub(crate) fn makes_a_param_field_first<'tcx>(tcx: rustc_middle::ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, args) = ty.kind() else { return false };
    adt.variants().iter().any(|variant| {
        let fields = &variant.fields.raw;
        (fields.iter().rev().skip(1)).any(|field| field.ty(tcx, args).skip_normalization().has_param())
    })
}

/// Of a collect into a `Vec` of `iter`, an iterator built of type
/// parameters, what says whether it's `TrustedLen` that only its callers
/// know (#1614): `vec_collect_trusted_facts`'s, its parts rustc says here
/// asked of no caller, one that's `TrustedLen` saying nothing and one that
/// isn't, a `Chain`'s side, that it's never one. None of one known by its
/// bounds, whose `IVecCollect` says it, nor of one of no type parameter.
pub(crate) fn trusted_len_asked<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    iter: Ty<'tcx>,
) -> Vec<(Ty<'tcx>, crate::recognition::TypeFact)> {
    use crate::recognition::{TypeFact, is_trusted_len, known_by_bounds, vec_collect_trusted_facts};
    if !iter.has_param() || known_by_bounds(iter) {
        return Vec::new();
    }
    let facts = vec_collect_trusted_facts(tcx, typing_env, iter);
    let never = |&(part, fact): &(Ty<'tcx>, TypeFact)| {
        fact == TypeFact::TrustedLen && !part.has_param() && !is_trusted_len(tcx, typing_env, part)
    };
    if facts.iter().any(never) {
        return Vec::new();
    }
    facts.into_iter().filter(|&(part, _)| part.has_param()).collect()
}

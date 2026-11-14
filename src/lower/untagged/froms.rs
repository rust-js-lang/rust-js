//! A `From` into an untagged enum: which calls of one are the value
//! they're given, as its variant is, and what a binding crate's `from` must
//! be so (ADR 0214, amended, #1775). Split from untagged.rs by concept, as
//! a file is at most 1,350 lines.

use super::super::recognition::{Recognition, StdItem, conversion_target};
use super::untagged_adt;
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt};
use rustc_span::def_id::DefId;

impl<'tcx> Recognition<'_, 'tcx> {
    /// Whether a call of `def_id`, of `args`, runs a `From` into an untagged
    /// enum each call of which is the value it's given
    /// (`from_gives_its_argument`): `Src::from(s)`, the impl's `from` it
    /// resolves to, or `s.into()`, the `from` its `Into` runs, a binding
    /// crate's too, `BlobPart::from(chunk)`, whose body rust-js has none of.
    pub(in crate::lower) fn calls_from_giving_its_argument(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
        let from = match self.resolve_into(def_id, args) {
            Some((_, _, implementation)) => Some(implementation),
            None => (super::super::items::resolve_instance_in(self.tcx, self.typing_env, def_id, args)
                .ok()
                .flatten())
            .map(|instance| instance.def_id()),
        };
        from.is_some_and(|from| from_gives_its_argument(self.foreign, from))
    }
}

/// Whether `def_id` is the `from` of a `From` into an untagged enum,
/// `impl From<&str> for Src`, whatever its body does.
fn is_untagged_from(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.def_kind(def_id) == DefKind::AssocFn
        && tcx.trait_impl_of_assoc(def_id).is_some()
        && conversion_target(tcx, def_id, ty::GenericArgs::identity_for_item(tcx, def_id))
            .is_some_and(|target| untagged_adt(tcx, target).is_some())
}

/// Whether each call of `def_id`, a `From` into an untagged enum, is the
/// value it's given, as its variant is (ADR 0214, amended, #1775): the one
/// answer lowering's calls, and what may run across one, ask. The crate's
/// own whose body is the variant of its argument (`gives_its_variant`); a
/// library's its contract lists among what gives its argument, its own
/// compile having asked this (`Foreign::gives_its_argument`); or one of a
/// crate rust-js writes no JS of, a binding crate's, `BlobPart::from(chunk)`,
/// which its `--rustc` compile holds to its variant
/// (`binding_froms_give_their_variants`). Any other's body is lowered, and
/// called, as any `From`'s is: `Detail::Text(value.to_owned())`.
pub(in crate::lower) fn from_gives_its_argument(
    foreign: &super::super::library::Foreign<'_, '_>,
    def_id: DefId,
) -> bool {
    let tcx = foreign.tcx();
    is_untagged_from(tcx, def_id)
        && match def_id.as_local() {
            Some(local) => gives_its_variant(tcx, local),
            None if foreign.is_library(def_id.krate) => foreign.gives_its_argument(def_id),
            None => true,
        }
}

/// Whether `def_id`, the crate's `from` of a `From` into an untagged enum,
/// is the variant of its argument, `Src::Text(s)`, read of its HIR, as a
/// crate `--rustc` compiles has no THIR: what its call is, the value
/// itself (`from_gives_its_argument`), as the variant is.
pub(in crate::lower) fn gives_its_variant(tcx: TyCtxt<'_>, local: rustc_span::def_id::LocalDefId) -> bool {
    variant_of_its_argument(tcx, local, false)
}

/// Whether `local`'s body is the variant of its argument, or, `boxed`, of
/// its argument's `Box::new`, a callback a binding takes as a `Box<dyn Fn>`,
/// react's `Ref::Callback(Box::new(callback))`, which JS has as the
/// function (ADR 0384).
fn variant_of_its_argument(tcx: TyCtxt<'_>, local: rustc_span::def_id::LocalDefId, boxed: bool) -> bool {
    let def_id = local.to_def_id();
    let Some(target) = (is_untagged_from(tcx, def_id))
        .then(|| conversion_target(tcx, def_id, ty::GenericArgs::identity_for_item(tcx, def_id)))
        .flatten()
    else {
        return false;
    };
    let Some((adt, _)) = untagged_adt(tcx, target) else {
        return false;
    };
    let body = tcx.hir_body_owned_by(local);
    let [param] = body.params else { return false };
    let mut value = body.value;
    while let hir::ExprKind::Block(block, _) = value.kind
        && block.stmts.is_empty()
        && let Some(inner) = block.expr
    {
        value = inner;
    }
    let typeck = tcx.typeck(local);
    match value.kind {
        hir::ExprKind::Call(callee, [arg]) => {
            let ctor = match callee.kind {
                hir::ExprKind::Path(ref qpath) => typeck.qpath_res(qpath, callee.hir_id),
                _ => Res::Err,
            };
            let of_adt = matches!(ctor, Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), id)
                if tcx.parent(tcx.parent(id)) == adt.did());
            let given = |arg: &hir::Expr<'_>| {
                matches!(arg.kind, hir::ExprKind::Path(hir::QPath::Resolved(None, path))
                    if path.res == Res::Local(param.pat.hir_id))
            };
            let boxed_given = || match arg.kind {
                hir::ExprKind::Call(new, [inner]) => {
                    let new = match new.kind {
                        hir::ExprKind::Path(ref qpath) => typeck.qpath_res(qpath, new.hir_id).opt_def_id(),
                        _ => None,
                    };
                    new.is_some_and(|new| super::super::recognition::is_std_def(tcx, new, StdItem::BoxNew))
                        && given(inner)
                        && typeck.expr_ty_adjusted(arg).boxed_ty().is_some_and(|held| {
                            matches!(held.kind(), ty::Dynamic(predicates, ..)
                                if super::super::recognition::is_fn_dyn(tcx, predicates))
                        })
                }
                _ => false,
            };
            of_adt && (given(arg) || (boxed && boxed_given()))
        }
        _ => false,
    }
}

/// Whether each `From` into an untagged enum of a crate `--rustc` compiles,
/// a binding crate's, is the variant of its argument: rust-js writes no JS
/// of the crate, so a call of its `from` is the value it's given
/// (`from_gives_its_argument`), and one whose body does more is refused
/// where it's written, its body never run.
pub fn binding_froms_give_their_variants(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    for local in tcx.hir_crate_items(()).definitions() {
        if is_untagged_from(tcx, local.to_def_id()) && !variant_of_its_argument(tcx, local, true) {
            let target = conversion_target(tcx, local.to_def_id(), ty::GenericArgs::identity_for_item(tcx, local));
            let name = target
                .and_then(|target| untagged_adt(tcx, target))
                .map(|(adt, _)| tcx.item_name(adt.did()));
            let name = name.map_or_else(String::new, |name| name.to_string());
            tcx.dcx().span_err(
                tcx.def_span(local),
                format!(
                    "rust-js: a `From` into the untagged enum `{name}` of a crate rust-js writes no JS of, a binding crate's, \
                     is its variant of its argument, `{name}::Variant(value)`: its call is the value itself (ADR 0214, amended)"
                ),
            );
            valid = false;
        }
    }
    valid
}

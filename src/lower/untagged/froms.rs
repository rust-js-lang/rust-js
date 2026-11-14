//! A `From` into an untagged enum: which calls of one are the value
//! they're given, as its variant is, and what its `from` must be so
//! (ADR 0214). Split from untagged.rs by concept, as a file is at most
//! 1,350 lines.

use super::super::recognition::{Recognition, conversion_target};
use super::untagged_adt;
use rustc_hir as hir;
use rustc_hir::def::{CtorKind, CtorOf, DefKind, Res};
use rustc_middle::ty::{self, GenericArgsRef, TyCtxt};
use rustc_span::def_id::DefId;

impl<'tcx> Recognition<'_, 'tcx> {
    /// A `From` into an untagged enum, or `into()` to one: the trait's method,
    /// or the crate's `from` a call of it resolves to. Each is the value
    /// itself, as its `from` is the variant of its argument, which is checked
    /// where it's declared.
    pub(in crate::lower) fn converts_to_untagged(&self, def_id: DefId, args: GenericArgsRef<'tcx>) -> bool {
        conversion_target(self.tcx, def_id, args).is_some_and(|target| untagged_adt(self.tcx, target).is_some())
    }
}

/// Whether `def_id` is the `from` of a `From` into an untagged enum, which
/// is the value it's given where it's called (ADR 0214), and so runs
/// nothing, its body the variant of its argument (`untagged_from_misuse`):
/// webapi's `BlobPart::from(chunk)` too, whose body rust-js doesn't see.
pub(crate) fn is_untagged_from(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.def_kind(def_id) == DefKind::AssocFn
        && tcx.trait_impl_of_assoc(def_id).is_some()
        && conversion_target(tcx, def_id, ty::GenericArgs::identity_for_item(tcx, def_id))
            .is_some_and(|target| untagged_adt(tcx, target).is_some())
}

/// A `From` into an untagged enum is the value itself where it's called
/// (ADR 0214), so its `from` must be the variant of its argument,
/// `Src::Text(s)`: what else it did, a call that's the value would skip.
pub(super) fn untagged_from_misuse(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    if !is_untagged_from(tcx, def_id) {
        return None;
    }
    let target = conversion_target(tcx, def_id, ty::GenericArgs::identity_for_item(tcx, def_id))?;
    let (adt, _) = untagged_adt(tcx, target)?;
    let local = def_id.as_local()?;
    let body = tcx.hir_body_owned_by(local);
    let param = body.params.first()?;
    let mut value = body.value;
    while let hir::ExprKind::Block(block, _) = value.kind
        && block.stmts.is_empty()
        && let Some(inner) = block.expr
    {
        value = inner;
    }
    let typeck = tcx.typeck(local);
    let variant_of_param = match value.kind {
        hir::ExprKind::Call(callee, [arg]) => {
            let ctor = match callee.kind {
                hir::ExprKind::Path(ref qpath) => typeck.qpath_res(qpath, callee.hir_id),
                _ => Res::Err,
            };
            let of_adt = matches!(ctor, Res::Def(DefKind::Ctor(CtorOf::Variant, CtorKind::Fn), id)
                if tcx.parent(tcx.parent(id)) == adt.did());
            let of_param = matches!(arg.kind, hir::ExprKind::Path(hir::QPath::Resolved(None, path))
                if path.res == Res::Local(param.pat.hir_id));
            of_adt && of_param
        }
        _ => false,
    };
    (!variant_of_param).then(|| {
        let name = tcx.item_name(adt.did());
        format!(
            "a `From` into the untagged enum `{name}` is its variant of its argument, `{name}::Variant(value)`, as rust-js makes it the value itself (ADR 0214)"
        )
    })
}

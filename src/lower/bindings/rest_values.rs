//! Rest arguments as a value, `&dyn js::Rest<T>` (ADRs 0667, 0851): which
//! type is one, and what it's read as, the array JS's rest parameter is.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TyCtxt, Upcast};
use rustc_span::def_id::DefId;
use rustc_span::{DUMMY_SP, Symbol};

/// Whether `ty` is rest arguments as a value, `&dyn js::Rest<T>`, of the
/// trait marked `#[rust_js::rest]` (ADR 0667): spread where it's given, to
/// a variadic binding or to a `fn` pointer whose last parameter it is.
pub(in crate::lower) fn is_rest_value(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("rest")];
    match ty.kind() {
        ty::Ref(_, inner, _) => match inner.kind() {
            ty::Dynamic(traits, ..) => traits
                .principal_def_id()
                .is_some_and(|t| tcx.get_attrs_by_path(t, &path).next().is_some()),
            _ => false,
        },
        _ => false,
    }
}

/// What rest arguments as a value, `ty`, a `&dyn js::Rest<T>`, are read as:
/// the slice their `Deref` gives, `[Option<&Unknown>]` of any value's, which
/// JS's rest parameter is the array of (ADR 0851); `[T]` of a type
/// parameter's, which `js::RestItem` reads as itself as it does any sized
/// type (#576), each parameter sized, as Rust's implicit bound has it. `None`
/// of another type, or of rest arguments of a `T` no `js::RestItem` reads.
pub(in crate::lower) fn rest_items<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Ref(_, rest, _) = ty.kind() else { return None };
    if !is_rest_value(tcx, ty) {
        return None;
    }
    let target = tcx.lang_items().deref_target()?;
    let rest = tcx.erase_and_anonymize_regions(*rest);
    let projection = Ty::new_projection(tcx, ty::IsRigid::No, target, [rest]);
    let sized = tcx.require_lang_item(LangItem::Sized, DUMMY_SP);
    let clauses = (rest.walk())
        .filter_map(|arg| arg.as_type())
        .filter(|t| matches!(t.kind(), ty::Param(_)))
        .map(|param| -> ty::Clause<'tcx> { ty::TraitRef::new(tcx, sized, [param]).upcast(tcx) });
    let param_env = ty::ParamEnv::new(tcx.mk_clauses_from_iter(clauses));
    let read: Ty<'tcx> = tcx
        .try_normalize_erasing_regions(
            ty::TypingEnv::new(param_env, ty::TypingMode::PostAnalysis),
            ty::Unnormalized::new_wip(projection),
        )
        .ok()?;
    (!matches!(read.kind(), ty::Alias(..))).then_some(read)
}

/// Whether a call of `trait_id`'s method on `self_ty` reads rest arguments as
/// a value, `dyn js::Rest<T>`, through the js crate's `Deref` of them, a
/// closure's `args.iter()` (ADR 0851): the array itself, which JS's rest
/// parameter is.
pub(in crate::lower) fn reads_rest_value<'tcx>(tcx: TyCtxt<'tcx>, trait_id: DefId, self_ty: Ty<'tcx>) -> bool {
    tcx.is_lang_item(trait_id, LangItem::Deref)
        && is_rest_value(tcx, Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, self_ty))
}

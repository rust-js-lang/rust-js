//! What `#[rust_js::nullable]` says, of a field, a function's result and
//! its parameters, and a thread-local: that a `None` is `null` there (ADRs
//! 0196, 0275, 0586).

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, FieldDef, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Whether a field of an `Option` is declared to take `null` too,
/// `#[rust_js::nullable]` (ADR 0196).
pub(in crate::lower) fn is_nullable(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether a nullable field is absent until set, `#[rust_js::optional]`, `field?: T | null` (ADR 0810).
pub(in crate::lower) fn is_optional(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("optional")];
    tcx.get_attrs_by_path(field.did, &path).next().is_some()
}

/// Whether a field's `None` is `null` from where it's made on, so its read is `null` or a value: a
/// nullable one's, not an optional one's, which a literal's `None` leaves out (ADR 0810, amended).
pub(in crate::lower) fn null_where_made(tcx: TyCtxt<'_>, field: &FieldDef) -> bool {
    is_nullable(tcx, field) && !is_optional(tcx, field)
}

/// The `Option` a `#[rust_js::nullable]` field of type `ty` holds: itself,
/// or a `Cell`'s, whose `None` is `null` in JS (ADRs 0196, 0772), or a
/// `RefCell`'s, a field's the value it holds (ADRs 0362, 0863). `None` of
/// any other type.
pub(in crate::lower) fn nullable_option<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let option = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    let cell = |adt: ty::AdtDef<'_>, item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    match ty.kind() {
        _ if option(ty) => Some(ty),
        ty::Adt(adt, args)
            if (cell(*adt, crate::lower::recognition::StdItem::Cell)
                || cell(*adt, crate::lower::recognition::StdItem::RefCell))
                && option(args.type_at(0)) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// The `Option` each item of a `#[rust_js::nullable]` field of type `ty`
/// is, a `Vec<Option<T>>`'s or a `Cell`'s or a `RefCell`'s of one, whose
/// `None` is `null` in JS, upstream's `createLaneMap(null)` (ADR 0586,
/// amended). `None` of any other type.
pub(in crate::lower) fn nullable_items<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let std = |item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    if std(crate::lower::recognition::StdItem::Cell) || std(crate::lower::recognition::StdItem::RefCell) {
        return nullable_items(tcx, args.type_at(0));
    }
    let item = args.types().next()?;
    (std(crate::lower::recognition::StdItem::Vec) && nullable_option(tcx, item) == Some(item)).then_some(item)
}

/// The parameters a binding names `#[rust_js::nullable(..)]`, each
/// `T | null` as a nullable field is (ADR 0275). On the function, since a
/// parameter's own attributes are kept neither in an `extern` block nor
/// in another crate's metadata.
pub(in crate::lower) fn nullable_params(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<Symbol> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(def_id, &path)
        .filter_map(|attr| attr.meta_item_list())
        .flatten()
        .filter_map(|item| item.ident().map(|ident| ident.name))
        .collect()
}

/// Whether a function says its result's `None` is `null`,
/// `#[rust_js::nullable]` alone on it, TypeScript's `T | null`, as a JS
/// caller of it tests `!== null`: React's scheduler's `peek(queue)`.
pub(in crate::lower) fn nullable_result(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    nullable_alone(tcx, def_id)
}

/// Whether `key`, a `thread_local!`'s, says its cell's `None` is `null`,
/// `#[rust_js::nullable]` alone on its static (ADR 0586, amended): its
/// module's variable, `null` where it's made and set, as a nullable
/// field's is, `Fiber | null` to TypeScript, as React's `let current:
/// Fiber | null = null` is.
pub(in crate::lower) fn nullable_local(tcx: TyCtxt<'_>, key: DefId) -> bool {
    nullable_alone(tcx, key)
}

/// `#[rust_js::nullable]` alone on `id`, of no parameter's name.
fn nullable_alone(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(id, &path).any(|attr| attr.is_word())
}

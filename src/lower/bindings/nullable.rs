//! What `#[rust_js::nullable]` says, of a field, a function's result and
//! its parameters, and a thread-local: that a `None` is `null` there (ADRs
//! 0196, 0275, 0586).

use crate::lower::recognition::{StdItem, is_std_def};
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

/// Whether a `#[rust_js::nullable]` field of type `ty`, a generic struct's,
/// is of a type parameter's value, itself or a `Cell`'s or a `RefCell`'s,
/// `current: RefCell<T>`: an `Option` where its struct is one of an
/// `Option`, `StackCursor<Option<Rc<HiddenContext>>>`, whose `None` is then
/// `null` as `nullable_option`'s is; as it is of any other (ADR 0586,
/// amended).
pub(in crate::lower) fn of_type_parameter(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    let cell = |adt: ty::AdtDef<'_>, item| adt.did() == crate::lower::recognition::std_item(tcx, item);
    match ty.kind() {
        ty::Param(_) => true,
        ty::Adt(adt, args)
            if cell(*adt, crate::lower::recognition::StdItem::Cell)
                || cell(*adt, crate::lower::recognition::StdItem::RefCell) =>
        {
            matches!(args.type_at(0).kind(), ty::Param(_))
        }
        _ => false,
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
/// Fiber | null = null` is. Or a `const`'s, of an `Option`: `export const
/// NoTransition = null`.
pub(in crate::lower) fn nullable_local(tcx: TyCtxt<'_>, key: DefId) -> bool {
    nullable_alone(tcx, key)
}

/// Whether `key`, a `#[rust_js::nullable]` thread-local's, holds an
/// `Option` itself, no cell of one: a `const` of its module, `null` where
/// it's made `None`, `export const callComponentInDEV = __DEV__ ? .. :
/// null`, as React's is (ADR 0586, amended).
pub(in crate::lower) fn nullable_local_option(tcx: TyCtxt<'_>, key: DefId) -> bool {
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    nullable_local(tcx, key)
        && matches!(tcx.type_of(key).instantiate_identity().skip_normalization().kind(),
            ty::Adt(local_key, args) if crate::lower::recognition::is_std_def(tcx, local_key.did(), crate::lower::recognition::StdItem::LocalKey)
                && option(args.type_at(0)))
}

/// What's wrong with `#[rust_js::nullable]` on `def`, a `const`, or a
/// `thread_local!`'s key, which is one: a thread-local's is of an `Option`,
/// or a `Cell` or a `RefCell` of one, or of a generic type made of one,
/// `StackCursor<Option<T>>`, whose generic constructor's given `None` is
/// `null`; a `const`'s is of an `Option` (ADR 0586, amended).
pub(in crate::lower) fn nullable_const_refused(tcx: TyCtxt<'_>, def: DefId) -> Option<&'static str> {
    if !nullable_local(tcx, def) {
        return None;
    }
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    let ty = tcx.type_of(def).instantiate_identity().skip_normalization();
    match ty.kind() {
        ty::Adt(key, args) if is_std_def(tcx, key.did(), StdItem::LocalKey) => {
            let held = args.type_at(0);
            let of_option = nullable_option(tcx, held).is_some() || made_of_options(tcx, held);
            (!of_option).then_some(
                "rust-js: `#[rust_js::nullable]` of a thread-local is of one of an `Option`, or a `Cell` or a `RefCell` of one, or of a generic type made of one, `StackCursor<Option<T>>`, whose `None` is then `null`; this one isn't",
            )
        }
        _ => (!option(ty)).then_some(
            "rust-js: `#[rust_js::nullable]` of a `const` is of one that's an `Option`, whose `None` is then `null`; this one isn't",
        ),
    }
}

/// Whether `held`, what a `#[rust_js::nullable]` thread-local holds, is
/// of a generic type one of whose type parameters is an `Option`,
/// `StackCursor<Option<Rc<HiddenContext>>>`, as a generic constructor
/// makes it, `create_cursor(None)`: each `Option` its initializer's call
/// is given for a type parameter's value is `null` for `None` (ADR 0586,
/// amended).
fn made_of_options(tcx: TyCtxt<'_>, held: Ty<'_>) -> bool {
    let option = |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    matches!(held.kind(), ty::Adt(adt, args) if !tcx.is_lang_item(adt.did(), LangItem::Option) && args.types().any(option))
}

/// Whether what `body` gives has its `None` `null`, as it writes it (ADR
/// 0578): a `#[rust_js::nullable]` function's result; and a nullable
/// thread-local's `init`, of one that holds the `Option` itself
/// (`nullable_local_option`), the value it's made, `__DEV__ ? f : null`.
pub(in crate::lower) fn null_result(tcx: TyCtxt<'_>, body: DefId) -> bool {
    nullable_result(tcx, body) || tcx.opt_parent(body).is_some_and(|key| nullable_local_option(tcx, key))
}

/// Whether `body`, a thread-local's `init`, is a `#[rust_js::nullable]`
/// one's (`nullable_local`), whose `None`s are `null` where it's made.
pub(in crate::lower) fn nullable_init(tcx: TyCtxt<'_>, body: DefId) -> bool {
    tcx.opt_parent(body).is_some_and(|key| nullable_local(tcx, key))
}

/// `#[rust_js::nullable]` alone on `id`, of no parameter's name.
fn nullable_alone(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("nullable")];
    tcx.get_attrs_by_path(id, &path).any(|attr| attr.is_word())
}

/// Is this `js::null()`, written as the `null` it gives, `link_name =
/// "null"` (ADR 0935, amended)? A constant written of it is its
/// initializer's, not rustc's value, whose `None` would be left out.
pub(in crate::lower) fn is_js_null(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    super::is_binding(tcx, def_id) && matches!(super::js_form(tcx, def_id), super::JsForm::Null)
}

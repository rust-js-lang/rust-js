//! A class's form (ADR 0761, amended, rd682): an ES class, `class Plugin {
//! constructor(..) { .. } apply(..) { .. } }`, of one that extends a
//! binding's class or whose struct is marked `#[rust_js::class(es)]`, or a
//! constructor function and its prototype; and what's refused of it.

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

use super::{is_class, superclass};

/// The word of `#[rust_js::class(es)]`, a class that's an ES class.
const ES: &str = "es";

/// What `#[rust_js::class(..)]` says of `def_id`'s form, the words in its
/// parentheses, `es` of `#[rust_js::class(es)]`; none of `#[rust_js::class]`.
fn class_form(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<String> {
    (tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("class")]))
        .flat_map(|attr| attr.meta_item_list().unwrap_or_default())
        .map(|item| item.ident().map_or_else(String::new, |ident| ident.name.to_string()))
        .collect()
}

/// Is `adt`, a class, an ES class, `class Plugin { constructor(..) { .. }
/// apply(..) { .. } }`: one that extends a binding's class (rd331), or
/// whose struct is marked `#[rust_js::class(es)]`, as its author writes it
/// where upstream does (rd682, the user's decision of 2026-11-03); else a
/// constructor function and its prototype (ADR 0761).
pub(crate) fn is_es_class(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    is_class(tcx, adt) && (class_form(tcx, adt).iter().any(|word| word == ES) || superclass(tcx, adt).is_some())
}

/// Each of the crate's ES classes (`is_es_class`).
pub(crate) fn es_classes(tcx: TyCtxt<'_>) -> Vec<DefId> {
    (tcx.hir_crate_items(()).definitions())
        .map(|id| id.to_def_id())
        .filter(|&id| tcx.def_kind(id) == DefKind::Struct && is_es_class(tcx, id))
        .collect()
}

/// Why `adt`, an ES class, has its prototype replaced refused: JS doesn't
/// let an ES class's `prototype` be written, and a load's assignment would
/// throw; a constructor function's is (#448, rd682).
pub(crate) fn es_prototype_refused(tcx: TyCtxt<'_>, adt: DefId) -> String {
    let class = tcx.item_name(adt);
    match superclass(tcx, adt) {
        Some((_, (_, extends))) => format!(
            "replacing the prototype of `{class}`, an ES class extending `{extends}`, whose `prototype` JS doesn't let be written"
        ),
        None => format!(
            "replacing the prototype of `{class}`, an ES class, `#[rust_js::class(es)]`, whose `prototype` JS doesn't let be written: a constructor function's is, `#[rust_js::class]` (rd682)"
        ),
    }
}

/// What's refused of the form `class`'s struct says, which extends a
/// binding's class where `extends`: a word that's no form, and `es` of one
/// that's an ES class already, by its impl (one way).
pub(super) fn form_misuse(tcx: TyCtxt<'_>, class: DefId, extends: bool) -> Option<&'static str> {
    match class_form(tcx, class).as_slice() {
        [] => None,
        [word] if word == ES && extends => {
            Some("that extends a binding's class is an ES class already: write it `#[rust_js::class]`")
        }
        [word] if word == ES => None,
        _ => Some("is a constructor function, `#[rust_js::class]`, or an ES class, `#[rust_js::class(es)]`"),
    }
}

/// What's refused of a class's impl, `imp`, marked with a form: its
/// struct's attribute says it, once.
pub(super) fn impl_form_misuse(tcx: TyCtxt<'_>, imp: DefId) -> Option<&'static str> {
    (!class_form(tcx, imp).is_empty()).then_some(
        "impl is marked `#[rust_js::class]` alone: its struct's attribute says its form, `#[rust_js::class(es)]`",
    )
}

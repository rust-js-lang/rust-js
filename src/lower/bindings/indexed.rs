//! `#[rust_js::indexed]`, of a binding's type: an indexed list, WebIDL's
//! `getter T item(unsigned long index)` with an integer `length`, a
//! `NodeList`, an `HTMLCollection`, whose `[Symbol.iterator]` WebIDL makes
//! `%Array.prototype.values%`: it reads `length`, then `list[i]`, at each
//! step, as an index loop does, so a `for` over it is one (ADR 1042). The
//! webapi generator says it of each interface WebIDL makes one.

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

fn marked(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("indexed")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// Whether `ty`, or what it borrows, is a binding's indexed list.
pub(in crate::lower) fn is_indexed_list<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if marked(tcx, adt.did()))
}

/// Why `def_id`'s `#[rust_js::indexed]` is refused: it's on no struct.
pub(super) fn indexed_refused(tcx: TyCtxt<'_>, def_id: DefId) -> Option<&'static str> {
    (marked(tcx, def_id) && tcx.def_kind(def_id) != DefKind::Struct).then_some(
        "rust-js: `#[rust_js::indexed]` is of a binding's struct, an indexed list's, WebIDL's indexed getter with a `length`",
    )
}

//! What a field's declaration reads of the type it's written with: an
//! `Option`'s payload, a `OnceCell`'s, and what a cell that's the property
//! it holds holds (ADR 0772), as written too (rd414, rd415, ADR 0935,
//! amended).

use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{self, Ty};

use super::super::recognition::{StdItem, is_std_def};
use super::Declarations;

impl<'tcx> Declarations<'_, 'tcx> {
    /// `Option<T>`'s `T` as written, where `written` is one, or a `Cell`'s or
    /// a `RefCell`'s of one, the `Option` a field's cell holds (rd414).
    pub(super) fn written_option<'h>(&self, written: &'h hir::Ty<'tcx>) -> Option<&'h hir::Ty<'tcx>> {
        let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = &written.kind else {
            return None;
        };
        let inner = || match path.segments.last()?.args?.args {
            [hir::GenericArg::Type(inner)] => Some(inner.as_unambig_ty()),
            _ => None,
        };
        match path.res {
            Res::Def(DefKind::Enum, did) if self.tcx.is_lang_item(did, LangItem::Option) => inner(),
            Res::Def(DefKind::Struct, did)
                if is_std_def(self.tcx, did, StdItem::Cell) || is_std_def(self.tcx, did, StdItem::RefCell) =>
            {
                self.written_option(inner()?)
            }
            _ => None,
        }
    }

    /// What a `OnceCell` or a `OnceLock` holds: in a field, the property,
    /// absent until it's set (ADR 0847).
    pub(super) fn once_cell_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::OnceCell) => Some(args.type_at(0)),
            _ => None,
        }
    }

    pub(super) fn option(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// What a field's cell holds where the field is the property it holds:
    /// a `Cell`'s, and a `RefCell`'s that holds its value by its declaration
    /// or is the object it holds (ADR 0863).
    pub(super) fn cell_item(&self, ty: Ty<'tcx>, field: Option<&ty::FieldDef>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args)
                if is_std_def(self.tcx, adt.did(), StdItem::Cell)
                    || field.is_some_and(|field| self.ref_cells.holds_value(self.tcx, field))
                    || self.ref_cells.is_value_cell(self.tcx, ty) =>
            {
                Some(args.type_at(0))
            }
            _ => None,
        }
    }
}

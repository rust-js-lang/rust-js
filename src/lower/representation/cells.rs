//! A field that's the property its cell holds (ADRs 0288, 0362, 0847): a
//! `Cell`'s, a struct's `RefCell`'s by its declaration (ADR 0863, amended),
//! a `OnceCell`'s; read as a handle on it, made of what the cell holds, and,
//! of a `RefCell` lent or borrowed where one may meet another, its one cell,
//! `$fieldCell`.

use super::super::FnCx;
use super::super::bindings;
use super::super::recognition::StdItem;
use crate::js;
use crate::js::Expr;
use rustc_middle::ty;
use rustc_middle::ty::Ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether field `i` of `ty` is a `#[rust_js::nullable]` field's `Cell`
    /// of an `Option`, whose `None` is `null`: the storage a handle on it
    /// writes so, wherever it's written through (ADR 0772).
    pub(in crate::lower) fn nullable_cell_field(&self, ty: Ty<'tcx>, (i, field_ty): (usize, Ty<'tcx>)) -> bool {
        let ty::Adt(adt, _) = ty.kind() else { return false };
        adt.is_struct()
            && bindings::is_nullable(
                self.tcx,
                &adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)],
            )
            && bindings::nullable_option(self.tcx, field_ty).is_some_and(|option| option != field_ty)
    }

    /// Whether field `i` of `ty`, a struct's, is `null` for `None` where
    /// it's made, and so never `undefined` (`bindings::null_where_made`).
    pub(in crate::lower) fn null_where_made(&self, ty: Ty<'tcx>, i: usize) -> bool {
        let ty::Adt(adt, _) = ty.kind() else { return false };
        adt.is_struct()
            && bindings::null_where_made(
                self.tcx,
                &adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)],
            )
    }

    /// A handle on `place`, a `nullable_cell_field`'s, `field` of type
    /// `ty`, that stores what's written to it as the field does: its
    /// `None`, `null` (ADR 0772); of a `RefCell` some borrow of which is
    /// checked, the cell `kept_cell` keeps of it, which stores so too.
    pub(in crate::lower) fn nullable_cell_handle(
        &self,
        place: Expr,
        field: Option<&ty::FieldDef>,
        ty: Ty<'tcx>,
    ) -> Expr {
        match self.kept_cell(&place, field, ty, true) {
            Some(kept) => kept,
            None => Expr::stored_handle(place, js::Storage::NullForNone),
        }
    }

    /// What a field `field` of type `ty` is, read where it's `read`:
    /// itself, or of a cell that's the property it holds (`property_cell`),
    /// a `Cell`'s say, set in place (ADR 0288), a handle on it. Every use of
    /// a cell, `.value`, reads and writes the property through it. Of a
    /// `RefCell` some borrow of which is checked, the one cell of it that
    /// counts them (`kept_cell`).
    pub(in crate::lower) fn held(&self, read: Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Expr {
        if let Some(kept) = self.kept_cell(&read, field, ty, false) {
            return kept;
        }
        match self.property_cell(field, ty) {
            Some(_) => Expr::handle(read),
            None => read,
        }
    }

    /// Of a struct's `RefCell` field that holds its value (`holds_value`),
    /// read where it's `read`, `x.f`, one lent, or some borrow of which may
    /// meet another, held while code that may borrow it runs:
    /// `$fieldCell(x, "f")`, a cell of it, the same one wherever it's
    /// asked, which counts its borrows where they're checked, its `value`
    /// the property; its `None` `null` where it's `nullable` (ADR 0863,
    /// amended). Not of one no borrow of which can meet another, never lent
    /// (`quiet_field`): a handle on it, made where it's read, never checked.
    fn kept_cell(&self, read: &Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>, nullable: bool) -> Option<Expr> {
        let field = field?;
        if !self.is_std_type(ty, StdItem::RefCell)
            || self.property_cell(Some(field), ty).is_none()
            || self.quiet_field(field.did)
        {
            return None;
        }
        let js::ExprKind::Member(object, key) = &read.kind else {
            return None;
        };
        let mut given = vec![(**object).clone(), Expr::str(key)];
        if nullable {
            given.push(Expr::bool(true));
        }
        Some(Expr::call_helper("$fieldCell", given))
    }

    /// Of `cell`, a struct's field's one cell (`kept_cell`), `$fieldCell(x,
    /// "f")`, borrowed where nothing counts its borrows: a handle on the
    /// field, as one never lent is, which reads and writes it in place, its
    /// nullable `None` `null` (ADR 0863, amended). Any other as it is.
    pub(in crate::lower) fn uncounted(&self, cell: Expr) -> Expr {
        if let js::ExprKind::Call(callee, given) = &cell.kind
            && callee.helper_name() == Some("$fieldCell")
            && let [holder, key, rest @ ..] = given.as_slice()
            && let js::ExprKind::Str(key) = &key.kind
        {
            let storage = match rest.is_empty() {
                true => js::Storage::AsGiven,
                false => js::Storage::NullForNone,
            };
            return Expr::stored_handle(Expr::member(holder.clone(), key.clone()), storage);
        }
        cell
    }

    /// Of a field `field` of type `ty` that's the property its cell holds,
    /// what the property is: a `Cell`'s `T` (ADR 0288), a struct's
    /// `RefCell`'s that holds its value by its declaration (ADR 0863,
    /// amended), but of a generic one made of an object, the object, which
    /// the cell is, and a `OnceCell`'s or `OnceLock`'s `Option<T>`, absent
    /// until it's set (ADR 0847). `None` of a field that's itself.
    pub(in crate::lower) fn property_cell(&self, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty::Adt(_, args) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::Cell)
            || field.is_some_and(|field| self.holds_value(field)) && !self.is_value_cell(ty)
        {
            return Some(args.type_at(0));
        }
        (self.is_std_type(ty, StdItem::OnceCell)).then(|| Ty::new_option(self.tcx, args.type_at(0)))
    }

    /// What a field `field` of type `ty` holds of `value`, a field's value
    /// made: of a cell that's the property (`property_cell`), what the
    /// cell holds.
    pub(in crate::lower) fn holding(&self, value: Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Expr {
        match self.property_cell(field, ty) {
            Some(_) => Expr::held(value),
            None => value,
        }
    }
}

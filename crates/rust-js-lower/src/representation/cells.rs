//! A field that's the property its cell holds (ADRs 0288, 0362, 0847): a
//! `Cell`'s, a struct's `RefCell`'s by its declaration (ADR 0863, amended),
//! a `OnceCell`'s; read as a handle on it, made of what the cell holds, and,
//! of a `RefCell` lent or borrowed where one may meet another, its one cell,
//! `$fieldCell`.

use super::super::FnCx;
use super::super::bindings;
use super::super::recognition::StdItem;
use rust_js_program::js;
use rust_js_program::js::Expr;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether field `i` of `ty` is a `#[rust_js::nullable]` field's `Cell`
    /// of an `Option`, whose `None` is `null`: the storage a handle on it
    /// writes so, wherever it's written through (ADR 0772).
    pub(crate) fn nullable_cell_field(&self, ty: Ty<'tcx>, (i, field_ty): (usize, Ty<'tcx>)) -> bool {
        let ty::Adt(adt, _) = ty.kind() else { return false };
        adt.is_struct()
            && super::none_form(
                self.tcx,
                super::NonePlace::Field(
                    &adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)],
                    Some(field_ty),
                ),
            )
            .stores_null()
            && bindings::nullable_option(self.tcx, field_ty) != Some(field_ty)
    }

    /// How field `i` of `ty`, a struct's, holds a `None`: of a nullable
    /// one not optional, `null` where it's made, and so never `undefined`
    /// (`NoneForm::Null`).
    pub(crate) fn field_none_form(&self, ty: Ty<'tcx>, i: usize) -> super::NoneForm {
        let ty::Adt(adt, _) = ty.kind() else {
            return super::NoneForm::Undefined;
        };
        match adt.is_struct() {
            true => super::none_form(
                self.tcx,
                super::NonePlace::Field(&adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)], None),
            ),
            false => super::NoneForm::Undefined,
        }
    }

    /// Whether a read of field `i` of `ty`, a nullable cell field's, is
    /// never `undefined`: one there where it's made, which stores `null`
    /// for `None` (`NoneForm::Null`), of a struct no getter answers the
    /// fields of (`fields_answered_by_getters`). The one answer of it: its
    /// read is `defined()` so (`project`), and the nulled owner gives what
    /// it holds, read whole, on as it is (`cell_field_held`, ADRs 0620 and
    /// 0810, amended; #1933).
    pub fn cell_field_read_defined(&self, ty: Ty<'tcx>, i: usize) -> bool {
        self.field_none_form(ty, i) == super::NoneForm::Null && !self.fields_answered_by_getters(ty)
    }

    /// A handle on `place`, a `nullable_cell_field`'s, `field` of type
    /// `ty`, that stores what's written to it as the field does: its
    /// `None`, `null` (ADR 0772); of a `RefCell` some borrow of which is
    /// checked, the cell `kept_cell` keeps of it, which stores so too.
    pub(crate) fn nullable_cell_handle(&self, place: Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Expr {
        match self.kept_cell(&place, field, ty, true) {
            Some(kept) => kept,
            None => Expr::stored_handle(place, super::NoneForm::Null.storage()),
        }
    }

    /// What a field `field` of type `ty` is, read where it's `read`:
    /// itself, or of a cell that's the property it holds (`property_cell`),
    /// a `Cell`'s say, set in place (ADR 0288), a handle on it. Every use of
    /// a cell, `.value`, reads and writes the property through it. Of a
    /// `RefCell` some borrow of which is checked, the one cell of it that
    /// counts them (`kept_cell`).
    /// Of a field held in a place of its own (`placed_field`, #2255), what's
    /// in that place, `both.a.value`.
    pub(crate) fn held(&self, read: Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Expr {
        if field.is_some_and(|field| self.placed_field(field, ty)) {
            return Expr::held(read);
        }
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
        let cell = Expr::call_helper("$fieldCell", given);
        Some(match nullable {
            true => Expr {
                facts: cell.facts.with(js::Known::NullStored),
                ..cell
            },
            false => cell,
        })
    }

    /// Of `cell`, a struct's field's one cell (`kept_cell`), `$fieldCell(x,
    /// "f")`, borrowed where nothing counts its borrows: a handle on the
    /// field, as one never lent is, which reads and writes it in place, its
    /// nullable `None` `null` (ADR 0863, amended). Any other as it is.
    pub(crate) fn uncounted(&self, cell: Expr) -> Expr {
        if let js::ExprKind::Call(callee, given) = &cell.kind
            && callee.helper_name() == Some("$fieldCell")
            && let [holder, key, ..] = given.as_slice()
            && let js::ExprKind::Str(key) = &key.kind
        {
            let storage = match cell.facts.has(js::Known::NullStored) {
                true => super::NoneForm::Null.storage(),
                false => super::NoneForm::Undefined.storage(),
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
    pub fn property_cell(&self, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        property_of(self.tcx, self.ref_cells(), field, ty)
    }

    /// Part `i` of `ty`'s array, of type `part`, read where it's `read`,
    /// `pair[0]`: of a tuple struct's field held in a place of its own
    /// (`placed_field`), what's in the place, `pair[0].value` (#2255). A
    /// cell in an array is the cell, never the property it holds (ADR
    /// 0288's are an object's fields).
    pub(crate) fn held_part(&self, read: Expr, ty: Ty<'tcx>, i: usize, part: Ty<'tcx>) -> Expr {
        match self
            .struct_field_def(ty, i)
            .is_some_and(|field| self.placed_field(field, part))
        {
            true => Expr::held(read),
            false => read,
        }
    }

    /// What part `i` of `ty`'s array, of type `part`, holds of `value`: of
    /// a tuple struct's field held in a place of its own, that place, `[{
    /// value: "B" }, 1]` (#2255).
    pub(crate) fn holding_part(&self, value: Expr, ty: Ty<'tcx>, i: usize, part: Ty<'tcx>) -> Expr {
        match self
            .struct_field_def(ty, i)
            .is_some_and(|field| self.placed_field(field, part))
        {
            true => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
            false => value,
        }
    }

    /// What a field `field` of type `ty` holds of `value`, a field's value
    /// made: of a cell that's the property (`property_cell`), what the
    /// cell holds; of one held in a place of its own (`placed_field`,
    /// #2255), that place, `{ value: "A" }`.
    pub fn holding(&self, value: Expr, field: Option<&ty::FieldDef>, ty: Ty<'tcx>) -> Expr {
        if field.is_some_and(|field| self.placed_field(field, ty)) {
            return Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]);
        }
        match self.property_cell(field, ty) {
            Some(_) => Expr::held(value),
            None => value,
        }
    }
}

/// Of a cell field's JS that's the property it holds, a handle on it,
/// `Handle(x.f)`, or its one cell, `$fieldCell(x, "f")` (ADRs 0288, 0863):
/// the struct's object and the field's key.
/// `FnCx::property_cell`'s answer, of the crate's `RefCells`: what else
/// asks it, the export net's records (`offered`), asks this.
pub fn property_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    ref_cells: &crate::drops::RefCells<'tcx>,
    field: Option<&ty::FieldDef>,
    ty: Ty<'tcx>,
) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let is = |item| crate::recognition::is_std_def(tcx, adt.did(), item);
    // Whether a struct's field holds its `RefCell`'s value as the property,
    // by its declaration (`RefCells::holds_value`, ADR 0863, amended).
    let holds_value = |field: &ty::FieldDef| {
        let holds = ref_cells.holds_value(tcx, field);
        rust_js_program::decisions::record("refcell-field", if holds { "holds-value" } else { "cell" });
        holds
    };
    if is(StdItem::Cell) || field.is_some_and(holds_value) && !ref_cells.is_value_cell(tcx, ty) {
        return Some(args.type_at(0));
    }
    is(StdItem::OnceCell).then(|| Ty::new_option(tcx, args.type_at(0)))
}

pub(crate) fn property_of_cell(e: &Expr) -> Option<(Expr, String)> {
    match &e.kind {
        js::ExprKind::Handle(read, _) => match &read.kind {
            js::ExprKind::Member(object, key) => Some(((**object).clone(), key.clone())),
            _ => None,
        },
        js::ExprKind::Call(callee, given) if callee.helper_name() == Some("$fieldCell") => match given.as_slice() {
            [object, key, ..] => match &key.kind {
                js::ExprKind::Str(key) => Some((object.clone(), key.clone())),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

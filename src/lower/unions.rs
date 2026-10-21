//! A union (ADR 0406): the object of the one field it holds, `{ a: 10 }`.
//! A field read is `$unionField(u, "a")`, which gives what it holds there,
//! or under a field of the same type, whose bytes are the same; a field
//! it doesn't hold read as another type's bytes, which JS has none of,
//! is rust-js's error where it runs. A field written is `$unionSet(u, "a",
//! v)`, the one it holds now.

use rustc_abi::FieldIdx;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::bindings::field_key;
use super::refusal::{Refused, refuse};
use super::{FnCx, R};
use crate::js::{Expr, Prop};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `ty`'s union definition and its arguments, if it's a union of a
    /// crate rust-js compiles: not std's `MaybeUninit`, which is what it
    /// holds (ADR 0332).
    fn union_of(&self, ty: Ty<'tcx>) -> Option<(ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>)> {
        match *ty.kind() {
            ty::Adt(adt, args) if adt.is_union() && self.is_rust_adt(adt.did()) => Some((adt, args)),
            _ => None,
        }
    }

    /// Whether `ty` is a union, whose fields are read and written as this
    /// module has them.
    pub(super) fn is_union(&self, ty: Ty<'tcx>) -> bool {
        self.union_of(ty).is_some()
    }

    /// A union made holding `field`, `U { a: 10 }`: `{ a: 10 }`.
    pub(super) fn union_value(&self, ty: Ty<'tcx>, field: FieldIdx, value: Expr) -> Expr {
        let (adt, _) = self.union_of(ty).expect("a union");
        let key = field_key(self.tcx, &adt.non_enum_variant().fields[field]);
        Expr::object(vec![Prop::Field(key, value)])
    }

    /// `u.field` read: what it holds there, or under its fields of the same
    /// type, `$unionField(u, "a", ["b"], { c: (value) => .. })`, or of
    /// another plain type, its bytes read as the field's, each other
    /// field's decoder as `transmute` reads them (ADRs 0408, 0562). A field
    /// of no bytes, `()`, MIR never reads.
    pub(super) fn union_read(&mut self, union: Expr, ty: Ty<'tcx>, field: FieldIdx, span: Span) -> R<Expr> {
        let (adt, args) = self.union_of(ty).expect("a union");
        let fields = &adt.non_enum_variant().fields;
        let read_ty = self.field_ty(&fields[field], args);
        let key = field_key(self.tcx, &fields[field]);
        let mut alike = Vec::new();
        let mut decoders = Vec::new();
        // A field JS holds as an object, which a write goes into, `u.a.0 =
        // x`: decoded only where it fills the union, which then holds it,
        // so the write is the union's and no byte is lost (ADR 0562); else
        // read as rust-js's error where it runs.
        let scalar = super::representation::Num::of(read_ty).is_some() || read_ty.is_bool() || read_ty.is_char();
        let union_size = self.layout(ty).map(|layout| layout.size.bytes());
        let writable_whole = scalar || union_size.is_some_and(|size| self.fills(read_ty, size));
        for (other, def) in fields.iter_enumerated() {
            if other == field {
                continue;
            }
            let other_ty = self.field_ty(def, args);
            let other_key = field_key(self.tcx, def);
            if other_ty == read_ty {
                alike.push(Expr::str(other_key));
            } else if writable_whole && let Some(decoder) = self.plain_decoder(other_ty, read_ty, span)? {
                decoders.push(Prop::Field(other_key, decoder));
            }
        }
        // The fields it may be read under, and the decoders, only where there are any.
        let mut list = vec![union, Expr::str(key)];
        if !alike.is_empty() || !decoders.is_empty() {
            list.push(Expr::array(alike));
        }
        if !decoders.is_empty() {
            list.push(Expr::object(decoders));
        }
        Ok(Expr::call(Expr::helper("$unionField"), list))
    }

    /// `u.field = value`: `$unionSet(u, "a", value)`, the field it holds now.
    pub(super) fn union_write(&self, union: Expr, ty: Ty<'tcx>, field: FieldIdx, value: Expr) -> Expr {
        let (adt, _) = self.union_of(ty).expect("a union");
        let key = field_key(self.tcx, &adt.non_enum_variant().fields[field]);
        Expr::call(Expr::helper("$unionSet"), vec![union, Expr::str(key), value])
    }

    /// A copy of `union`, a union of its own: `{ ...u }`, and where a field
    /// holds what changes in place, of the one it holds, a copy of that,
    /// `Object.hasOwn(u, "a") ? { a: <copy of u.a> } : { ...u }`.
    pub(super) fn union_copy(&self, union: Expr, ty: Ty<'tcx>) -> Expr {
        let (adt, args) = self.union_of(ty).expect("a union");
        let mut copied = Expr::object(vec![Prop::Spread(union.clone())]);
        for def in adt.non_enum_variant().fields.iter().rev() {
            let field_ty = self.field_ty(def, args);
            if !self.contains_mutated(field_ty) {
                continue;
            }
            let key = field_key(self.tcx, def);
            let held = Expr::call(
                Expr::member(Expr::var("Object"), "hasOwn"),
                vec![union.clone(), Expr::str(key.clone())],
            );
            let field = self.copy(Expr::member(union.clone(), key.clone()), field_ty);
            copied = Expr::cond(held, Expr::object(vec![Prop::Field(key, field)]), copied);
        }
        copied
    }

    /// Whether a field of the union `ty` holds what changes in place, so a
    /// copy of it copies that.
    pub(super) fn union_holds_mutated(&self, ty: Ty<'tcx>) -> bool {
        let (adt, args) = self.union_of(ty).expect("a union");
        adt.all_fields()
            .any(|def| self.contains_mutated(self.field_ty(def, args)))
    }

    /// A `&mut` to a union's field, which a write through would make the
    /// one it holds without saying so: not yet.
    pub(super) fn union_field_borrowed(&self, span: Span) -> R<Expr> {
        Err(refuse(self.tcx, span, Refused::Feature("a `&mut` to a union's field")))
    }
}

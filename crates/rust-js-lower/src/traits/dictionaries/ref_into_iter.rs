//! A reference's `IntoIterator` dictionary, a collection's `&C` or `&mut
//! C` a bound gives generic code (ADR 0457), and an iterator's `&mut I`
//! where generic code holds it as concrete code does (#2336). Split from
//! dictionaries.rs by concept, as a file is at most 1500 lines.

use super::super::*;
use crate::generic_facts::{Crossing, Form, RefHeld};
use crate::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `tr`, std's `IntoIterator` of a reference, is a collection's,
    /// `&C`'s or `&mut C`'s, which `ref_into_iter_dictionary` makes: not a
    /// `&mut I` of an iterator, which std's impl for every iterator takes as
    /// it is, as rustc's selection says (`iterates_as_itself`, #742), nor
    /// one the crate implements, nor one a bound of this function's gives,
    /// `&'a I: IntoIterator`'s, its dictionary (#1264). The one answer, of a
    /// bound's dictionary and of a type parameter given one, `zip`'s `U`.
    pub(crate) fn ref_into_iter_of_collection(&self, tr: ty::TraitRef<'tcx>) -> bool {
        let ty = tr.self_ty();
        is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && matches!(ty.kind(), ty::Ref(..))
            && !self.recognition().iterates_as_itself(ty)
            && !self.has_user_impl(tr.def_id, ty)
            && self.evidence_for(tr).is_none()
    }

    /// Whether `tr` is std's `IntoIterator` of a `&mut I` of an iterator,
    /// which std's impl for every iterator takes as it is
    /// (`iterates_as_itself`, #742), not one the crate implements.
    pub(in crate::traits) fn iterator_ref(&self, tr: ty::TraitRef<'tcx>) -> bool {
        let ty = tr.self_ty();
        is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut))
            && self.recognition().iterates_as_itself(ty)
            && !self.has_user_impl(tr.def_id, ty)
    }

    /// Whether `tr`, std's `IntoIterator` of a `&mut`, is given where
    /// generic code holds that `&mut` in the box its caller makes of a
    /// `&mut` to an object, `crossing`'s (`Form::Box`, ADR 0454).
    pub(in crate::traits) fn mut_ref_in_box(&self, tr: ty::TraitRef<'tcx>, crossing: Crossing<'tcx>) -> bool {
        is_std_def(self.tcx, tr.def_id, StdItem::IntoIterator)
            && matches!(tr.self_ty().kind(), ty::Ref(_, _, Mutability::Mut))
            && self.form_differs(Form::Box, crossing)
    }

    /// `dictionary`, std's `IntoIterator` of a `&mut`, `tr`, where generic
    /// code holds that `&mut` as it is, a `C: Iterator`'s, passed on where
    /// its callee holds it in its box (`mut_ref_in_box`): its `into_iter`
    /// given what the box holds, `$throughBox(CRefMutIntoIterator, {
    /// intoIter: [1] })` (ADR 0454, #2336).
    pub(in crate::traits) fn mut_ref_through_box(&self, tr: ty::TraitRef<'tcx>, dictionary: Expr) -> Expr {
        let into_iter = crate::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
        let entries = Expr::object(vec![Prop::Field(
            bindings::fn_name(self.tcx, into_iter),
            Expr::array(vec![Expr::int(1)]),
        )]);
        Expr::call_helper("$throughBox", vec![dictionary, entries])
    }

    /// std's `IntoIterator` of a `&C` or a `&mut C`, `tr`, as generic code
    /// holds it, `held` (`ref_held`): in the box a `&mut` to an object is,
    /// as a `c: &mut C` of an unknown `C` holds it (ADR 0454); or as it is,
    /// as a `U: IntoIterator` given a `&mut [T]` holds it, std's `zip`'s
    /// and `chain`'s, which a bound by `IntoIterator` never boxes (ADR
    /// 0976). Of a `&mut I` of an iterator, which std's impl takes as it is,
    /// held as concrete code has it: `into_iter` gives it as its caller has
    /// it, stepped by `&mut I`'s own `Iterator`, `I`'s steps passed on,
    /// `{ intoIter: (cell) => cell.value, IntoIterIterator: () => ({ next:
    /// $rangeNext }) }` (#2336).
    pub(crate) fn ref_into_iter_dictionary(&mut self, tr: ty::TraitRef<'tcx>, held: RefHeld, span: Span) -> R<Expr> {
        let ty = tr.self_ty();
        let ty::Ref(_, pointee, mutability) = *ty.kind() else {
            return Err(self.no_evidence(span, tr));
        };
        let into_iter = crate::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
        let itself = self.recognition().iterates_as_itself(ty);
        // A `&mut`'s, the cell generic code has, a box of an object's.
        let role = match mutability {
            Mutability::Mut if itself && held != RefHeld::InBox => Role::Items,
            Mutability::Mut => Role::Cell,
            Mutability::Not => self.parameter_name(pointee),
        };
        let name = js::Name::declared(role.text().to_string());
        let given = held.read(Expr::var(name.clone()));
        let mut body = Vec::new();
        let items = match itself {
            true => given,
            false => (self.ref_into_iter(given, ty, span, &mut body)?).ok_or_else(|| self.no_evidence(span, tr))?,
        };
        body.push(StmtKind::Return(Some(items)).at(js::Span::NONE));
        let f = Expr::arrow(vec![name.into()], body);
        let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
        props.extend(self.made_steps(tr, itself, span)?);
        Ok(Expr::object(props))
    }
}

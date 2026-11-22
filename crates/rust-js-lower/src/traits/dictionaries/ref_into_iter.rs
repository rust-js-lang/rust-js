//! A reference's `IntoIterator` dictionary, a collection's `&C` or `&mut
//! C` a bound gives generic code (ADR 0457). Split from dictionaries.rs by
//! concept, as a file is at most 1500 lines.

use super::super::*;
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

    /// std's `IntoIterator` of a `&C` or a `&mut C`, `tr`, as generic code
    /// holds it: in the box a `&mut` to an object is, `boxed`, as a `c: &mut
    /// C` of an unknown `C` holds it (ADR 0454); or as it is, as a `U:
    /// IntoIterator` given a `&mut [T]` holds it, std's `zip`'s and
    /// `chain`'s, which a bound by `IntoIterator` never boxes (ADR 0976).
    pub(crate) fn ref_into_iter_dictionary(&mut self, tr: ty::TraitRef<'tcx>, boxed: bool, span: Span) -> R<Expr> {
        let ty = tr.self_ty();
        let ty::Ref(_, pointee, mutability) = *ty.kind() else {
            return Err(self.no_evidence(span, tr));
        };
        let into_iter = crate::recognition::trait_method(self.tcx, tr.def_id, "into_iter");
        // A `&mut`'s, the cell generic code has, a box of an object's.
        let role = match mutability {
            Mutability::Mut => Role::Cell,
            Mutability::Not => self.parameter_name(pointee),
        };
        let name = js::Name::declared(role.text().to_string());
        let given = match boxed {
            true => Expr::held(Expr::var(name.clone())),
            false => Expr::var(name.clone()),
        };
        let mut body = Vec::new();
        let items = (self.ref_into_iter(given, ty, span, &mut body)?).ok_or_else(|| self.no_evidence(span, tr))?;
        body.push(StmtKind::Return(Some(items)).at(js::Span::NONE));
        let f = Expr::arrow(vec![name.into()], body);
        let mut props = vec![Prop::Field(bindings::fn_name(self.tcx, into_iter), f)];
        props.extend(self.made_steps(tr, false, span)?);
        Ok(Expr::object(props))
    }
}

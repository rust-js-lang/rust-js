//! What a type parameter is given for a `&mut` to a function or an
//! iterator: the value itself where generic code only calls or steps it,
//! through what refers to it, `lent.value` (#992, #1909), and where it
//! can't be, refused. Split from `mut_refs.rs` by concept.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `declared`, a type parameter of `def_id`'s, given a `&mut` to
    /// a function or an iterator, `instantiated` its caller's type, takes it
    /// as it is, not in generic code's box: where its value is read as
    /// another type parameter its bound names that's called or stepped,
    /// `P: Is<F>` of `F: FnMut()`, `I: Iterator` or `I: IntoIterator`, as
    /// that one is given it, its form's (`mut_ref_form`), the function, the
    /// iterator or the range itself (#992). A collection given for an
    /// `IntoIterator` is boxed still, as its dictionary takes its cell. And
    /// where its own bound is a function trait, `F: FnOnce()` given `&mut
    /// callback` of a `Box<dyn Fn()>`, the function, as std's `impl FnOnce
    /// for &mut F` calls what it refers to (#1909): generic code can only
    /// call it, move it and drop it, so no write through the `&mut` is
    /// lost, where `F` is in none of its other parameters and not in what
    /// it returns, which could give the `&mut` back.
    pub fn given_as_the_value(&self, def_id: DefId, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> bool {
        let ty::Ref(_, pointee, Mutability::Mut) = *instantiated.kind() else {
            return false;
        };
        matches!(declared.kind(), ty::Param(_))
            && ((self.is_callable(pointee) || self.implements_iterator(pointee))
                && self.read_as_stepped(self.tcx.param_env(def_id), declared)
                || self.called_through(pointee) && self.only_called(def_id, declared))
    }

    /// Whether `param`, a type parameter of `def_id`'s, is bound by a
    /// function trait and is in only one of its parameters' types, and not
    /// in what it returns: what it's given is only called there (#1909).
    fn only_called(&self, def_id: DefId, param: Ty<'tcx>) -> bool {
        let param_env = self.tcx.param_env(def_id);
        let called = (self.declared_bounds(param, param_env).into_iter())
            .any(|id| self.tcx.fn_trait_kind_from_def_id(id).is_some());
        let sig = self
            .tcx
            .instantiate_bound_regions_with_erased(self.tcx.fn_sig(def_id).instantiate_identity().skip_normalization());
        let mentions = |ty: Ty<'tcx>| ty.walk().any(|part| part.as_type() == Some(param));
        called && !mentions(sig.output()) && sig.inputs().iter().filter(|&&input| mentions(input)).count() == 1
    }

    /// Whether `declared`, a type parameter of `def_id`'s bound by a
    /// function trait, given a `&mut` to a function in a box, `instantiated`,
    /// would be given it as generic code's box, which its call can't call:
    /// where `F` is in another parameter or what it returns, the `&mut` is
    /// both what's called and a place a write through may change (#1909).
    pub fn called_box_escapes(&self, def_id: DefId, declared: Ty<'tcx>, instantiated: Ty<'tcx>) -> bool {
        let ty::Ref(_, pointee, Mutability::Mut) = *instantiated.kind() else {
            return false;
        };
        let param_env = self.tcx.param_env(def_id);
        (self.declared_bounds(declared, param_env).into_iter())
            .any(|id| self.tcx.fn_trait_kind_from_def_id(id).is_some())
            && self.called_through(pointee)
            && !self.given_as_the_value(def_id, declared, instantiated)
            && self.mut_ref_form(pointee).is_cell()
    }

    /// `value`, of `ty`, a function through references and boxes, as the
    /// function it calls: each `&mut` held in a box read through,
    /// `lent.value`, a shared one or a `Box` the value itself (#1909).
    pub fn called_value(&self, value: Expr, ty: Ty<'tcx>) -> Expr {
        match ty.kind() {
            ty::Ref(_, inner, Mutability::Mut) if self.mut_ref_form(*inner).is_cell() => {
                self.called_value(Expr::member(value, js::HELD), *inner)
            }
            ty::Ref(_, inner, _) => self.called_value(value, *inner),
            _ if let Some(inner) = ty.boxed_ty() => self.called_value(value, inner),
            _ => value,
        }
    }

    /// Whether `ty` is a function, or one through references and boxes,
    /// `Box<dyn Fn()>`, `&dyn Fn()`, which std's `impl Fn for &F`, `&mut F`
    /// and `Box<F>` call (#1909).
    fn called_through(&self, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Ref(_, inner, _) => self.called_through(*inner),
            _ if let Some(inner) = ty.boxed_ty() => self.called_through(inner),
            _ => self.is_callable(ty),
        }
    }

    /// Whether `param`'s value is read as another type parameter its bound
    /// names, `P: Is<F>`, that's called or stepped, bound by a function
    /// trait, `Iterator` or `IntoIterator`, which is given a function or an
    /// iterator as it is (#992).
    fn read_as_stepped(&self, param_env: ty::ParamEnv<'tcx>, param: Ty<'tcx>) -> bool {
        (param_env.caller_bounds())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == param)
            .flat_map(|bound| bound.skip_binder().trait_ref.args.types().skip(1))
            .any(|named| {
                matches!(named.kind(), ty::Param(_))
                    && self.declared_bounds(named, param_env).into_iter().any(|id| {
                        self.tcx.fn_trait_kind_from_def_id(id).is_some()
                            || crate::recognition::is_std_def(self.tcx, id, crate::recognition::StdItem::Iterator)
                            || crate::recognition::is_std_def(self.tcx, id, crate::recognition::StdItem::IntoIterator)
                    })
            })
    }
}

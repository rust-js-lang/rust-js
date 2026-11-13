//! A call of a method std's impl for a `&mut W` or a `Box<I>` writes by
//! passing it on to `W`'s or `I`'s: that method's, of what the receiver
//! is to (#1278, #1309). Split from `dispatch.rs` by concept (#1334).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A method std's impl for a `&mut W` passes on to `W`'s
    /// (`passed_through_mut_ref`): `W`'s, given the `&mut W` its receiver
    /// is of, `*self` (#1309), of `&self` the `&mut W` read as `W` itself
    /// (#1709). Another call as it is.
    pub(super) fn passed_through<'b>(
        &self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        args: &'b [rustc_span::Spanned<Operand<'tcx>>],
    ) -> (
        ty::GenericArgsRef<'tcx>,
        std::borrow::Cow<'b, [rustc_span::Spanned<Operand<'tcx>>]>,
    ) {
        // Of `&self`, only where the `&mut W` reads as `W` itself, an
        // object's, not a number's cell (#1709).
        let passed = (self.recognition())
            .passed_through_mut_ref(def_id, generic_args)
            .filter(|&(inner, shared)| !shared || self.mut_ref_form(inner.type_at(0)).read_is_itself())
            .map(|(inner, _)| inner)
            .zip(args.first().and_then(|receiver| receiver.node.place()));
        let Some((inner, receiver)) = passed else {
            return (generic_args, std::borrow::Cow::Borrowed(args));
        };
        let deref = rustc_span::Spanned {
            node: Operand::Copy(self.tcx.mk_place_deref(receiver)),
            span: args[0].span,
        };
        (inner, std::iter::once(deref).chain(args[1..].iter().cloned()).collect())
    }

    /// A method std's impl for a `Box<I>` passes on to `I`'s
    /// (`passed_through_box`): `I`'s arguments, its receiver's type made
    /// `I`'s of the box's (#1278). A `Box<I>` or a `&Box<I>` is `I`'s value;
    /// a `&mut Box<I>` of an iterator is what a `&mut I` is (#1314), given as
    /// it is, where that's the iterator itself; `None` where it's a cell, and
    /// of another call.
    pub(super) fn passed_through_box(
        &mut self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        arg_tys: &mut [Ty<'tcx>],
    ) -> Option<ty::GenericArgsRef<'tcx>> {
        let args = self.recognition().passed_through_box(def_id, generic_args)?;
        let receiver_ty = arg_tys.first_mut()?;
        let (boxed, mutability) = match *receiver_ty.kind() {
            ty::Ref(_, boxed, mutability) => (boxed, Some(mutability)),
            _ => (*receiver_ty, None),
        };
        let inner = boxed.boxed_ty()?;
        if mutability == Some(ty::Mutability::Mut) && !self.mut_ref_form(inner).read_is_itself() {
            return None;
        }
        *receiver_ty = match mutability {
            Some(mutability) => Ty::new_ref(self.tcx, self.tcx.lifetimes.re_erased, inner, mutability),
            None => inner,
        };
        Some(args)
    }
}

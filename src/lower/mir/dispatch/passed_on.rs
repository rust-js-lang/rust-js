//! A call of a method std's impl for a `&mut W` or a `Box<I>` writes by
//! passing it on to `W`'s or `I`'s: that method's, of what the receiver
//! is to (#1278, #1309). Split from `dispatch.rs` by concept (#1334).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A method std's impl for a `&mut W` passes on to `W`'s
    /// (`passed_through_mut_ref`): `W`'s, given the `&mut W` its receiver
    /// is of, `*self` (#1309), of `&self` the `&mut W` read as `W` itself
    /// (#1709). Through each `&mut` such an impl only forwards, a `&mut
    /// &mut W`'s `finish()`, `**self` (#1724), but never past an impl of the
    /// crate's own, `impl Hasher for &mut Tally`, which isn't std's. Another
    /// call as it is.
    pub(super) fn passed_through<'b>(
        &self,
        body: &mir::Body<'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        args: &'b [rustc_span::Spanned<Operand<'tcx>>],
    ) -> (
        ty::GenericArgsRef<'tcx>,
        std::borrow::Cow<'b, [rustc_span::Spanned<Operand<'tcx>>]>,
    ) {
        let Some(mut place) = args.first().and_then(|receiver| receiver.node.place()) else {
            return (generic_args, std::borrow::Cow::Borrowed(args));
        };
        // The deepest `W` the call can be of: of `&self`, only where the
        // `&mut W` reads as `W` itself, an object's, not a number's cell
        // (#1709), or, where `W` is a reference, the crate's own `impl
        // Hasher for &mut Tally`'s, the `W` its place holds, whose `&W` is
        // the `W` itself, never a copy (#1724). Each `&mut` before it is
        // read through as a place is.
        let mut levels = Vec::new();
        let mut at = generic_args;
        while let Some((inner, shared)) = self.recognition().passed_through_mut_ref(def_id, at) {
            levels.push((inner, shared));
            at = inner;
        }
        let deepest = levels.iter().enumerate().rev().find_map(|(k, &(inner, shared))| {
            let w = inner.type_at(0);
            match !shared || self.mut_ref_form(w).read_is_itself() {
                true => Some((k, inner, false)),
                false => w.is_ref().then_some((k, inner, true)),
            }
        });
        let Some((k, inner, holds)) = deepest else {
            return (generic_args, std::borrow::Cow::Borrowed(args));
        };
        // Read through more than once, a `&self`'s receiver, a temporary `&`
        // of the `&mut`, `_10 = &_3`, is read from what it borrows, `_3`,
        // not through the temporary, which read one `.value` too many; read
        // through once, through it, as `_3` may be no JS name (#1724).
        let lent = (k + usize::from(holds) > 0 && levels[0].1)
            .then(|| lent_by(body, place))
            .flatten();
        place = match lent {
            Some(lent) => lent,
            None => self.tcx.mk_place_deref(place),
        };
        for _ in 0..k + usize::from(holds) {
            place = self.tcx.mk_place_deref(place);
        }
        let deref = rustc_span::Spanned {
            node: Operand::Copy(place),
            span: args[0].span,
        };
        (inner, std::iter::once(deref).chain(args[1..].iter().cloned()).collect())
    }

    /// A method std's impl for a `Box<I>` passes on to `I`'s
    /// (`passed_through_box`): `I`'s arguments, its receiver's type made
    /// `I`'s of the box's (#1278). A `Box<I>` or a `&Box<I>` is `I`'s value;
    /// a `&mut Box<I>` of an iterator is what a `&mut I` is (#1314), given as
    /// it is, where that's the iterator itself. A `&mut Box<H>` of a hasher
    /// is a cell of the box, its receiver, of `values`, read through,
    /// `dest_place`'s, `b` of `&mut b`, where a `&mut H` is the hasher
    /// itself (#1493). `None` where a `&mut I` is a cell, and of another
    /// call; refused, naming #2037, of a hasher's method still of a `&mut
    /// Box<H>`, `r.finish()` of a `r = &mut b`, `&self`'s, which the `&mut`'s
    /// pass-through (`passed_through`) takes only where the `&mut` reads as
    /// the box itself.
    pub(super) fn passed_through_box(
        &mut self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        arg_tys: &mut [Ty<'tcx>],
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<Option<ty::GenericArgsRef<'tcx>>> {
        let Some(args) = self.recognition().passed_through_box(def_id, generic_args) else {
            if let Some(trait_id) = self.tcx.trait_of_assoc(def_id)
                && crate::lower::recognition::is_hasher(self.tcx, trait_id)
                && let Some(&ty::Ref(_, boxed, ty::Mutability::Mut)) = generic_args.types().next().map(|t| t.kind())
                && boxed.is_box()
            {
                let what = format!(
                    "`{}` of a `&mut` to a `Box` of a hasher (#2037)",
                    self.tcx.item_name(def_id)
                );
                let why = "the `&mut` is a cell of the box, not the hasher: call it of the box, `(**r).finish()`";
                return Err(crate::lower::refusal::refuse(
                    self.tcx,
                    span,
                    crate::lower::refusal::Refused::Because(&what, why),
                ));
            }
            return Ok(None);
        };
        let Some(receiver_ty) = arg_tys.first_mut() else {
            return Ok(None);
        };
        let (boxed, mutability) = match *receiver_ty.kind() {
            ty::Ref(_, boxed, mutability) => (boxed, Some(mutability)),
            _ => (*receiver_ty, None),
        };
        let Some(inner) = boxed.boxed_ty() else {
            return Ok(None);
        };
        let lent = mutability == Some(ty::Mutability::Mut);
        if lent && !self.mut_ref_form(inner).read_is_itself() {
            return Ok(None);
        }
        if lent
            && !self.mut_ref_form(boxed).read_is_itself()
            && let Some(receiver) = values.first_mut()
        {
            *receiver = Value::Expr(self.dest_place(receiver, boxed, span)?);
        }
        *receiver_ty = match mutability {
            Some(mutability) => Ty::new_ref(self.tcx, self.tcx.lifetimes.re_erased, inner, mutability),
            None => inner,
        };
        Ok(Some(args))
    }
}

/// What `place`, a temporary assigned once a shared borrow, `_10 = &_3`,
/// borrows: `_3`. `None` of another.
fn lent_by<'tcx>(body: &mir::Body<'tcx>, place: Place<'tcx>) -> Option<Place<'tcx>> {
    if !place.projection.is_empty() {
        return None;
    }
    match crate::lower::body_queries::assigned_once(body).get(&place.local) {
        Some(Some(mir::Rvalue::Ref(_, mir::BorrowKind::Shared, lent))) => Some(*lent),
        _ => None,
    }
}

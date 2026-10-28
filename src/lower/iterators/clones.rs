//! An iterator's clone: of one made in place, and of a `copied()`, which
//! is the iterator it copies or a JS iterator made of it.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `clone_value` of an iterator made in place, a part of an adapter
    /// cloned apart or what `cycle()` takes: the JS iterator it is, an adapter
    /// std compiles too, whose clone makes it again (ADR 0525, ADR 0871).
    pub(in crate::lower) fn clone_in_place(
        &mut self,
        place: Expr,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let ty = self.reveal(ty);
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty)
            && self.copy_is_its_source(source)
        {
            return self.clone_in_place(place, source, span, out);
        }
        match self.iterator_cloned_apart(ty) {
            true => Ok(Expr::call_helper("$clone", vec![place])),
            false => self.clone_value(place, ty, span, out),
        }
    }

    /// A clone of `place`, a `copied()` of a `source`. Of std's iterator, it
    /// is the iterator it copies (ADR 0480), its clone that one's. Of the
    /// crate's own, it's a JS iterator of it, `$iterator(value, next)`: one
    /// of a clone of its value, `copied.with({ ...copied.of })`; of what
    /// generic code was given, `$given`'s, that or the value, as `$given`
    /// made it, `$givenClone(copied, IClone.clone)` (#384).
    pub(in crate::lower) fn copied_clone(
        &mut self,
        place: Expr,
        (ty, source): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let source = self.reveal(source);
        // A copy of items something changes in place, `source.map((item) =>
        // ({ ...item }))`, a JS iterator of JS's, which nothing makes again.
        if !self.is_its_source(ty) {
            return Err(self.unsupported(span, &format!("cloning a `{ty}`, whose items are copied as it steps")));
        }
        if self.copy_is_its_source(source) {
            return self.clone_value(place, source, span, out);
        }
        // One closed to drop what it steps, `$closing($iterator(..), drop)`:
        // the crate's own with a destructor, or what generic code is given
        // its drop of (ADRs 0421, 0465).
        let dropped = match self.is_unknown(source) {
            true => self.given_drop(source).is_some(),
            false => self.drops(source) != crate::lower::drops::Drops::Nothing,
        };
        if dropped {
            return Err(self.unsupported(span, &format!("cloning a `copied()` of a `{source}` it drops")));
        }
        if self.is_unknown(source) {
            let tr = ty::TraitRef::new(self.tcx, self.clone_trait(), [source]);
            let clone = match self.evidence_for(tr) {
                Some(dictionary) => Expr::member(dictionary, "clone"),
                None => self.clone_fn("value", source, span)?,
            };
            return Ok(Expr::call_helper("$givenClone", vec![place, clone]));
        }
        let of = self.clone_value(Expr::member(place.clone(), "of"), source, span, out)?;
        Ok(Expr::call(Expr::member(place, "with"), vec![of]))
    }

    /// Whether a `copied()` of a `source` is, in JS, the value `source` is:
    /// std's iterators', not the crate's own, which is a `$iterator` of it,
    /// nor what generic code was given, which may be a `$given` of it.
    fn copy_is_its_source(&self, source: Ty<'tcx>) -> bool {
        let source = self.reveal(source);
        !self.is_user_iterator(source) && !self.is_unknown(source)
    }
}

//! `dest.clone_from(&source)` of a std type, as std's `clone_from` writes
//! it (#919, #922, #927, #928): its own in place, `clone_into`'s; an
//! `Option`'s value's where both are `Some`; a value only the caller knows,
//! its dictionary's `clone_from`, asked first; else `*dest =
//! source.clone()`, what it held dropped.

use super::*;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Std::CloneFrom`'s lowering, of `values`, the destination then the
    /// source, `arg_tys` theirs.
    pub(super) fn mir_clone_from(
        &mut self,
        state: &mut State<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Value<'tcx>>,
        (def_id, arg_tys): (DefId, &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        Ok(Value::Expr({
            let ty = arg_tys[1].peel_refs();
            let dest = values.next().expect("the destination");
            let source = values.next().expect("the source");
            let source = self.value_expr(source, span)?;
            if let Some(into) = self.clone_into(ty, span)? {
                let dest = self.dest_place(&dest, ty, span)?;
                self.flush(state, out)?;
                out.push(StmtKind::Expr(into.call(dest, source)).at(self.js_span(span)));
                return Ok(Value::Expr(Expr::js_undefined()));
            }
            // Of an `Option`, its value's, where both are `Some`; of a
            // value only the caller knows, a `T` or a `Box<T>`, or an
            // `Option` of one, its dictionary's `clone_from`, where
            // the caller's type has one, asked first (#928): given
            // the box generic code has of `dest`, or one of its place
            // here, written back.
            let inner = self.option_of(ty).filter(|&inner| !self.boxed_payload(inner));
            let to = self.dest_place(&dest, ty, span)?;
            let given_box = match &dest {
                Value::Ref(place) => self.box_of_place(state, place),
                _ => Some(self.value_expr(dest.clone(), span)?),
            };
            let made = given_box.is_none().then(|| self.fresh(Role::To));
            let pointer = given_box.unwrap_or_else(|| Expr::var(made.as_ref().expect("a box made")));
            let held = inner.unwrap_or(ty);
            if self
                .written_over(held, (to.clone(), pointer.clone()), source.clone(), span)?
                .is_some()
            {
                self.flush(state, out)?;
                // Read again by what's asked and by the clone.
                let source = match source.reads_same() {
                    true => source,
                    false => self.spill(Role::Source, source, out),
                };
                let (asked, written) = self
                    .written_over(held, (to.clone(), pointer), source.clone(), span)?
                    .expect("written over, as asked above");
                // The box made of the place, where a dictionary's is
                // given it, not `clone_into`'s, which writes the place.
                let made = made.filter(|_| asked.is_some());
                let some = |e: &Expr| Expr::bin(Op::LooseNe, e.clone(), Expr::null());
                let both = inner.map(|_| Expr::bin(Op::And, some(&to), some(&source)));
                let test = match (both, asked) {
                    (Some(both), Some(asked)) => Expr::bin(Op::And, both, asked),
                    (Some(test), None) | (None, Some(test)) => test,
                    (None, None) => unreachable!("`clone_into`'s is written above"),
                };
                let mut then = Vec::new();
                if let Some(name) = &made {
                    then.push(
                        StmtKind::Const(name.into(), crate::lower::mut_refs::boxed(to.clone())).at(self.js_span(span)),
                    );
                }
                then.push(StmtKind::Expr(written).at(self.js_span(span)));
                if let Some(name) = &made {
                    then.push(
                        StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),
                    );
                }
                let mut replaced = Vec::new();
                self.replace_with_clone(state, (dest, source), (ty, def_id), arg_tys, span, &mut replaced)?;
                out.push(StmtKind::If(test, then, Some(replaced)).at(self.js_span(span)));
                return Ok(Value::Expr(Expr::js_undefined()));
            }
            if self.forwards_clone_from(ty, span)? {
                return Err(self.unsupported(
                    span,
                    &format!("`clone_from` of `{ty}`, whose std `clone_from` is its value's, which the crate's or std's own writes in place (#927): write the value's `clone_from` where both hold one, `match (&mut *dest, source) {{ (Some(to), Some(from)) => to.clone_from(from), _ => *dest = source.clone() }}`"),
                ));
            }
            self.replace_with_clone(state, (dest, source), (ty, def_id), arg_tys, span, out)?;
            Expr::js_undefined()
        }))
    }
}

//! `dest.clone_from(&source)` of a std type, as std's `clone_from` writes
//! it (#919, #922, #927, #928): by the one composition generic code's
//! dictionary asks too (`std_types/clone_from.rs`), its own in place,
//! `clone_into`'s; a `Box`'s and a `Reverse`'s value's; an `Option`'s
//! value's where both are `Some`; a value only the caller knows, its
//! dictionary's `clone_from`, asked first; else `*dest = source.clone()`,
//! what it held dropped.

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
            // Written over in place as std's `clone_from` writes it,
            // the one composition generic code's dictionary asks too
            // (`dictionary_clone_from`): of the type itself, a `Box`'s
            // or a `Reverse`'s value's, a type's own, or a value only
            // the caller knows, its dictionary's, asked first (#928);
            // else an `Option`'s value's of two `Some`s
            // (`option_clone_over`), a `Result`'s of two `Ok`s or two
            // `Err`s (`result_clone_over`, #927). Given the box generic
            // code has of `dest`, or one of its place here, written
            // back, where what's written reads it.
            let to = self.dest_place(&dest, ty, span)?;
            let given_box = match &dest {
                Value::Ref(place) => self.box_of_place(state, place),
                _ => Some(self.value_expr(dest.clone(), span)?),
            };
            let made = given_box.is_none().then(|| self.fresh(Role::To));
            let pointer = given_box.unwrap_or_else(|| Expr::var(made.as_ref().expect("a box made")));
            self.flush(state, out)?;
            // Read again by what's asked, each side and the clone; not where
            // it's written over whatever it holds, read once, `clone_into`'s.
            let once = matches!(
                self.written_over(ty, (to.clone(), pointer.clone()), source.clone(), span)?,
                Some((None, _))
            );
            let source = match source.reads_same() || once {
                true => source,
                false => self.spill(Role::Source, source, out),
            };
            let boxed = || crate::lower::mut_refs::boxed(to.clone());
            let mentions = |stmts: &[Stmt], name: &js::Name| {
                let mut read = false;
                js::visit_stmt_names(stmts, &mut |n| read |= n == name);
                read
            };
            if let Some((asked, written)) =
                self.written_over(ty, (to.clone(), pointer.clone()), source.clone(), span)?
            {
                // The box made of the place, where what's written
                // reads it, written back there, in that branch alone.
                let mut then = vec![StmtKind::Expr(written).at(self.js_span(span))];
                if let Some(name) = made.as_ref().filter(|name| mentions(&then, name)) {
                    then.insert(0, StmtKind::Const(name.into(), boxed()).at(self.js_span(span)));
                    then.push(
                        StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),
                    );
                }
                match asked {
                    Some(asked) => {
                        let mut replaced = Vec::new();
                        self.replace_with_clone(state, (dest, source), (ty, def_id), arg_tys, span, &mut replaced)?;
                        out.push(StmtKind::If(asked, then, Some(replaced)).at(self.js_span(span)));
                    }
                    None => out.extend(then),
                }
                return Ok(Value::Expr(Expr::js_undefined()));
            }
            // Of an `Option`, by its place, which holds a value, not an
            // object: a `Result` within it written there too. Not a
            // `Result` itself, an object a `&mut` is, which
            // `result_clone_over`'s `to = ..` would only rebind here,
            // whose own place this body can't tell from another's
            // (#927).
            let composed = |this: &mut Self, place: Expr, pointer: Expr| -> R<Option<Vec<Stmt>>> {
                this.option_clone_over(ty, (place, pointer), source.clone(), span)
            };
            if let Some(over) = composed(self, to.clone(), pointer.clone())? {
                // Of a box made of the place that what's written reads,
                // each branch written through it, its `value` the place,
                // then written back once.
                let Some(name) = made.as_ref().filter(|name| mentions(&over, name)) else {
                    out.extend(over);
                    return Ok(Value::Expr(Expr::js_undefined()));
                };
                let place = Expr::member(Expr::var(name), js::HELD);
                let over = composed(self, place, Expr::var(name))?.expect("composed as above");
                out.push(StmtKind::Const(name.into(), boxed()).at(self.js_span(span)));
                out.extend(over);
                out.push(StmtKind::Assign(to, Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)));
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

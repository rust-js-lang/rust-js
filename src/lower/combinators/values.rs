//! A combinator's values: what it derefs and copies, what it calls its
//! closure with, and what's dropped of what it doesn't keep.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a `value` of `ty` points at, as `*value` reads it: a counted
    /// `Rc`'s value, or the `deref()` of a type with its own `Deref`.
    pub(super) fn deref_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        // A counted `Rc`'s, its `value` (ADRs 0320, 0520).
        if let Some(pointee) = self.counted_rc(ty) {
            return Ok(self.counted_pointee(value, pointee));
        }
        // A `RefCell`'s guard's, what it guards (`guard_value`): `as_deref`
        // borrows it, so as a `Ref` does.
        if crate::lower::recognition::is_cell_guard(self.tcx, ty) {
            return Ok(self.guard_value(value, ty, false));
        }
        let deref = self.tcx.require_lang_item(LangItem::Deref, span);
        if self.has_user_impl(deref, ty) {
            let method = trait_method(self.tcx, deref, "deref");
            let args = self.args_of(deref, ty);
            if let Some(call) = self.trait_call(method, args, vec![value], span, out)? {
                return Ok(call);
            }
        }
        Err(self.unsupported(span, &format!("`as_deref` of a `{ty}`")))
    }

    /// `vec![item; n]`, of their values (ADR 0364), `n`'s value where
    /// rustc knows it, a constant's.
    pub(in crate::lower) fn copies_values(
        &mut self,
        (item, item_ty): (Expr, Ty<'tcx>),
        (n, known): (Expr, Option<u64>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A value with a destructor is owned once each, and dropped of none,
        // or of no room, which std checks first (ADR 0774), `$repeat`'s;
        // else the check is a statement, after what it's given runs.
        let drop = self.std_drop_function(item_ty, span)?;
        let size = self.room_item_size(item_ty);
        let fills = !self.needs_clone(item_ty) && drop.is_none();
        let repeats = !fills && (!self.structural_clone(item_ty) || drop.is_some());
        let (mut item, mut n) = (item, n);
        if !repeats {
            if !item.reads_same() {
                item = self.spill("item", item, out);
            }
            if !n.reads_same() {
                n = self.spill("n", n, out);
            }
            self.room_for_count((n.clone(), known), size.clone(), span, out);
        }
        if fills {
            let array = Expr::new_(Expr::var("Array"), vec![n]);
            return Ok(Expr::call(Expr::member(array, "fill"), vec![item]));
        }
        // A hand-written Clone (including one in a field) can change values
        // and run effects: clone n - 1 times, then move the original item.
        if repeats {
            let name = self.fresh("item");
            let mut body = Vec::new();
            let copy = self.clone_value(Expr::var(&name), item_ty, span, &mut body)?;
            body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
            let mut list = vec![item, n, size, Expr::arrow(vec![name.into()], body)];
            list.extend(drop);
            return Ok(Expr::call(Expr::helper("$repeat"), list));
        }
        let item = if item.reads_same() {
            item
        } else {
            self.spill("item", item, out)
        };
        let mut body = Vec::new();
        let copy = self.clone_value(item, item_ty, span, &mut body)?;
        body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
        let length = Expr::object(vec![Prop::Field("length".into(), n)]);
        let from = Expr::member(Expr::var("Array"), "from");
        Ok(Expr::call(from, vec![length, Expr::arrow(Vec::new(), body)]))
    }

    /// `f(args)`, with a closure that only returns written in place. One of
    /// statements gets a name first: `const f = (x) => { .. }; f(o)`.
    pub(in crate::lower) fn call_with(&mut self, f: Expr, args: Vec<Expr>, name: &str, out: &mut Vec<Stmt>) -> Expr {
        let args: Vec<Expr> = args
            .into_iter()
            .map(|a| if a.reads_same() { a } else { self.spill("value", a, out) })
            .collect();
        let applied = crate::lower::calls::apply(f.clone(), args.clone());
        match &applied.kind {
            js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Arrow(..)) => {
                let f = self.spill(name, f, out);
                Expr::call(f, args)
            }
            _ => applied,
        }
    }

    /// `name`, set by `then` where `test` holds, and otherwise to
    /// `otherwise`, and then `given_up`, the other variant's value, dropped,
    /// as std's drops it when the call returns (ADRs 0179, 0509): `let
    /// holds; if (r.TAG === "Ok") { holds = f(r._0); } else { holds = false;
    /// eDrop_drop(r._0); }`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn or_dropped(
        &mut self,
        test: Expr,
        name: String,
        then: Vec<Stmt>,
        given_up: (Expr, Ty<'tcx>),
        otherwise: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let js_span = self.js_span(span);
        let mut dropped = vec![StmtKind::Assign(Expr::var(&name), otherwise).at(js_span)];
        self.drop_value(given_up.0, given_up.1, span, &mut dropped)?;
        out.push(StmtKind::Let(name.clone(), None).at(js_span));
        out.push(StmtKind::If(test, then, Some(dropped)).at(js_span));
        Ok(Expr::var(&name))
    }

    /// `a.xor(b)`, or `a.zip(b)`, of `a`, an `Option`, its value and its
    /// `Some`'s type, and `b`, another, of type `other_ty`: what it makes,
    /// with what it gives up of values with a destructor dropped after, of
    /// `xor` both where both are `Some`, of `zip` the one that is where the
    /// other isn't (ADR 0498).
    pub(super) fn unpaired(
        &mut self,
        xor: bool,
        (a, a_value, a_ty): (Expr, Expr, Ty<'tcx>),
        (b, other_ty): (Expr, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let a_some = Expr::bin(Op::LooseNe, a.clone(), Expr::null());
        let b_some = Expr::bin(Op::LooseNe, b.clone(), Expr::null());
        let b_ty = self.option_of(other_ty).expect("an `Option`");
        let b_value = match self.boxed_payload(b_ty) {
            true => self.some_value(b.clone()),
            false => b.clone(),
        };
        let made = match xor {
            true => {
                let b_none = Expr::bin(Op::LooseEq, b.clone(), Expr::null());
                Expr::cond(a_some.clone(), Expr::cond(b_none, a, Expr::undefined()), b)
            }
            false => {
                let both = Expr::bin(Op::And, a_some.clone(), b_some.clone());
                Expr::cond(
                    both,
                    Expr::array(vec![a_value.clone(), b_value.clone()]),
                    Expr::undefined(),
                )
            }
        };
        if self.drops(a_ty) == Drops::Nothing && self.drops(b_ty) == Drops::Nothing {
            return Ok(made);
        }
        let made = self.spill("value", made, out);
        let js_span = self.js_span(span);
        let mut dropped_a = Vec::new();
        self.drop_value(a_value, a_ty, span, &mut dropped_a)?;
        let mut dropped_b = Vec::new();
        self.drop_value(b_value, b_ty, span, &mut dropped_b)?;
        let not = |e: Expr| Expr::unary(js::UnaryOp::Not, e);
        if xor {
            // `b` dropped even where `a`'s drop panics, as Rust's unwinding
            // drops it (ADR 0502).
            let both = Expr::bin(Op::And, a_some, b_some);
            let dropped = match dropped_b.is_empty() {
                true => dropped_a,
                false => vec![StmtKind::Try(dropped_a, dropped_b).at(js_span)],
            };
            out.push(StmtKind::If(both, dropped, None).at(js_span));
        } else {
            let only_a = Expr::bin(Op::And, a_some.clone(), not(b_some.clone()));
            out.push(StmtKind::If(only_a, dropped_a, None).at(js_span));
            let only_b = Expr::bin(Op::And, not(a_some), b_some);
            out.push(StmtKind::If(only_b, dropped_b, None).at(js_span));
        }
        Ok(made)
    }

    /// `test ? kept : otherwise`, where `given_up`, the other variant's
    /// value std drops, has nothing to drop; else `or_dropped`'s (ADRs 0179,
    /// 0490): `unwrap_or_default`'s `Err`, once the default's made (ADR
    /// 0509).
    pub(super) fn kept_or(
        &mut self,
        test: Expr,
        kept: Expr,
        given_up: (Expr, Ty<'tcx>),
        otherwise: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if self.drops(given_up.1) == Drops::Nothing {
            return Ok(Expr::cond(test, kept, otherwise));
        }
        let name = self.fresh("kept");
        let then = vec![StmtKind::Assign(Expr::var(&name), kept).at(js::Span::NONE)];
        self.or_dropped(test, name, then, given_up, otherwise, span, out)
    }
}

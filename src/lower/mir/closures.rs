//! A closure made in a body: its arrow, of its own MIR, and what it
//! captured.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A closure made here, an arrow of its own MIR: what it captured, as
    /// it's read where it's made, the places it borrows themselves, as JS's
    /// closures read theirs, and a value it took a copy of where the
    /// variable may change after.
    pub(super) fn mir_closure(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        captured: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let closure = self.mir_closure_fn(state, def_id, captured, span, out)?;
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        if !self.krate.closure_facts.clones_apart(def_id) {
            return self.closure_carrying_drop(closure, ty, span);
        }
        // One cloned apart: made by a function of the variables it took,
        // `make`, which its clone, `$cloning`'s, calls again with a clone of
        // each (ADR 0522).
        let taken: Vec<(String, Ty<'tcx>)> = (self.closure_held(def_id).into_iter())
            .filter_map(|(e, ty)| match e.kind {
                js::ExprKind::Var(name) => Some((name, ty)),
                _ => None,
            })
            .collect();
        let make = self.fresh("make");
        let mut cloned = Vec::new();
        let mut clones = Vec::new();
        for (name, ty) in &taken {
            clones.push(self.clone_value(Expr::var(name), *ty, span, &mut cloned)?);
        }
        cloned.push(StmtKind::Return(Some(Expr::call(Expr::var(&make), clones))).at(js::Span::NONE));
        let closure = Expr::call_helper("$cloning", vec![closure, Expr::arrow(Vec::new(), cloned)]);
        let closure = self.closure_carrying_drop(closure, ty, span)?;
        let params = taken.iter().map(|(name, _)| name.as_str().into()).collect();
        let maker = Expr::arrow(params, vec![StmtKind::Return(Some(closure)).at(js::Span::NONE)]);
        out.push(StmtKind::Let(make.clone(), Some(maker)).at(self.js_span(span)));
        Ok(Expr::call(
            Expr::var(&make),
            taken.iter().map(|(name, _)| Expr::var(name)).collect(),
        ))
    }

    /// The closure `closure`, of type `ty`: one its maker gives back, makes
    /// a `dyn Fn` or clones apart, holding a value with a destructor, carries
    /// its drop, what it took dropped, where it's made (ADRs 0484, 0521,
    /// 0522).
    fn closure_carrying_drop(&mut self, closure: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty::Closure(def_id, _) = *ty.kind() else {
            return Ok(closure);
        };
        if !self.krate.closure_facts.carries_its_drop(self.tcx, def_id)
            || self.drops(ty) == crate::lower::drops::Drops::Nothing
        {
            return Ok(closure);
        }
        let dropped = self.carried_drop(def_id, span)?;
        Ok(Expr::call_helper(
            "$dropping",
            vec![closure, Expr::arrow(Vec::new(), dropped)],
        ))
    }

    /// `mir_closure`'s function, of what it took.
    fn mir_closure_fn(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        captured: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let Some(body) = self.krate.closures.get(&def_id).copied() else {
            return Err(self.unsupported(span, "this closure, from its MIR"));
        };
        let Some(mir) = &body.mir else {
            return Err(self.unsupported(span, "this closure, without its MIR"));
        };
        let mut captures = Vec::new();
        let upvars = crate::lower::recognition::held_types(self.tcx.type_of(def_id).skip_binder())
            .map_or_else(Vec::new, |(_, held)| held.to_vec());
        for (i, value) in captured.into_iter().enumerate() {
            captures.push(match value {
                Value::Ref(place) | Value::Place(place) => (place, true),
                // A `&mut` to a variable of the maker's own, an object: the
                // variable, which the closure reads and assigns as JS's
                // closures do theirs.
                Value::Expr(e)
                    if matches!(
                        upvars.get(i).map(|t| t.kind()),
                        Some(ty::Ref(_, _, ty::Mutability::Mut))
                    ) && matches!(&e.kind, js::ExprKind::Var(name) if (state.locals.names.iter_enumerated())
                            .any(|(local, n)| n == name && !state.body.local_decls[local].ty.is_ref())) =>
                {
                    (e, true)
                }
                // One it took and changes is its own: a `let` of it, made here.
                value if writes_capture(&mir.body, i) => {
                    let e = self.value_expr(value, span)?;
                    self.flush(state, out)?;
                    let name = self.fresh(match &e.kind {
                        js::ExprKind::Var(name) => name,
                        _ => "captured",
                    });
                    out.push(StmtKind::Let(name.clone(), Some(e)).at(self.js_span(span)));
                    (Expr::var(&name), false)
                }
                value => {
                    let e = self.value_expr(value, span)?;
                    // Written again where it's made, or after: a copy, as it was.
                    let changes = matches!(&e.kind, js::ExprKind::Var(name)
                        if state.locals.names.iter_enumerated().any(|(l, n)| n == name
                            && super::queries::written_again(state, l)
                            && written_from(state.body, state.at, l)));
                    let e = if e.is_constant() || (matches!(e.kind, js::ExprKind::Var(_)) && !changes) {
                        e
                    } else {
                        self.flush(state, out)?;
                        self.spill("captured", e, out)
                    };
                    (e, false)
                }
            });
        }
        // What its drop drops, where it's made: what it holds, `closure_made`
        // (ADR 0098); a reference it holds drops nothing.
        let held: Vec<(Expr, Ty<'tcx>)> = (captures.iter().zip(&upvars))
            .map(|((e, _), &ty)| (e.clone(), ty))
            .collect();
        self.closure_made(def_id, held);
        // Its own THIR's shape hints, and what's known of it, where its body
        // is written; and whose body it is, its own, as `enter_body` has it:
        // `Location::caller()` in it is where it's written (ADR 0427).
        let outer = std::mem::replace(&mut self.thir, &body.thir);
        let outer_facts = std::mem::replace(&mut self.body_facts, &body.facts);
        let outer_owner = std::mem::replace(&mut self.body_owner, def_id);
        let lowered = self.mir_function(mir, Some(captures));
        self.thir = outer;
        self.body_facts = outer_facts;
        self.body_owner = outer_owner;
        let (mut params, mut stmts) = lowered?;
        // The functions written in it, in its body (ADR 0308).
        let written: Vec<rustc_span::def_id::DefId> = (self.krate.local_functions.keys())
            .filter(|&&id| self.tcx.parent(id) == def_id)
            .copied()
            .collect();
        // An `async` closure's `|_|`, which its MIR leaves unnamed, and its
        // THIR binds as `__arg0`: read nowhere, none either.
        while let Some(js::Pattern::Name(name)) = params.last()
            && (name == "_" || name.starts_with("_$"))
            && js::mentions_in(&stmts, name) == 0
        {
            params.pop();
        }
        // Its directives, `"use server";`, its body's first statements
        // (ADR 0349).
        let directives = self.directives(def_id);
        // An `async` body runs as soon as it's made: an async arrow, called
        // at once (ADR 0029).
        if self.tcx.coroutine_is_async(def_id) {
            self.place_written(written, &mut stmts);
            // What it gives, settled where a promise would adopt it (#172).
            if let ty::Coroutine(_, args) = self.tcx.type_of(def_id).skip_binder().kind()
                && self.may_be_thenable(args.as_coroutine().return_ty())
            {
                js::statement_lists(&mut stmts, &mut |list| {
                    for stmt in list.iter_mut() {
                        if let StmtKind::Return(Some(value)) = &mut stmt.kind {
                            let given = std::mem::replace(value, Expr::undefined());
                            *value = Expr::call_helper("$settled", vec![given]);
                        }
                    }
                });
            }
            return Ok(Expr::call(Expr::async_arrow(params, stmts), Vec::new()));
        }
        // An `async` closure's body is the one it makes: its async arrow's.
        if matches!(self.tcx.type_of(def_id).skip_binder().kind(), ty::CoroutineClosure(..))
            && let Some(mut body) = called_async_body(&mut stmts)
        {
            self.place_written(written, &mut body);
            body.splice(0..0, directives);
            return Ok(Expr::async_arrow(params, body));
        }
        // A closure of `()` that only spawns an async block is that block's
        // async arrow: it starts the same, and gives back a promise where
        // nothing reads its `()` (ADR 0257).
        if let ty::Closure(_, args) = self.tcx.type_of(def_id).skip_binder().kind()
            && written.is_empty()
            && directives.is_empty()
            && args.as_closure().sig().output().skip_binder().is_unit()
            && let [
                Stmt {
                    kind: StmtKind::Expr(call),
                    ..
                },
            ] = stmts.as_slice()
            && let js::ExprKind::Call(callee, given) = &call.kind
            && given.is_empty()
            && let js::ExprKind::AsyncArrow(inner, spawned) = &callee.kind
            && inner.is_empty()
        {
            return Ok(Expr::async_arrow(params, spawned.clone()));
        }
        self.place_written(written, &mut stmts);
        stmts.splice(0..0, directives);
        let forwarding = self.closure_calls_rust(body, false);
        let concise = self.closure_calls_for_nothing(body);
        if forwarding || concise {
            let span = self.js_span(span);
            if forwarding {
                self.locals.mir.forwarding.push(span);
            }
            if concise {
                self.locals.mir.concise.push(span);
            }
            return Ok(Expr {
                span,
                ..Expr::arrow(params, stmts)
            });
        }
        Ok(Expr::arrow(params, stmts))
    }

    /// A hole for each function of `written`, before the first statement of
    /// `out` written after it, or the `return` that ends it: a function
    /// declaration's, which JS hoists (ADR 0308).
    pub(super) fn place_written(&self, written: Vec<rustc_span::def_id::DefId>, out: &mut Vec<Stmt>) {
        let mut written: Vec<(u32, rustc_span::def_id::DefId)> = (written.into_iter())
            .map(|id| (self.js_span(self.tcx.def_span(id)).lo, id))
            .collect();
        written.sort_by_key(|&(at, _)| at);
        for (at, id) in written {
            let after = out.iter().position(|s| !s.span.is_none() && s.span.lo > at);
            let ends = matches!(out.last(), Some(s) if matches!(s.kind, StmtKind::Return(_)));
            let i = after.unwrap_or(out.len() - usize::from(ends));
            let hole = Expr {
                reads: false,
                kind: js::ExprKind::FunctionHole(id.index.as_u32()),
                span: js::Span::NONE,
            };
            out.insert(i, StmtKind::Expr(hole).at(js::Span::NONE));
        }
    }
}

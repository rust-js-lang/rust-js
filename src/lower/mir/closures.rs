//! A closure made in a body: its arrow, of its own MIR, and what it
//! captured.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A closure made here, an arrow of its own MIR: what it captured, as
    /// it's read where it's made, the places it borrows themselves, as JS's
    /// closures read theirs, and a value it took a copy of where the
    /// variable may change after. Of its type `ty` as this body makes it, in
    /// this body's terms: one std's MIR inlines, `map_fold`'s in `Map`'s
    /// `fold`, holds `F` and `G` of the `fold`, not of `map_fold`.
    pub(super) fn mir_closure(
        &mut self,
        state: &mut State<'_, 'tcx>,
        ty: Ty<'tcx>,
        captured: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let Some((def_id, upvars)) = crate::lower::recognition::held_types(ty) else {
            return Err(self.unsupported(span, "this closure, from its MIR"));
        };
        let closure = self.mir_closure_fn(state, ty, upvars, captured, span, out)?;
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
        ty: Ty<'tcx>,
        upvars: &'tcx ty::List<Ty<'tcx>>,
        captured: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let Some((def_id, _)) = crate::lower::recognition::held_types(ty) else {
            return Err(self.unsupported(span, "this closure, from its MIR"));
        };
        let Some(body) = self.krate.closures.get(&def_id).copied() else {
            return Err(self.unsupported(span, "this closure, from its MIR"));
        };
        let Some(mir) = &body.mir else {
            return Err(self.unsupported(span, "this closure, without its MIR"));
        };
        // One rustc's MIR inlined, made in another function than its own,
        // std's `flatten::<U, Acc>`'s in `FlattenCompat::rfold`: its body
        // read in its maker's terms, as its captures are (ADR 0899).
        let inlined;
        let mir = match self.inlined_closure_args(def_id, ty) {
            Some(args) => {
                inlined = mir.instantiated(self.tcx, args);
                &inlined
            }
            None => mir,
        };
        let mut captures = Vec::new();
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
        let held: Vec<(Expr, Ty<'tcx>)> = (captures.iter().zip(upvars))
            .map(|((e, _), ty)| (e.clone(), ty))
            .collect();
        self.closure_made(def_id, held);
        // Its own THIR's shape hints, and what's known of it, where its body
        // is written; and whose body it is, its own, as `enter_body` has it:
        // `Location::caller()` in it is where it's written (ADR 0427).
        let outer = std::mem::replace(&mut self.frame.thir, &body.thir);
        let outer_facts = std::mem::replace(&mut self.frame.facts, &body.facts);
        let outer_owner = std::mem::replace(&mut self.frame.owner, def_id);
        // Given a place found, what it's told of it (ADR 0642).
        let seed = state.facts.offsets().seed(def_id).cloned();
        let lowered = self.mir_function(mir, Some(captures), seed);
        self.frame.thir = outer;
        self.frame.facts = outer_facts;
        self.frame.owner = outer_owner;
        let (mut params, mut stmts) = lowered?;
        // The functions and `const`s written in it, in its body (ADRs
        // 0308, 0031).
        let written: Vec<rustc_span::def_id::DefId> = (self.krate.local_functions.keys())
            .chain(self.krate.local_consts.keys())
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
        if forwarding || concise.is_some() {
            let span = self.js_span(span);
            if forwarding {
                self.locals.mir.forwarding.push(span);
            }
            if let Some(written) = concise {
                self.locals.mir.concise.push((span, written));
            }
            return Ok(Expr {
                span,
                ..Expr::arrow(params, stmts)
            });
        }
        Ok(Expr::arrow(params, stmts))
    }

    /// Of a closure of type `ty`, `def_id`'s, made where its own function
    /// isn't, as rustc's MIR inlines that one into its caller: what its own
    /// function's generics are where it's made, its type's parent args.
    /// `None` of one made in its own function.
    fn inlined_closure_args(
        &self,
        def_id: rustc_span::def_id::DefId,
        ty: Ty<'tcx>,
    ) -> Option<ty::GenericArgsRef<'tcx>> {
        let own = self.tcx.typeck_root_def_id(def_id);
        if own == self.tcx.typeck_root_def_id(self.frame.owner) {
            return None;
        }
        let parent = match *ty.kind() {
            ty::Closure(_, args) => args.as_closure().parent_args(),
            ty::Coroutine(_, args) => args.as_coroutine().parent_args(),
            ty::CoroutineClosure(_, args) => args.as_coroutine_closure().parent_args(),
            _ => return None,
        };
        (parent.len() == self.tcx.generics_of(own).count()).then(|| self.tcx.mk_args(parent))
    }

    /// A hole for each function of `written`, before the first statement of
    /// `out` written after it, or the `return` that ends it: a function
    /// declaration's, which JS hoists (ADR 0308). Then one for each `const`
    /// of it, once the functions that may read it are there: where it's
    /// written too, or, where a statement before that reads it, before that
    /// one, as JS's `const` is read only once it's made, which Rust's is
    /// wherever its block is (ADR 0031, amended); one a function written
    /// there reads first in its block, as what's before may call it.
    pub(super) fn place_written(&self, written: Vec<rustc_span::def_id::DefId>, out: &mut Vec<Stmt>) {
        let mut written: Vec<(u32, rustc_span::def_id::DefId)> = (written.into_iter())
            .map(|id| (self.js_span(self.tcx.def_span(id)).lo, id))
            .collect();
        written.sort_by_key(|&(at, _)| at);
        let (consts, functions): (Vec<_>, Vec<_>) = written
            .into_iter()
            .partition(|(_, id)| self.krate.local_consts.contains_key(id));
        for (at, id) in functions.into_iter().chain(consts) {
            self.place_item(id, at, out);
        }
    }

    /// A hole for item `id`, written at `at`, in `out`, `place_written`'s.
    fn place_item(&self, id: rustc_span::def_id::DefId, at: u32, out: &mut Vec<Stmt>) {
        let hole = Expr {
            facts: js::Facts::NONE,
            kind: js::ExprKind::ItemHole(id.index.as_u32()),
            span: js::Span::NONE,
        };
        let hole = StmtKind::Expr(hole).at(js::Span::NONE);
        let name = &self.krate.fns[&id].name;
        let constant = self.krate.local_consts.get(&id);
        // What reads it: its name, or, of a `const`, a hole of a function
        // written here that reads it, filled later.
        let readers: Vec<u32> = (constant.into_iter().flat_map(|(_, readers)| readers))
            .map(|reader| reader.index.as_u32())
            .collect();
        let reads = |stmts: &[Stmt]| {
            let mut found = js::mentions_in(stmts, name);
            js::each_expr(stmts, &mut |e| {
                found += usize::from(matches!(e.kind, js::ExprKind::ItemHole(index) if readers.contains(&index)));
            });
            found
        };
        let uses = reads(out);
        // Where in a list it goes: before what's written after it, or what
        // reads a `const` first; first, of one a function written here
        // reads, which what's before may call.
        let called = (constant.into_iter().flat_map(|(_, readers)| readers))
            .any(|reader| self.krate.local_functions.contains_key(reader));
        let before = |list: &[Stmt]| {
            let written = written_before(list, at);
            match constant {
                Some(_) if called => Some(0),
                Some(_) => {
                    let first = list.iter().position(|s| reads(std::slice::from_ref(s)) > 0);
                    written.zip(first).map(|(w, f)| w.min(f)).or(written).or(first)
                }
                None => written,
            }
        };
        // In the block it's written in, a JS function declaration
        // being its block's alone (ADR 0696): `if (construct) {
        // function Fake() {..} .. }`; one written at its body's top is
        // there, not in the `if` that uses it.
        let block = self.js_span(match constant {
            Some((block, _)) => *block,
            None => self.krate.local_functions[&id],
        });
        // The lists holding them all nest, each in the one before,
        // walked outermost first: the first of them with a statement
        // written after it in its block is the block's.
        let (mut index, mut written_in) = (0, None);
        js::statement_lists(out, &mut |list| {
            if written_in.is_none()
                && uses > 0
                && reads(list) == uses
                && list
                    .iter()
                    .any(|s| !s.span.is_none() && s.span.lo > at && s.span.lo < block.hi)
            {
                written_in = Some(index);
            }
            index += 1;
        });
        if let Some(written_in) = written_in {
            let mut index = 0;
            js::statement_lists(out, &mut |list| {
                if index == written_in
                    && let Some(i) = before(list)
                {
                    list.insert(i, hole.clone());
                }
                index += 1;
            });
            return;
        }
        let ends = matches!(out.last(), Some(s) if matches!(s.kind, StmtKind::Return(_)));
        let last = out.len() - usize::from(ends);
        let i = before(out).map_or(last, |i| i.min(last));
        out.insert(i, hole);
    }
}

/// Where in `list` a function written at `at` goes: before its first
/// statement written after it.
fn written_before(list: &[Stmt], at: u32) -> Option<usize> {
    list.iter().position(|s| !s.span.is_none() && s.span.lo > at)
}

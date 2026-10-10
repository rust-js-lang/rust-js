//! A function's body from its MIR (ADR 0364): what runs, in the order it
//! runs, made structured JS. MIR says everything that happens: each value in
//! a temporary of its own, in order, each borrow, move and check. So the JS
//! is correct as it's made, and made readable only where that's proved to
//! change nothing:
//!
//! - **Structure**: blocks become labeled blocks and loops by Ramsey's
//!   "Beyond Relooper", which turns any reducible graph, as Rust's are, into
//!   `break` and `continue`.
//! - **Folding**: a temporary used once is written where it's used, where
//!   nothing it could see or that could see it runs between (`fold`).
//!
//! The function's THIR is beside it, for its shape: names and spans.

mod cfg;
mod iter;

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_index::IndexVec;
use rustc_middle::mir::interpret::GlobalId;
use rustc_middle::mir::{
    self, AggregateKind, AssertKind, BasicBlock, BinOp, BorrowKind, Const, ConstOperand, Local, LocalKind, Operand,
    Place, PlaceElem, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::recognition::Std;
use super::representation::{const_js, variant_field};
use super::std_types::number::NumOp;
use super::{Body, FnCx, LoweredFn, R, bindings, fn_def};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::runtime::Helper;

/// A body's MIR, as borrowck reads it (ADR 0364).
pub struct Mir<'tcx> {
    pub(super) body: mir::Body<'tcx>,
}

/// Whether bodies are lowered from their MIR: `RUST_JS_MIR=1`, while the
/// proof of concept runs (ADR 0364).
pub fn mir_mode() -> bool {
    std::env::var_os("RUST_JS_MIR").is_some_and(|v| v == "1")
}

/// What a temporary holds until it's used: a value, or a part of a
/// `format_args!` that only its `Arguments` shows.
#[derive(Clone)]
enum Value<'tcx> {
    Expr(Expr),
    /// An argument of a `format_args!`: how it's shown, its type, and it.
    Fmt(Std, Ty<'tcx>, Expr),
    /// An array of values, `format_args!`'s arguments among them.
    List(Vec<Value<'tcx>>),
    /// The variant of an enum, read to choose a branch: the enum and its type.
    Discriminant(Expr, Ty<'tcx>),
    /// A shared reference to a place, which nothing changes while it's held
    /// (borrowck says so): the place, read where it's used.
    Place(Expr),
    /// A `&mut` to a place JS can't share, a number's: the place, read and
    /// written through where it's used, or by a closure that captured it.
    Ref(Expr),
}

/// What's known of a body's locals.
struct Locals {
    /// Each one's JS name, empty for one of `()`, which holds nothing.
    names: IndexVec<Local, String>,
    /// How many times each is read, as an operand or a place's base.
    reads: IndexVec<Local, usize>,
    /// How many times each is assigned.
    writes: IndexVec<Local, usize>,
    /// Each one whose place is borrowed: it stays a variable.
    borrowed: IndexVec<Local, bool>,
    /// Each one declared already.
    declared: IndexVec<Local, bool>,
    /// Each one the program names, not a macro of std's.
    user: IndexVec<Local, bool>,
}

/// A body's lowering state.
struct State<'m, 'tcx> {
    body: &'m mir::Body<'tcx>,
    graph: cfg::Graph,
    locals: Locals,
    /// Temporaries made and not yet used, in the order they were made.
    pending: Vec<(Local, Value<'tcx>)>,
    /// The blocks being lowered whose label a `break` or a `continue` names.
    labels: Vec<BasicBlock>,
    /// The names of the locals borrowed, which a call may change.
    borrowed_names: std::collections::HashSet<String>,
    /// Of a closure: what it captured, each its environment's field.
    captures: Vec<Expr>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn lower_fn_mir(&mut self, body: &Body<'tcx>, mir: &Mir<'tcx>) -> R<LoweredFn> {
        let def_id = body.def_id.to_def_id();
        let span = self.tcx.def_span(def_id);
        if self.krate.fns.get(&def_id).is_none() {
            return Err(self.unsupported(span, "this item from its MIR"));
        }
        // Its dictionaries, for its bounds, after its parameters (ADR 0049).
        let evidence = self.evidence_params(def_id);
        let (mut params, out) = self.mir_function(mir, None)?;
        params.extend(evidence);
        Ok(LoweredFn {
            function: js::Function {
                name: self.krate.fns[&def_id].name.clone(),
                params,
                body: out,
                export: self.tcx.visibility(def_id).is_public()
                    && (self.tcx.def_kind(def_id) != rustc_hir::def::DefKind::AssocFn
                        || self.tcx.inherent_impl_of_assoc(def_id).is_some()),
                is_async: false,
                span: self.js_span(
                    self.tcx
                        .hir_span_with_body(self.tcx.local_def_id_to_hir_id(body.def_id)),
                ),
                name_span: self
                    .tcx
                    .def_ident_span(def_id)
                    .map_or(js::Span::NONE, |s| self.js_span(s)),
            },
            runtime: std::mem::take(&mut self.runtime),
            jsx: self.jsx,
            dependencies: self.dependencies.take(),
        })
    }

    /// A body's parameters and statements. A closure's, given what it
    /// captured, reads them in place of its environment's fields.
    fn mir_function(&mut self, mir: &Mir<'tcx>, captures: Option<Vec<Expr>>) -> R<(Vec<js::Pattern>, Vec<Stmt>)> {
        let mir_body = &mir.body;
        let mut state = State {
            body: mir_body,
            graph: cfg::Graph::of(mir_body),
            locals: self.mir_locals(mir_body),
            pending: Vec::new(),
            labels: Vec::new(),
            borrowed_names: Default::default(),
            captures: captures.clone().unwrap_or_default(),
        };
        state.borrowed_names = (state.locals.borrowed.iter_enumerated())
            .filter(|&(_, &borrowed)| borrowed)
            .map(|(local, _)| state.locals.names[local].clone())
            .collect();
        // A closure's first argument is its environment, read through its fields.
        let skip = usize::from(captures.is_some());
        let params: Vec<js::Pattern> = mir_body
            .args_iter()
            .skip(skip)
            .map(|local| {
                state.locals.declared[local] = true;
                js::Pattern::Name(match state.locals.names[local].as_str() {
                    "" => self.fresh("_"),
                    name => name.to_string(),
                })
            })
            .collect();
        let mut out = Vec::new();
        // Locals assigned more than once are `let`s, declared first.
        for local in mir_body.local_decls.indices() {
            if mir_body.local_kind(local) != LocalKind::Arg
                && !state.locals.names[local].is_empty()
                && state.locals.writes[local] > 1
            {
                out.push(StmtKind::Let(state.locals.names[local].clone(), None).at(js::Span::NONE));
                state.locals.declared[local] = true;
            }
        }
        let start = BasicBlock::from_usize(0);
        out.extend(self.mir_tree(&mut state, start)?);
        // A function's last `return;` says nothing.
        if matches!(
            out.last(),
            Some(Stmt {
                kind: StmtKind::Return(None),
                ..
            })
        ) {
            out.pop();
        }
        Ok((params, out))
    }

    /// A closure made here, an arrow of its own MIR: what it captured, as
    /// it's read where it's made, the places it borrows themselves, as JS's
    /// closures read theirs, and a value it took a copy of where the
    /// variable may change after.
    fn mir_closure(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        captured: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut captures = Vec::new();
        for value in captured {
            captures.push(match value {
                Value::Ref(place) | Value::Place(place) => place,
                value => {
                    let e = self.value_expr(value, span)?;
                    let changes = matches!(&e.kind, js::ExprKind::Var(name)
                        if state.locals.names.iter_enumerated().any(|(l, n)| n == name && state.locals.writes[l] > 1));
                    if e.is_constant() || (matches!(e.kind, js::ExprKind::Var(_)) && !changes) {
                        e
                    } else {
                        self.flush(state, out)?;
                        self.spill("captured", e, out)
                    }
                }
            });
        }
        let Some(body) = def_id
            .as_local()
            .and_then(|local| self.krate.closures.get(&local))
            .copied()
        else {
            return Err(self.unsupported(span, "this closure, from its MIR"));
        };
        let Some(mir) = &body.mir else {
            return Err(self.unsupported(span, "this closure, without its MIR"));
        };
        let (params, stmts) = self.mir_function(mir, Some(captures))?;
        Ok(Expr::arrow(params, stmts))
    }

    /// Each local's name, the user's where it has one, and how it's used.
    fn mir_locals(&mut self, body: &mir::Body<'tcx>) -> Locals {
        let n = body.local_decls.len();
        let mut locals = Locals {
            names: IndexVec::from_elem_n(String::new(), n),
            reads: IndexVec::from_elem_n(0, n),
            writes: IndexVec::from_elem_n(0, n),
            borrowed: IndexVec::from_elem_n(false, n),
            declared: IndexVec::from_elem_n(false, n),
            user: IndexVec::from_elem_n(false, n),
        };
        let mut user: IndexVec<Local, Option<String>> = IndexVec::from_elem_n(None, n);
        let sm = self.tcx.sess.source_map();
        for info in &body.var_debug_info {
            // A variable a std macro makes, `format_args!`'s `args`, isn't
            // one the program names.
            if let mir::VarDebugInfoContents::Place(place) = info.value
                && place.projection.is_empty()
                && user[place.local].is_none()
                && !info.source_info.span.in_external_macro(sm)
            {
                user[place.local] = Some(super::camel_case(info.name.as_str()));
                locals.user[place.local] = true;
            }
        }
        for (block, data) in body.basic_blocks.iter_enumerated() {
            if data.is_cleanup {
                continue;
            }
            let _ = block;
            for statement in &data.statements {
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        let (place, rvalue) = &**assign;
                        let place = match self.whole(body, place) {
                            true => &Place::from(place.local),
                            false => place,
                        };
                        count_place(&mut locals, place, true);
                        count_rvalue(&mut locals, rvalue);
                    }
                    StatementKind::SetDiscriminant { place, .. } => count_place(&mut locals, place, false),
                    _ => {}
                }
            }
            match &data.terminator().kind {
                TerminatorKind::SwitchInt { discr, .. } => count_operand(&mut locals, discr),
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    count_operand(&mut locals, func);
                    for arg in args {
                        count_operand(&mut locals, &arg.node);
                    }
                    count_place(&mut locals, destination, true);
                }
                TerminatorKind::Assert { cond, msg, .. } => {
                    count_operand(&mut locals, cond);
                    for operand in assert_operands(msg) {
                        count_operand(&mut locals, operand);
                    }
                }
                TerminatorKind::Return => locals.reads[mir::RETURN_PLACE] += 1,
                // A drop of what has nothing to drop does nothing (ADR 0098).
                TerminatorKind::Drop { place, .. }
                    if self.drops(place.ty(&body.local_decls, self.tcx).ty) != super::drops::Drops::Nothing =>
                {
                    count_place(&mut locals, place, false)
                }
                _ => {}
            }
        }
        for local in body.local_decls.indices() {
            if body.local_decls[local].ty.is_unit() || body.local_decls[local].ty.is_never() {
                continue;
            }
            let base = match (&user[local], body.local_kind(local)) {
                (Some(name), _) => name.clone(),
                (None, LocalKind::ReturnPointer) => "result".to_string(),
                (None, _) => format!("_{}", local.as_usize()),
            };
            locals.names[local] = self.fresh(&base);
        }
        locals
    }

    /// Whether `local` is a temporary written where it's read: made once,
    /// read once, never borrowed. The return place too, read by `return`.
    fn foldable(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let kind = state.body.local_kind(local);
        matches!(kind, LocalKind::Temp | LocalKind::ReturnPointer)
            && !state.locals.user[local]
            && state.locals.writes[local] == 1
            && state.locals.reads[local] == 1
            && !state.locals.borrowed[local]
    }

    // ── Structure ───────────────────────────────────────────────────────

    /// `block`, and what it dominates (Ramsey's `doTree`): a loop of it if
    /// an edge goes back to it.
    fn mir_tree(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let merges = state.graph.merge_children(block);
        if state.graph.headers.contains(&block) {
            self.flush(state, &mut Vec::new())?;
            state.labels.push(block);
            let body = self.mir_within(state, block, &merges)?;
            state.labels.pop();
            return Ok(vec![
                StmtKind::While {
                    label: Some(label(block)),
                    cond: Expr::bool(true),
                    body,
                }
                .at(js::Span::NONE),
            ]);
        }
        self.mir_within(state, block, &merges)
    }

    /// `block`'s code, inside a labeled block for each of `merges`, each
    /// followed by its own code (Ramsey's `nodeWithin`).
    fn mir_within(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, merges: &[BasicBlock]) -> R<Vec<Stmt>> {
        let Some((&merge, inner)) = merges.split_first() else {
            return self.mir_block(state, block);
        };
        state.labels.push(merge);
        let mut within = self.mir_within(state, block, inner)?;
        state.labels.pop();
        self.flush(state, &mut within)?;
        let mut out = vec![StmtKind::Labeled(label(merge), within).at(js::Span::NONE)];
        out.extend(self.mir_tree(state, merge)?);
        Ok(out)
    }

    /// Going from `from` to `to`: on, back to a loop's start, or out to a
    /// block that follows.
    fn mir_branch(&mut self, state: &mut State<'_, 'tcx>, from: BasicBlock, to: BasicBlock) -> R<Vec<Stmt>> {
        if state.graph.backward(from, to) {
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            out.push(StmtKind::Continue(Some(label(to))).at(js::Span::NONE));
            return Ok(out);
        }
        if state.graph.merges.contains(&to) {
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            out.push(StmtKind::Break(Some(label(to))).at(js::Span::NONE));
            return Ok(out);
        }
        self.mir_tree(state, to)
    }

    /// A block's statements, then where it goes.
    fn mir_block(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let data = &state.body.basic_blocks[block];
        let mut out = Vec::new();
        for statement in &data.statements {
            let span = statement.source_info.span;
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let (place, rvalue) = &**assign;
                    self.mir_assign(state, place, rvalue, span, &mut out)?;
                }
                StatementKind::SetDiscriminant { .. } => {
                    return Err(self.unsupported(span, "an enum made by parts, from its MIR"));
                }
                // What borrowck alone reads, and what says nothing that runs.
                StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType(..)
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Nop
                | StatementKind::BackwardIncompatibleDropHint { .. } => {}
                StatementKind::Intrinsic(_) => {
                    return Err(self.unsupported(span, "an intrinsic, from its MIR"));
                }
            }
        }
        let terminator = data.terminator();
        let span = terminator.source_info.span;
        match &terminator.kind {
            TerminatorKind::Goto { target } => out.extend(self.mir_branch(state, block, *target)?),
            TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
                out.extend(self.mir_branch(state, block, *real_target)?)
            }
            TerminatorKind::Return => {
                let ret = mir::RETURN_PLACE;
                let value = match state.locals.names[ret].is_empty() {
                    true => None,
                    false => Some(self.read_local(state, ret, &mut out)?),
                };
                self.flush(state, &mut out)?;
                out.push(StmtKind::Return(value).at(self.js_span(span)));
            }
            TerminatorKind::Unreachable => {
                self.flush(state, &mut out)?;
                out.push(
                    StmtKind::Throw(Expr::new_(
                        Expr::var("Error"),
                        vec![Expr::str("internal error: entered unreachable code")],
                    ))
                    .at(self.js_span(span)),
                );
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let subject = self.mir_operand(state, discr, &mut out)?;
                self.flush(state, &mut out)?;
                let ty = discr.ty(&state.body.local_decls, self.tcx);
                // The cases, those that can't be reached left out.
                let reached = |b: BasicBlock| !is_unreachable(state.body, b);
                let mut cases: Vec<(Expr, BasicBlock)> = Vec::new();
                for (value, target) in targets.iter() {
                    if reached(target) {
                        cases.push((self.switch_test(&subject, ty, value, span)?, target));
                    }
                }
                let otherwise = Some(targets.otherwise()).filter(|&b| reached(b));
                let mut chain: Option<Vec<Stmt>> = match otherwise {
                    Some(b) => Some(self.mir_branch(state, block, b)?),
                    None => None,
                };
                // Without an `otherwise`, the last case is what's left.
                if chain.is_none()
                    && let Some((_, last)) = cases.pop()
                {
                    chain = Some(self.mir_branch(state, block, last)?);
                }
                for (test, target) in cases.into_iter().rev() {
                    let then = self.mir_branch(state, block, target)?;
                    chain = Some(vec![StmtKind::If(test, then, chain).at(self.js_span(span))]);
                }
                out.extend(chain.unwrap_or_default());
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
                ..
            } => {
                let cond = self.mir_expr(state, cond, &mut out)?;
                let message = self.assert_message(state, msg, span, &mut out)?;
                self.flush(state, &mut out)?;
                let failed = match expected {
                    true => Expr::unary(js::UnaryOp::Not, cond),
                    false => cond,
                };
                let js_span = self.js_span(span);
                out.push(
                    StmtKind::If(
                        failed,
                        vec![StmtKind::Throw(Expr::new_(Expr::var("Error"), vec![message])).at(js_span)],
                        None,
                    )
                    .at(js_span),
                );
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Drop { place, target, .. } => {
                let ty = place.ty(&state.body.local_decls, self.tcx).ty;
                if self.drops(ty) != super::drops::Drops::Nothing {
                    return Err(self.unsupported(span, &format!("dropping a `{ty}` with a destructor, from its MIR")));
                }
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                fn_span,
                ..
            } => {
                let value = self.mir_call(state, func, args, *destination, *fn_span, &mut out)?;
                match target {
                    Some(target) => {
                        self.mir_store(state, *destination, value, span, &mut out)?;
                        out.extend(self.mir_branch(state, block, *target)?);
                    }
                    // A call that never returns, a panic's: what it does is all.
                    None => {
                        let value = self.value_expr(value, span)?;
                        self.flush(state, &mut out)?;
                        if !matches!(value.kind, js::ExprKind::Undefined) {
                            out.push(StmtKind::Expr(value).at(self.js_span(span)));
                        }
                    }
                }
            }
            TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::CoroutineDrop
            | TerminatorKind::Yield { .. }
            | TerminatorKind::InlineAsm { .. }
            | TerminatorKind::TailCall { .. } => {
                return Err(self.unsupported(span, "this terminator, from its MIR"));
            }
        }
        Ok(out)
    }

    // ── Folding ─────────────────────────────────────────────────────────

    /// Each temporary made and not yet used, made here, as `const`s: what
    /// runs next must not run before them, nor change what they read.
    fn flush(&mut self, state: &mut State<'_, 'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        for (local, value) in std::mem::take(&mut state.pending) {
            let expr = self.value_expr(value, Span::default())?;
            let name = state.locals.names[local].clone();
            state.locals.declared[local] = true;
            out.push(StmtKind::Const(name, expr).at(js::Span::NONE));
        }
        Ok(())
    }

    /// A local's value: its temporary's, taken where it's used, or its
    /// variable. A temporary used before others made after it, which would
    /// change their order, has all of them made first.
    fn read_local_value(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<Value<'tcx>> {
        if state.pending.iter().any(|(l, _)| *l == local) && self.admit(state, &[local], true, out)? {
            let at = state.pending.iter().position(|(l, _)| *l == local).expect("admitted");
            return Ok(state.pending.remove(at).1);
        }
        match state.locals.names[local].as_str() {
            "" => Ok(Value::Expr(Expr::undefined())),
            name => Ok(Value::Expr(Expr::var(name))),
        }
    }

    /// Operands read in this order, `f(a, b)`'s: the temporaries they take
    /// written in place where they're the latest made, in the order they
    /// were made, and what's read before the last of them can't change
    /// meanwhile, a constant or a local nothing borrows. Otherwise what's
    /// made is made first, and they're read by name.
    fn take_operands(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operands: &[&Operand<'tcx>],
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Value<'tcx>>> {
        let pending = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => state
                .pending
                .iter()
                .any(|(l, _)| *l == place.local)
                .then_some(place.local),
            _ => None,
        };
        let stable = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Constant(_) => true,
            Operand::Copy(place) | Operand::Move(place) => {
                place.projection.is_empty() && !state.locals.borrowed[place.local]
            }
            #[allow(unreachable_patterns)]
            _ => false,
        };
        let pends: Vec<Option<Local>> = operands.iter().map(|o| pending(state, o)).collect();
        let stables: Vec<bool> = operands.iter().map(|o| stable(state, o)).collect();
        let taken: Vec<Local> = pends.iter().flatten().copied().collect();
        // What's read before the last taken that does something can't change meanwhile.
        let last = pends.iter().rposition(|p| {
            p.is_some_and(|l| {
                let value = &state.pending.iter().find(|(p, _)| *p == l).expect("pending").1;
                !movable(state, value)
            })
        });
        let others_stable = last.is_none_or(|last| (0..last).all(|i| pends[i].is_some() || stables[i]));
        let admitted = taken.is_empty() || self.admit(state, &taken, others_stable, out)?;
        let mut values = Vec::new();
        for (operand, pend) in operands.iter().zip(pends) {
            values.push(match pend.filter(|_| admitted) {
                Some(local) => {
                    let at = state.pending.iter().position(|(l, _)| *l == local).expect("pending");
                    state.pending.remove(at).1
                }
                None => self.mir_operand(state, operand, out)?,
            });
        }
        Ok(values)
    }

    /// Whether temporaries `taken`, in the order they're read, can be
    /// written where they're read: those that do something read in the
    /// order they were made, what was made after the first of them movable,
    /// and `others_stable`, what else is read before them unchanged
    /// meanwhile. What was made before them is made first, here. If they
    /// can't be, everything made is, and false.
    fn admit(
        &mut self,
        state: &mut State<'_, 'tcx>,
        taken: &[Local],
        others_stable: bool,
        out: &mut Vec<Stmt>,
    ) -> R<bool> {
        let at = |l: Local| state.pending.iter().position(|(p, _)| *p == l).expect("pending");
        let acting: Vec<usize> = taken
            .iter()
            .map(|&l| at(l))
            .filter(|&i| !movable(state, &state.pending[i].1))
            .collect();
        let ordered = acting.windows(2).all(|w| w[0] < w[1]);
        let first = acting.iter().copied().min();
        let positions: Vec<usize> = taken.iter().map(|&l| at(l)).collect();
        let after_movable = first.is_none_or(|first| {
            (first..state.pending.len())
                .filter(|i| !positions.contains(i))
                .all(|i| movable(state, &state.pending[i].1))
        });
        if !(ordered && after_movable && (others_stable || acting.is_empty())) {
            self.flush(state, out)?;
            return Ok(false);
        }
        // What was made before the first that does something, made first.
        if let Some(first) = first {
            let before: Vec<usize> = (0..first).filter(|i| !positions.contains(i)).collect();
            if !before.iter().all(|&i| movable(state, &state.pending[i].1)) {
                let mut made = Vec::new();
                for i in before.into_iter().rev() {
                    made.push(state.pending.remove(i));
                }
                made.reverse();
                let kept = std::mem::replace(&mut state.pending, made);
                self.flush(state, out)?;
                state.pending = kept;
            }
        }
        Ok(true)
    }

    fn read_local(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<Expr> {
        let value = self.read_local_value(state, local, out)?;
        self.value_expr(value, state.body.local_decls[local].source_info.span)
    }

    /// A value as JS: a `format_args!` argument, the string it shows.
    fn value_expr(&mut self, value: Value<'tcx>, span: Span) -> R<Expr> {
        Ok(match value {
            Value::Expr(e) => e,
            Value::Fmt(Std::FmtDisplay, ty, e) => self.display_string(e, ty, span)?,
            Value::Fmt(Std::FmtDebug, ty, e) => self.debug_string(e, ty, span)?,
            Value::Fmt(_, _, e) => e,
            Value::List(items) => Expr::array(
                items
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?,
            ),
            Value::Discriminant(..) => return Err(self.unsupported(span, "an enum's discriminant read as a value")),
            Value::Place(e) => e,
            Value::Ref(_) => return Err(self.unsupported(span, "a `&mut` of a number kept or given, from its MIR")),
        })
    }

    // ── Statements ──────────────────────────────────────────────────────

    fn mir_assign(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let value = self.mir_rvalue(state, rvalue, span, out)?;
        self.mir_store(state, *place, value, span, out)
    }

    /// `place = value`: a temporary's value kept for where it's used, a
    /// variable's `const` where it's made once, or an assignment.
    fn mir_store(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let local = place.local;
        let place = match self.whole(state.body, &place) {
            true => Place::from(local),
            false => place,
        };
        if place.projection.is_empty() && state.locals.names[local].is_empty() {
            // Of `()`: what made it ran, and it holds nothing.
            let expr = self.value_expr(value, span)?;
            if expr.has_effects() {
                self.flush(state, out)?;
                out.push(StmtKind::Expr(expr).at(self.js_span(span)));
            }
            return Ok(());
        }
        if place.projection.is_empty() && self.foldable(state, local) {
            state.pending.push((local, value));
            return Ok(());
        }
        let expr = self.value_expr(value, span)?;
        let js_span = self.js_span(span);
        if place.projection.is_empty() && !state.locals.declared[local] {
            self.flush(state, out)?;
            state.locals.declared[local] = true;
            out.push(StmtKind::Const(state.locals.names[local].clone(), expr).at(js_span));
            return Ok(());
        }
        // The value is made before the place is reached, as Rust makes them.
        let mut value = expr;
        if !place.projection.is_empty() && value.has_effects() {
            value = self.spill("value", value, out);
        }
        let target = self.mir_place(state, place, span, out)?;
        self.flush(state, out)?;
        out.push(StmtKind::Assign(target, value).at(js_span));
        Ok(())
    }

    /// A place, for a read or a write.
    fn mir_place(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut value = if place.projection.is_empty() {
            match state.locals.names[place.local].as_str() {
                "" => Expr::undefined(),
                name => Expr::var(name),
            }
        } else if matches!(
            state.body.local_decls[place.local].ty.peel_refs().kind(),
            ty::Closure(..)
        ) && !state.captures.is_empty()
        {
            // A closure's environment, read only through its fields.
            Expr::undefined()
        } else {
            match self.read_local_value(state, place.local, out)? {
                // Through a `&mut` to a place: the place.
                Value::Ref(place) => place,
                value => self.value_expr(value, span)?,
            }
        };
        let mut ty = mir::PlaceTy::from_ty(state.body.local_decls[place.local].ty);
        for elem in place.projection {
            value = match (ty.ty.peel_refs().kind(), elem) {
                // Of a closure's environment, `(*_1).0`: what it captured.
                (ty::Closure(..), PlaceElem::Field(field, _)) if !state.captures.is_empty() => {
                    state.captures[field.as_usize()].clone()
                }
                _ => self.project_mir(state, value, ty, elem, span, out)?,
            };
            ty = ty.projection_ty(self.tcx, elem);
        }
        Ok(value)
    }

    fn project_mir(
        &mut self,
        state: &mut State<'_, 'tcx>,
        base: Expr,
        ty: mir::PlaceTy<'tcx>,
        elem: PlaceElem<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        Ok(match elem {
            // A reference is what it refers to (ADR 0023), and so is a box.
            PlaceElem::Deref if ty.ty.is_ref() || ty.ty.is_box() => base,
            PlaceElem::Field(field, _) => self.mir_field(base, ty, field, span)?,
            PlaceElem::Downcast(..) | PlaceElem::OpaqueCast(_) => base,
            PlaceElem::Index(local) => {
                let index = self.read_local(state, local, out)?;
                Expr::index(base, index)
            }
            PlaceElem::ConstantIndex {
                offset,
                from_end: false,
                ..
            } => Expr::index(base, Expr::int(offset.into())),
            _ => return Err(self.unsupported(span, "this place, from its MIR")),
        })
    }

    /// A field: of a struct or a tuple by its shape, of a variant as its
    /// enum holds it, and of what's the value it holds, itself.
    fn mir_field(&mut self, base: Expr, ty: mir::PlaceTy<'tcx>, field: FieldIdx, span: Span) -> R<Expr> {
        let i = field.as_usize();
        if let ty::Closure(..) = ty.ty.kind() {
            return Err(self.unsupported(span, "a closure's field read outside it, from its MIR"));
        }
        if self.transparent(ty.ty) {
            return Ok(base);
        }
        match (ty.ty.kind(), ty.variant_index) {
            (ty::Adt(adt, _), Some(variant)) if adt.is_enum() => {
                if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
                    let inner = self.option_of(ty.ty).expect("an `Option`");
                    return Ok(match self.boxed_payload(inner) {
                        true => self.some_value(base),
                        false => base,
                    });
                }
                if bindings::is_untagged(self.tcx, adt.did()) {
                    return Ok(base);
                }
                Ok(Expr::member(base, variant_field(self.tcx, adt.variant(variant), i)))
            }
            (ty::Tuple(_) | ty::Adt(..), _) => Ok(self.project(base, ty.ty, i)),
            _ => Err(self.unsupported(span, "this field, from its MIR")),
        }
    }

    /// Whether `place` is its local's value, through what's the value it
    /// holds: a box's, or a `MaybeUninit`'s; and, of a read, a reference's.
    fn whole(&self, body: &mir::Body<'tcx>, place: &Place<'tcx>) -> bool {
        self.through_values(body, place, false)
    }

    fn through_values(&self, body: &mir::Body<'tcx>, place: &Place<'tcx>, read: bool) -> bool {
        let mut ty = mir::PlaceTy::from_ty(body.local_decls[place.local].ty);
        for elem in place.projection {
            let through = match elem {
                PlaceElem::Deref => ty.ty.is_box() || (read && ty.ty.is_ref()),
                PlaceElem::Field(..) => self.transparent(ty.ty),
                _ => false,
            };
            if !through {
                return false;
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        true
    }

    // ── Values ──────────────────────────────────────────────────────────

    fn mir_operand(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        match operand {
            // A local's value, read through references to it, as it is.
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => {
                self.read_local_value(state, place.local, out)
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let span = state.body.local_decls[place.local].source_info.span;
                Ok(Value::Expr(self.mir_place(state, *place, span, out)?))
            }
            Operand::Constant(c) => Ok(Value::Expr(self.mir_const(state, c)?)),
            #[allow(unreachable_patterns)]
            _ => Err(self.unsupported(Span::default(), "this operand, from its MIR")),
        }
    }

    fn mir_expr(&mut self, state: &mut State<'_, 'tcx>, operand: &Operand<'tcx>, out: &mut Vec<Stmt>) -> R<Expr> {
        let value = self.mir_operand(state, operand, out)?;
        self.value_expr(value, Span::default())
    }

    /// A constant's JS, by its value tree.
    fn mir_const(&mut self, state: &State<'_, 'tcx>, c: &ConstOperand<'tcx>) -> R<Expr> {
        let ty = c.ty();
        if ty.is_unit() || matches!(ty.kind(), ty::FnDef(..)) {
            return Ok(Expr::undefined());
        }
        let _ = state;
        self.mir_const_value(c.const_, ty, c.span)
            .and_then(|value| const_js(self.tcx, value))
            .ok_or_else(|| self.unsupported(c.span, "this constant, from its MIR"))
    }

    fn mir_const_value(&self, c: Const<'tcx>, ty: Ty<'tcx>, span: Span) -> Option<ty::Value<'tcx>> {
        match c {
            Const::Ty(_, ct) => ct.try_to_value(),
            Const::Unevaluated(uv, _) => {
                let instance = ty::Instance::try_resolve(self.tcx, self.typing_env, uv.def, uv.args).ok()??;
                let cid = GlobalId {
                    instance,
                    promoted: uv.promoted,
                };
                let inputs = self.tcx.erase_and_anonymize_regions(
                    self.typing_env
                        .with_post_analysis_normalized(self.tcx)
                        .as_query_input(cid),
                );
                let valtree = self.tcx.at(span).eval_to_valtree(inputs).ok()?;
                Some(ty::Value { ty, valtree })
            }
            Const::Val(value, _) => {
                if let Some(scalar) = value.try_to_scalar_int() {
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_scalar_int(self.tcx, scalar),
                    });
                }
                // A reference to bytes rustc made, a `format_args!` template's.
                if let mir::ConstValue::Scalar(mir::interpret::Scalar::Ptr(ptr, _)) = value
                    && let ty::Ref(_, inner, _) = ty.kind()
                    && let ty::Array(item, len) = inner.kind()
                    && *item == self.tcx.types.u8
                {
                    let (provenance, offset) = ptr.into_raw_parts();
                    let alloc = self.tcx.global_alloc(provenance.alloc_id()).unwrap_memory();
                    let len = len.try_to_target_usize(self.tcx)? as usize;
                    let start = offset.bytes_usize();
                    let bytes = alloc
                        .inner()
                        .inspect_with_uninit_and_ptr_outside_interpreter(start..start + len);
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_raw_bytes(self.tcx, bytes),
                    });
                }
                if !matches!(value, mir::ConstValue::Slice { .. }) {
                    return None;
                }
                let bytes = value.try_get_slice_bytes_for_diagnostics(self.tcx)?;
                Some(ty::Value {
                    ty,
                    valtree: ty::ValTree::from_raw_bytes(self.tcx, bytes),
                })
            }
        }
    }

    fn mir_rvalue(
        &mut self,
        state: &mut State<'_, 'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        let decls = &state.body.local_decls;
        Ok(Value::Expr(match rvalue {
            Rvalue::Use(operand, _) => return self.mir_operand(state, operand, out),
            Rvalue::CopyForDeref(place) if self.through_values(state.body, place, true) => {
                return self.read_local_value(state, place.local, out);
            }
            Rvalue::CopyForDeref(place) => self.mir_place(state, *place, span, out)?,
            // A reference is what it refers to (ADR 0023): a `&mut` to a
            // value JS can't share, a number's, needs what isn't made yet.
            Rvalue::Ref(_, kind, place) => {
                let pointee = place.ty(decls, tcx).ty;
                if matches!(kind, BorrowKind::Mut { .. })
                    && !place.is_indirect()
                    && !self.is_object(pointee)
                    && !matches!(pointee.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..))
                {
                    return Ok(Value::Ref(self.mir_place(state, *place, span, out)?));
                }
                if self.through_values(state.body, place, true) {
                    let value = self.read_local_value(state, place.local, out)?;
                    return Ok(match (kind, value) {
                        (BorrowKind::Shared, Value::Expr(e)) if e.reads_same() => Value::Place(e),
                        (_, value) => value,
                    });
                }
                let place_expr = self.mir_place(state, *place, span, out)?;
                if matches!(kind, BorrowKind::Shared) && place_expr.reads_same() {
                    return Ok(Value::Place(place_expr));
                }
                place_expr
            }
            Rvalue::BinaryOp(op, operands) => {
                let (l, r) = &**operands;
                let ty = l.ty(decls, tcx);
                let [l, r]: [Value<'tcx>; 2] = self.take_operands(state, &[l, r], out)?.try_into().ok().expect("two");
                let l = self.value_expr(l, span)?;
                let r = self.value_expr(r, span)?;
                match op {
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow => {
                        return Err(self.unsupported(span, "arithmetic checked for overflow, from its MIR"));
                    }
                    BinOp::Cmp => return Err(self.unsupported(span, "`cmp`, from its MIR")),
                    _ => self.binary(*op, l, r, None, ty, span)?,
                }
            }
            Rvalue::UnaryOp(op, operand) => {
                let ty = operand.ty(decls, tcx);
                let a = self.mir_expr(state, operand, out)?;
                match op {
                    UnOp::Not | UnOp::Neg => self.unary(*op, a, ty, span)?,
                    UnOp::PtrMetadata => Expr::member(a, "length"),
                }
            }
            // A reference made another kind of reference is the same value:
            // an array's a slice's (ADR 0023). One made a `dyn` isn't yet.
            Rvalue::Cast(mir::CastKind::PointerCoercion(coercion, _), operand, to) => {
                use rustc_middle::ty::adjustment::PointerCoercion;
                let to_dyn = to
                    .builtin_deref(true)
                    .is_some_and(|pointee| matches!(pointee.kind(), ty::Dynamic(..)));
                match coercion {
                    PointerCoercion::Unsize if to_dyn => {
                        return Err(self.unsupported(span, &format!("a `{to}`, from its MIR")));
                    }
                    PointerCoercion::Unsize | PointerCoercion::MutToConstPointer | PointerCoercion::ArrayToPointer => {
                        return self.mir_operand(state, operand, out);
                    }
                    PointerCoercion::ReifyFnPointer(_)
                    | PointerCoercion::UnsafeFnPointer
                    | PointerCoercion::ClosureFnPointer(_) => {
                        return Err(self.unsupported(span, "a function made a pointer, from its MIR"));
                    }
                }
            }
            Rvalue::Cast(_, operand, to) => {
                let from = operand.ty(decls, tcx);
                let v = self.mir_expr(state, operand, out)?;
                if from == *to {
                    v
                } else if to.is_char() {
                    Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![v])
                } else {
                    self.cast(v, from, *to, span)?
                }
            }
            Rvalue::Discriminant(place) => {
                let ty = place.ty(decls, tcx).ty;
                let subject = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Discriminant(subject, ty));
            }
            Rvalue::Aggregate(kind, operands) => return self.mir_aggregate(state, kind, operands, span, out),
            _ => return Err(self.unsupported(span, "this value, from its MIR")),
        }))
    }

    fn mir_aggregate(
        &mut self,
        state: &mut State<'_, 'tcx>,
        kind: &AggregateKind<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let operands: Vec<&Operand<'tcx>> = operands.iter().collect();
        let values = self.take_operands(state, &operands, out)?;
        match kind {
            AggregateKind::Array(_) => Ok(Value::List(values)),
            AggregateKind::Tuple if values.is_empty() => Ok(Value::Expr(Expr::undefined())),
            AggregateKind::Tuple => Ok(Value::List(values)),
            AggregateKind::Adt(did, variant_index, args, _, _) => {
                let adt = self.tcx.adt_def(*did);
                let ty = Ty::new_adt(self.tcx, adt, args);
                let exprs = values
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?;
                Ok(Value::Expr(self.mir_adt(adt, *variant_index, ty, exprs, span)?))
            }
            AggregateKind::Closure(def_id, _) => Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?)),
            _ => Err(self.unsupported(span, "this aggregate, from its MIR")),
        }
    }

    /// A struct's or a variant's value, as the THIR's `adt` makes it.
    fn mir_adt(
        &mut self,
        adt: ty::AdtDef<'tcx>,
        variant_index: VariantIdx,
        ty: Ty<'tcx>,
        fields: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        let variant = adt.variant(variant_index);
        if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
            return Ok(match fields.into_iter().next() {
                Some(value) if self.boxed_payload(self.option_of(ty).expect("an `Option`")) => self.some(value),
                Some(value) => value,
                None => Expr::undefined(),
            });
        }
        if adt.is_enum() {
            if bindings::is_untagged(self.tcx, adt.did()) {
                return fields
                    .into_iter()
                    .next()
                    .ok_or_else(|| self.unsupported(span, "an untagged enum's variant without fields, from its MIR"));
            }
            if variant.fields.is_empty() {
                return Ok(bindings::unit_variant(self.tcx, adt.did(), variant));
            }
            let props = std::iter::once(Prop::Field(
                bindings::tag_key(self.tcx, adt.did()),
                bindings::variant_tag(self.tcx, variant),
            ))
            .chain(
                fields
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| Prop::Field(variant_field(self.tcx, variant, i), v)),
            )
            .collect();
            return Ok(Expr::object(props));
        }
        match self.shape(ty) {
            super::Shape::Array(_) => Ok(Expr::array(fields)),
            super::Shape::Object(names) => Ok(Expr::object(
                names
                    .into_iter()
                    .zip(fields)
                    .map(|((name, field_ty), value)| Prop::Field(name, self.holding(value, field_ty)))
                    .collect(),
            )),
            super::Shape::Other if fields.is_empty() => {
                Ok(bindings::unit_name(self.tcx, adt.did()).map_or_else(Expr::undefined, Expr::str))
            }
            super::Shape::Other => Err(self.unsupported(span, &format!("a `{ty}`, from its MIR"))),
        }
    }

    /// The test that `subject`, of type `ty`, is `value`: of an enum's
    /// discriminant, its variant.
    fn switch_test(&mut self, subject: &Value<'tcx>, ty: Ty<'tcx>, value: u128, span: Span) -> R<Expr> {
        match subject {
            Value::Discriminant(enum_value, enum_ty) => {
                let ty::Adt(adt, _) = enum_ty.kind() else {
                    return Err(self.unsupported(span, "a discriminant of this type, from its MIR"));
                };
                let variant = adt
                    .discriminants(self.tcx)
                    .find(|(_, d)| d.val == value)
                    .map(|(v, _)| v)
                    .ok_or_else(|| self.unsupported(span, "this discriminant, from its MIR"))?;
                self.variant_test(enum_value.clone(), *enum_ty, *adt, variant, span)
            }
            _ => {
                let subject = self.value_expr(subject.clone(), span)?;
                if ty.is_bool() {
                    return Ok(match value {
                        0 => Expr::unary(js::UnaryOp::Not, subject),
                        _ => subject,
                    });
                }
                if ty.is_char() {
                    let c = char::from_u32(value as u32).ok_or_else(|| self.unsupported(span, "this `char`"))?;
                    return Ok(Expr::bin(Op::Eq, subject, Expr::str(c.to_string())));
                }
                let num = self.num(ty, span)?;
                let literal = match num.signed() {
                    true => {
                        let bits = num.bits();
                        let v = ((value << (128 - bits)) as i128) >> (128 - bits);
                        num.literal(v)
                    }
                    false => num.literal(value as i128),
                };
                Ok(Expr::bin(Op::Eq, subject, literal))
            }
        }
    }

    /// That `subject` is `variant` of its enum, as a pattern tests it.
    fn variant_test(
        &mut self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
        span: Span,
    ) -> R<Expr> {
        let def = adt.variant(variant);
        if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
            let inner = self.option_of(ty).expect("an `Option`");
            return Ok(match self.tcx.is_lang_item(def.def_id, LangItem::OptionSome) {
                true => self.present(subject, inner),
                false => self.absent(subject, inner),
            });
        }
        if bindings::is_untagged(self.tcx, adt.did()) || bindings::is_tagged_otherwise(self.tcx, adt.did(), def) {
            return Err(self.unsupported(span, "this enum's variant tested, from its MIR"));
        }
        if let Some(n) = super::recognition::ordering_value(self.tcx, adt.did(), def.name) {
            return Ok(Expr::bin(Op::Eq, subject, Expr::int(n)));
        }
        let name = bindings::variant_tag(self.tcx, def);
        let unit = def.fields.is_empty() && bindings::declared_tag(self.tcx, adt.did()).is_none();
        Ok(match unit {
            true => Expr::bin(Op::Eq, subject, name),
            false => Expr::bin(
                Op::Eq,
                Expr::member(subject, bindings::tag_key(self.tcx, adt.did())),
                name,
            ),
        })
    }

    /// What a failed check says, as Rust's panic does.
    fn assert_message(
        &mut self,
        state: &mut State<'_, 'tcx>,
        msg: &AssertKind<Operand<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        Ok(match msg {
            AssertKind::BoundsCheck { len, index } => {
                let [len, index]: [Value<'tcx>; 2] = self
                    .take_operands(state, &[len, index], out)?
                    .try_into()
                    .ok()
                    .expect("two");
                let len = self.value_expr(len, span)?;
                let index = self.value_expr(index, span)?;
                let usize = self.tcx.types.usize;
                let len = self.display_string(len, usize, span)?;
                let index = self.display_string(index, usize, span)?;
                super::display::join(vec![
                    Expr::str("index out of bounds: the len is "),
                    len,
                    Expr::str(" but the index is "),
                    index,
                ])
            }
            AssertKind::Overflow(..)
            | AssertKind::OverflowNeg(_)
            | AssertKind::DivisionByZero(_)
            | AssertKind::RemainderByZero(_) => {
                for operand in assert_operands(msg) {
                    self.mir_expr(state, operand, out)?;
                }
                Expr::str(match msg {
                    AssertKind::Overflow(op, ..) => overflow_message(*op),
                    AssertKind::OverflowNeg(_) => "attempt to negate with overflow",
                    AssertKind::DivisionByZero(_) => "attempt to divide by zero",
                    _ => "attempt to calculate the remainder with a divisor of zero",
                })
            }
            _ => return Err(self.unsupported(span, "this check, from its MIR")),
        })
    }

    // ── Calls ───────────────────────────────────────────────────────────

    fn mir_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let decls = &state.body.local_decls;
        let Some((def_id, generic_args)) = fn_def(func.ty(decls, self.tcx)) else {
            return Err(self.unsupported(span, "calling a function value, from its MIR"));
        };
        let arg_tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
        let output = destination.ty(decls, self.tcx).ty;
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let values = self.take_operands(state, &operands, out)?;
        let tcx = self.tcx;
        // `x.into()` is the `From::from(x)` it calls, the crate's own (ADR 0052).
        let (def_id, generic_args) = self
            .resolve_into(def_id, generic_args)
            .unwrap_or((def_id, generic_args));
        // Calling a closure, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`.
        if let Some(fn_trait) = tcx.trait_of_assoc(def_id)
            && tcx.fn_trait_kind_from_def_id(fn_trait).is_some()
        {
            let mut values = values.into_iter();
            let callee = self.value_expr(values.next().expect("the closure"), span)?;
            let list = match values.next() {
                Some(Value::List(items)) => items,
                Some(Value::Expr(e)) if matches!(e.kind, js::ExprKind::Undefined) => Vec::new(),
                _ => return Err(self.unsupported(span, "this closure call, from its MIR")),
            };
            let mut exprs = list
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            // Not a closure of the crate's: a `dyn Fn`, a generic one or a
            // pointer, which may be JS's (ADR 0330).
            if !matches!(arg_tys[0].peel_refs().kind(), ty::Closure(..)) {
                super::calls::given_to_js(&mut exprs);
            }
            return Ok(Value::Expr(Expr::call(callee, exprs)));
        }
        // `a += b` of a number or a string, through its `&mut`: the place
        // assigned, as the operator's is.
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && let Some(op) = super::recognition::assign_operator(tcx, trait_id)
            && let [place, rhs] = &values[..]
            && let Value::Ref(place) = place
        {
            let ty = arg_tys[0].peel_refs();
            let rhs = self.value_expr(rhs.clone(), span)?;
            let value = if self.is_lang_adt(ty, LangItem::String) && op == BinOp::Add {
                Expr::bin(Op::Add, place.clone(), rhs)
            } else if super::representation::Num::of(ty).is_some() {
                self.binary(op, place.clone(), rhs, None, ty, span)?
            } else {
                return Err(self.unsupported(
                    span,
                    &format!("`{}` of a `{ty}`, from its MIR", tcx.def_path_str(def_id)),
                ));
            };
            self.flush(state, out)?;
            out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));
            return Ok(Value::Expr(Expr::undefined()));
        }
        // A std iterator's method: a JS iterator's, lazy as Rust's.
        if tcx.trait_of_assoc(def_id).is_some() {
            let exprs = values
                .clone()
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            if let Some(call) = self.mir_iter_call(def_id, &arg_tys, exprs, output, span)? {
                return Ok(Value::Expr(call));
            }
        }
        // A trait's method: its impl's, a dictionary's, or what rust-js
        // writes itself, `==` of a struct say (ADRs 0049, 0052).
        let known = self.recognition().classify(def_id, generic_args);
        if !matches!(known, Some(Std::Any(_)))
            && let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && (super::recognition::operational(tcx, self.krate.foreign, trait_id)
                || self
                    .resolve_instance(def_id, generic_args)?
                    .is_some_and(|i| self.is_rust_fn(i.def_id())))
        {
            let exprs = values
                .clone()
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            if let Some(call) = self.trait_call(def_id, generic_args, exprs, span, out)? {
                return Ok(Value::Expr(call));
            }
        }
        // The crate's own function, given its dictionaries (ADR 0049).
        if self.is_rust_fn(def_id) && tcx.trait_of_assoc(def_id).is_none() && !bindings::is_binding(tcx, def_id) {
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            exprs.extend(self.evidence_args(def_id, generic_args, span)?);
            let called = Expr::call(self.fn_ref(def_id), exprs);
            return Ok(Value::Expr(self.fmt_result_value(def_id, generic_args, called)));
        }
        if let Some(known) = known {
            return self.mir_std_call(
                state,
                (known, def_id),
                generic_args,
                &arg_tys,
                values,
                output,
                span,
                out,
            );
        }
        Err(self.unsupported(
            span,
            &format!("calling `{}`, from its MIR", self.tcx.def_path_str(def_id)),
        ))
    }

    /// A std function of its arguments' values, as THIR's are lowered once
    /// they're values (`std_values`).
    #[allow(clippy::too_many_arguments)]
    fn std_by_values(
        &mut self,
        known: Std,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        arg_tys: &[Ty<'tcx>],
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let boxed = self.option_of(output).is_some_and(|inner| self.boxed_payload(inner));
        let call = super::calls::Call {
            fun: None,
            def_id,
            generic_args,
            args: &[],
            tys: arg_tys,
            discarded: false,
            span,
        };
        self.std_values(known, call, values, boxed, output, out)
    }

    /// A std function rust-js knows, given what MIR made of its arguments.
    #[allow(clippy::too_many_arguments)]
    fn mir_std_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id): (Std, rustc_span::def_id::DefId),
        generic_args: ty::GenericArgsRef<'tcx>,
        arg_tys: &[Ty<'tcx>],
        values: Vec<Value<'tcx>>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let first_ty = || generic_args.types().next();
        let mut values = values.into_iter();
        Ok(Value::Expr(match known {
            // `format_args!`'s parts, kept until `Arguments::new` shows them.
            Std::FmtDisplay | Std::FmtDebug => {
                let ty = first_ty().expect("a type argument");
                let value = self.value_expr(values.next().expect("an argument"), span)?;
                return Ok(Value::Fmt(known, ty, value));
            }
            Std::FmtNew => {
                let template = values.next().expect("a template");
                let items = values.next().expect("the arguments");
                let Value::Expr(template) = template else {
                    return Err(self.unsupported(span, "this format string, from its MIR"));
                };
                let bytes =
                    expr_bytes(&template).ok_or_else(|| self.unsupported(span, "this format string, from its MIR"))?;
                let Value::List(items) = items else {
                    return Err(self.unsupported(span, "these format arguments, from its MIR"));
                };
                let mut parts = Vec::new();
                for piece in super::format_args::decode_template(&bytes)
                    .ok_or_else(|| self.unsupported(span, "this format string"))?
                {
                    parts.push(match piece {
                        super::format_args::Piece::Text(text) => Expr::str(text),
                        super::format_args::Piece::Argument(i, spec) => {
                            let Some(Value::Fmt(kind, ty, value)) = items.get(i).cloned() else {
                                return Err(self.unsupported(span, "this format argument, from its MIR"));
                            };
                            self.format_value(value, (kind, ty), spec, (None, None), span)?
                        }
                    });
                }
                super::display::join(parts)
            }
            // `assert_failed(kind, &left, &right, None or Some(message))`: the
            // two values' `{:?}`, by their types (ADR 0060).
            Std::AssertFailed => {
                let mut list = Vec::new();
                for (i, value) in values.enumerate() {
                    let value = self.value_expr(value, span)?;
                    list.push(match i {
                        1 | 2 => self.debug_string(value, arg_tys[i].peel_refs(), span)?,
                        _ => value,
                    });
                }
                if matches!(list.last().map(|e| &e.kind), Some(js::ExprKind::Undefined)) {
                    list.pop();
                }
                self.runtime.insert(Helper::AssertFailed);
                Expr::call(Expr::var("$assertFailed"), list)
            }
            // `vec![..]`: the array its box was written with.
            Std::VecMacro => self.value_expr(values.next().expect("the array"), span)?,
            Std::Range(op) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.range_call(op, exprs, arg_tys, span, out)?
            }
            Std::Number(op) => {
                // `i32::from_str_radix(s, 16)`'s is what its `Result` holds.
                let ty = match (op, output.kind()) {
                    (NumOp::FromStrRadix, ty::Adt(_, result)) => result.type_at(0),
                    (NumOp::FromBytes { .. } | NumOp::FloatFromBytes { .. } | NumOp::FromBits, _) => output,
                    _ => arg_tys.first().copied().map(|t| t.peel_refs()).expect("a number"),
                };
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.number_call(op, exprs, ty, span, out)?
            }
            // Panics end what runs: what's made before is made first.
            Std::Panic | Std::PanicFmt | Std::PanicDisplay | Std::BeginPanic => {
                self.flush(state, out)?;
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?
            }
            _ if lowered_from_values(known) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let made = self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?;
                // An iterator is a JS iterator (ADR 0364), where a source of
                // THIR's makes an array: its items, lazily.
                if self.range_kind(output).is_none()
                    && !self.is_user_iterator(output)
                    && self.implements_iterator(output)
                {
                    self.js_iterator(made)
                } else {
                    made
                }
            }
            _ => {
                let name = self.tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("`{name}`, from its MIR")));
            }
        }))
    }
}

/// Whether a value moves freely among those made around it: it does
/// nothing, and reads variables alone, none borrowed, which nothing can
/// change meanwhile.
fn movable(state: &State<'_, '_>, value: &Value<'_>) -> bool {
    match value {
        Value::Fmt(_, _, e) if e.reads_same() => true,
        Value::Expr(e) | Value::Fmt(_, _, e) | Value::Discriminant(e, _) => {
            let mut stable = true;
            e.visit_vars(&mut |name| {
                stable &= !state.borrowed_names.contains(name);
            });
            e.reads_only_vars() && stable
        }
        Value::List(items) => items.iter().all(|item| movable(state, item)),
        // A place shared, or shown, is read where it's used, as Rust reads
        // it; nothing changes it meanwhile.
        Value::Place(_) => true,
        Value::Ref(_) => false,
    }
}

/// Whether `std_values` lowers `known` of its arguments' values alone:
/// not one THIR's `std_call` lowers from its places or its shape first.
fn lowered_from_values(known: Std) -> bool {
    !matches!(
        known,
        Std::Swap
            | Std::Replace
            | Std::OptionTake
            | Std::OptionReplace
            | Std::MemTake
            | Std::Last
            | Std::Cloned
            | Std::Fuse
            | Std::ArrayMethod(_)
            | Std::Enumerate
            | Std::Rev
            | Std::Skip
            | Std::Take
            | Std::Fold
            | Std::Sum
            | Std::CollectString
            | Std::CollectFallible
            | Std::Collect
            | Std::Position
            | Std::Extreme(_)
            | Std::Sort
            | Std::SortByKey
            | Std::PushStr
            | Std::AssignOperator(_)
            | Std::StringEdit(_)
            | Std::StringWithCapacity
            | Std::VecMacro
            | Std::FmtNew
            | Std::AssertFailed
            | Std::Map(_)
            | Std::Range(_)
            | Std::Stream(_)
            | Std::TypeName { .. }
            | Std::Comb(_)
            | Std::IterComb(_)
            | Std::Text(_)
            | Std::Number(_)
            | Std::FromElem
            | Std::Heap(_)
            | Std::IterLen
            | Std::ExactLen
            | Std::SizeHint(_)
            | Std::WrappingOp(..)
            | Std::GenericSizeHint
            | Std::IterByRef
            | Std::NonZeroNew
            | Std::UserWrite
            | Std::DequeRemove
            | Std::Step(_)
            | Std::ToJson(_)
            | Std::FromJson
            | Std::Cow(_)
            | Std::Rc(_)
            | Std::Slice(_)
            | Std::OptionPlace(_)
            | Std::PtrEq
    )
}

/// What overflowing `op` says, as Rust's check does.
fn overflow_message(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "attempt to add with overflow",
        BinOp::Sub => "attempt to subtract with overflow",
        BinOp::Mul => "attempt to multiply with overflow",
        BinOp::Div => "attempt to divide with overflow",
        BinOp::Rem => "attempt to calculate the remainder with overflow",
        BinOp::Shl => "attempt to shift left with overflow",
        _ => "attempt to shift right with overflow",
    }
}

/// Labels, by block.
fn label(block: BasicBlock) -> String {
    format!("bb{}", block.as_usize())
}

/// Whether `block` does nothing but say it's never reached.
fn is_unreachable(body: &mir::Body<'_>, block: BasicBlock) -> bool {
    let data = &body.basic_blocks[block];
    matches!(data.terminator().kind, TerminatorKind::Unreachable)
        && data.statements.iter().all(|s| {
            matches!(
                s.kind,
                StatementKind::FakeRead(_)
                    | StatementKind::StorageLive(_)
                    | StatementKind::StorageDead(_)
                    | StatementKind::Nop
            )
        })
}

/// The bytes of a constant that's an array of them: a template's.
fn expr_bytes(e: &Expr) -> Option<Vec<u8>> {
    match &e.kind {
        js::ExprKind::Array(items) => items
            .iter()
            .map(|item| match item.kind {
                js::ExprKind::Num(n) if (0.0..256.0).contains(&n) && n.fract() == 0.0 => Some(n as u8),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn assert_operands<'m, 'tcx>(msg: &'m AssertKind<Operand<'tcx>>) -> Vec<&'m Operand<'tcx>> {
    match msg {
        AssertKind::BoundsCheck { len, index } => vec![len, index],
        AssertKind::Overflow(_, l, r) => vec![l, r],
        AssertKind::OverflowNeg(o) | AssertKind::DivisionByZero(o) | AssertKind::RemainderByZero(o) => vec![o],
        _ => Vec::new(),
    }
}

fn count_operand(locals: &mut Locals, operand: &Operand<'_>) {
    if let Operand::Copy(place) | Operand::Move(place) = operand {
        count_place(locals, place, false);
    }
}

/// A place's uses: its base read, unless it's written whole, and each
/// index read.
fn count_place(locals: &mut Locals, place: &Place<'_>, written: bool) {
    if written && place.projection.is_empty() {
        locals.writes[place.local] += 1;
    } else {
        locals.reads[place.local] += 1;
        // A part of its own written: the local is a variable. Through a
        // reference, it's what it refers to that's written.
        if written && !place.is_indirect() {
            locals.borrowed[place.local] = true;
        }
    }
    for elem in place.projection {
        if let PlaceElem::Index(index) = elem {
            locals.reads[index] += 1;
        }
    }
}

fn count_rvalue(locals: &mut Locals, rvalue: &Rvalue<'_>) {
    match rvalue {
        Rvalue::Use(o, _) | Rvalue::Repeat(o, _) | Rvalue::Cast(_, o, _) | Rvalue::UnaryOp(_, o) => {
            count_operand(locals, o)
        }
        Rvalue::BinaryOp(_, operands) => {
            count_operand(locals, &operands.0);
            count_operand(locals, &operands.1);
        }
        Rvalue::Aggregate(_, operands) => operands.iter().for_each(|o| count_operand(locals, o)),
        Rvalue::Ref(_, kind, place) => {
            count_place(locals, place, false);
            // The whole borrowed mutably may be written through the borrow: a
            // `let`, which a closure that captured it writes.
            if matches!(kind, BorrowKind::Mut { .. }) && place.projection.is_empty() {
                locals.writes[place.local] += 1;
            }
            if !place.projection.is_empty() && !place.is_indirect() {
                locals.borrowed[place.local] = true;
            }
        }
        Rvalue::RawPtr(_, place) => {
            count_place(locals, place, false);
            // A part of its own borrowed keeps its local a variable; the
            // whole borrowed is the value itself, read once, and a reborrow,
            // `&*r`, borrows what `r` refers to.
            if !place.projection.is_empty() && !place.is_indirect() {
                locals.borrowed[place.local] = true;
            }
        }
        Rvalue::Discriminant(place) | Rvalue::CopyForDeref(place) => count_place(locals, place, false),
        _ => {}
    }
}

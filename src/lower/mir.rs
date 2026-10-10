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
mod drops;
mod iter;
mod tidy;

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_index::IndexVec;
use rustc_middle::mir::interpret::GlobalId;
use rustc_middle::mir::{
    self, AggregateKind, AssertKind, BasicBlock, BinOp, BorrowKind, Const, ConstOperand, Local, LocalKind, Operand,
    Place, PlaceElem, Promoted, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use rustc_middle::ty::{self, Ty};
use rustc_mir_dataflow::move_paths::MovePathIndex;
use rustc_span::Span;

use super::combinators::StepOp;
use super::recognition::{Std, StdItem};
use super::representation::{const_js, variant_field};
use super::std_types::map::MapOp;
use super::std_types::number::NumOp;
use super::std_types::rc::RcOp;
use super::std_types::slice::SliceOp;
use super::std_types::text::TextOp;
use super::{Body, FnCx, LoweredFn, R, bindings, fn_def};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::runtime::Helper;

/// A body's MIR, as borrowck reads it (ADR 0364), and its promoted constants.
pub struct Mir<'tcx> {
    pub(super) body: mir::Body<'tcx>,
    pub(super) promoted: IndexVec<Promoted, mir::Body<'tcx>>,
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
    /// What `?`'s `Try::branch` makes of an `Option` or a `Result`: the
    /// value tried, and its type, whose `None` or `Err` is its `Break`.
    Branch(Expr, Ty<'tcx>),
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
    /// Each one a `&mut` is the place of, a number's: a call made while
    /// it lives, a closure's say, may assign it.
    changed: IndexVec<Local, bool>,
    /// Each one declared already.
    declared: IndexVec<Local, bool>,
    /// Each one the program names, not a macro of std's.
    user: IndexVec<Local, bool>,
}

/// A body's lowering state.
struct State<'m, 'tcx> {
    body: &'m mir::Body<'tcx>,
    promoted: &'m IndexVec<Promoted, mir::Body<'tcx>>,
    graph: cfg::Graph,
    locals: Locals,
    /// Temporaries made and not yet used, in the order they were made.
    pending: Vec<(Local, Value<'tcx>)>,
    /// The blocks being lowered whose label a `break` or a `continue` names.
    labels: Vec<BasicBlock>,
    /// The names of the locals borrowed, which a call may change.
    borrowed_names: std::collections::HashSet<String>,
    /// The names of the body's own locals: another variable, a static's, may
    /// change by any call.
    own_names: std::collections::HashSet<String>,
    /// Of a closure: what it captured, each its environment's field.
    /// And whether each is the place a `&mut` it captured is to.
    captures: Vec<(Expr, bool)>,
    /// What each `Drop` drops (`drops.rs`), and the flag of each path whose
    /// drop a flag says.
    drops: Option<drops::Elaboration<'tcx>>,
    flags: IndexVec<MovePathIndex, Option<String>>,
    /// The cleanup the temporaries made and not yet used unwind to, if one
    /// that drops something: the statement that runs them is its `try`.
    unwind: Option<BasicBlock>,
    /// Each local that's a `&mut` to a number's place that's always the
    /// same, `COUNT.value`: the place, named where it's used.
    refs: std::collections::HashMap<Local, Expr>,
    /// The boxes a call is given for its `&mut`s to numbers, each copied
    /// back to its place once the call returns (ADR 0074); an object's, in
    /// place, which each name for it sees (ADR 0147).
    copy_backs: Vec<(Expr, String, bool)>,
    /// The parameters that are boxes their callers give (ADR 0074).
    boxes: std::collections::HashSet<String>,
    /// Each local holding what `?`'s `Try::branch` made (`Value::Branch`).
    branches: std::collections::HashMap<Local, (Expr, Ty<'tcx>)>,
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
        // The drops it used, which its callers give it (ADR 0300).
        self.note_drop_uses(def_id);
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
    fn mir_function(
        &mut self,
        mir: &Mir<'tcx>,
        captures: Option<Vec<(Expr, bool)>>,
    ) -> R<(Vec<js::Pattern>, Vec<Stmt>)> {
        let mir_body = &mir.body;
        let drops = self.mir_elaboration(mir_body);
        let mut locals = self.mir_locals(mir_body);
        // A drop reads what it drops more than once, so it's a variable.
        for local in drops.iter().flat_map(|drops| drops.read(mir_body)) {
            locals.reads[local] += 2;
        }
        let mut state = State {
            body: mir_body,
            promoted: &mir.promoted,
            graph: cfg::Graph::of(mir_body),
            locals,
            pending: Vec::new(),
            labels: Vec::new(),
            borrowed_names: Default::default(),
            own_names: Default::default(),
            captures: captures.clone().unwrap_or_default(),
            flags: IndexVec::new(),
            drops,
            unwind: None,
            refs: Default::default(),
            copy_backs: Vec::new(),
            boxes: Default::default(),
            branches: Default::default(),
        };
        state.borrowed_names = (state.locals.borrowed.iter_enumerated())
            .filter(|&(_, &borrowed)| borrowed)
            .map(|(local, _)| state.locals.names[local].clone())
            .collect();
        state.own_names = state.locals.names.iter().filter(|n| !n.is_empty()).cloned().collect();
        // A closure's first argument is its environment, read through its fields.
        let skip = usize::from(captures.is_some());
        let params: Vec<js::Pattern> = mir_body
            .args_iter()
            .skip(skip)
            .map(|local| {
                state.locals.declared[local] = true;
                let name = match state.locals.names[local].as_str() {
                    "" => self.fresh("_"),
                    name => name.to_string(),
                };
                // A `&mut` to a value JS can't change in place: the box its
                // caller gives, its place the box's `value` (ADR 0074).
                let boxed = match captures {
                    // A closure's, by what it points at.
                    Some(_) => matches!(mir_body.local_decls[local].ty.kind(),
                        ty::Ref(_, pointee, ty::Mutability::Mut) if self.is_cell_pointee(*pointee)),
                    None => self.param_is_box(mir_body.source.def_id(), local.as_usize() - 1),
                };
                if boxed {
                    state.boxes.insert(name.clone());
                }
                js::Pattern::Name(name)
            })
            .collect();
        let mut out = Vec::new();
        // Each flag that says whether a value is there to drop, and the
        // values: a drop may be where their `const` isn't seen.
        let mut flagged = vec![false; mir_body.local_decls.len()];
        if let Some(drops) = &state.drops {
            state.flags = IndexVec::from_elem(None, &drops.move_data.move_paths);
            for (path, &is) in drops.flagged.iter_enumerated() {
                if !is {
                    continue;
                }
                let place = drops.move_data.move_paths[path].place;
                flagged[place.local.as_usize()] = true;
                let mut name = match state.locals.names[place.local].as_str() {
                    "" => format!("_{}", place.local.as_usize()),
                    name => name.to_string(),
                };
                for elem in place.projection {
                    if let PlaceElem::Field(field, _) = elem {
                        name = format!("{name}${}", field.as_usize());
                    }
                }
                let flag = self.fresh(&format!("{name}$live"));
                out.push(StmtKind::Let(flag.clone(), Some(Expr::bool(drops.initial[path]))).at(js::Span::NONE));
                state.flags[path] = Some(flag);
            }
        }
        // Locals assigned more than once are `let`s, declared first.
        for local in mir_body.local_decls.indices() {
            if mir_body.local_kind(local) != LocalKind::Arg
                && !state.locals.names[local].is_empty()
                && (state.locals.writes[local] > 1 || flagged[local.as_usize()])
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
        tidy::tidy(&mut out, &state.locals.names[mir::RETURN_PLACE]);
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
        let mut captures = Vec::new();
        for (i, value) in captured.into_iter().enumerate() {
            captures.push(match value {
                Value::Ref(place) | Value::Place(place) => (place, true),
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
                    let changes = matches!(&e.kind, js::ExprKind::Var(name)
                        if state.locals.names.iter_enumerated().any(|(l, n)| n == name && state.locals.writes[l] > 1));
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
            changed: IndexVec::from_elem_n(false, n),
            declared: IndexVec::from_elem_n(false, n),
            user: IndexVec::from_elem_n(false, n),
        };
        let mut user: IndexVec<Local, Option<String>> = IndexVec::from_elem_n(None, n);
        let sm = self.tcx.sess.source_map();
        // `?`'s own `val`, what its `Continue` holds: a temporary.
        let continued = self.continued(body);
        for info in &body.var_debug_info {
            // A variable a std macro makes, `format_args!`'s `args`, isn't
            // one the program names.
            if let mir::VarDebugInfoContents::Place(place) = info.value
                && place.projection.is_empty()
                && user[place.local].is_none()
                && !info.source_info.span.in_external_macro(sm)
                && !continued.contains(&place.local)
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
                        // The whole through its box or `MaybeUninit` borrowed mutably,
                        // `Box::new_uninit()`'s written by `write`: as the local.
                        if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue
                            && !borrowed.projection.is_empty()
                            && self.whole(body, borrowed)
                        {
                            locals.writes[borrowed.local] += 1;
                        }
                        // A `&mut` of a value JS can't share is its place, which
                        // a write through it, a closure's say, assigns.
                        let changes = match rvalue {
                            Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) => {
                                self.is_cell_pointee(borrowed.ty(&body.local_decls, self.tcx).ty)
                            }
                            Rvalue::RawPtr(kind, _) => kind.to_mutbl_lossy().is_mut(),
                            _ => false,
                        };
                        if changes
                            && let Rvalue::Ref(_, _, borrowed) | Rvalue::RawPtr(_, borrowed) = rvalue
                            && !borrowed.is_indirect()
                        {
                            locals.changed[borrowed.local] = true;
                        }
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
                // What a drop reads, its elaboration counts (`mir_function`).
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

    /// The locals `?` binds what a `ControlFlow::Continue` holds to, `val`.
    fn continued(&self, body: &mir::Body<'tcx>) -> Vec<Local> {
        let mut continued = Vec::new();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let (place, Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..)) = &**assign
                    && let [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] = from.projection.as_slice()
                    && let ty::Adt(adt, _) = body.local_decls[from.local].ty.kind()
                    && self
                        .tcx
                        .is_lang_item(adt.variant(*variant).def_id, LangItem::ControlFlowContinue)
                {
                    continued.push(place.local);
                }
            }
        }
        continued
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
            let mut body = self.mir_statements(state, block)?;
            body.extend(self.mir_within(state, block, &merges)?);
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
        let mut out = self.mir_statements(state, block)?;
        out.extend(self.mir_within(state, block, &merges)?);
        Ok(out)
    }

    /// Where `block` goes, inside a labeled block for each of `merges`, each
    /// followed by its own code (Ramsey's `nodeWithin`). Its statements come
    /// before, as none of them branches: what they bind is seen where its
    /// branches meet.
    fn mir_within(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, merges: &[BasicBlock]) -> R<Vec<Stmt>> {
        let Some((&merge, inner)) = merges.split_first() else {
            return self.mir_terminator(state, block);
        };
        // What's made here and read after the block, where its branches meet,
        // is declared here, not in the block: all but those the branch reads
        // last, which it reads where it's written.
        let read = terminator_reads(state.body.basic_blocks[block].terminator());
        if let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {
            let rest = state.pending.split_off(last + 1);
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            state.pending = rest;
            out.extend(self.mir_within(state, block, merges)?);
            return Ok(out);
        }
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
        // To the block that returns, and does nothing else: the `return`,
        // where a person writes it.
        if is_return(state.body, to) {
            return self.mir_terminator(state, to);
        }
        if state.graph.merges.contains(&to) {
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            out.push(StmtKind::Break(Some(label(to))).at(js::Span::NONE));
            return Ok(out);
        }
        self.mir_tree(state, to)
    }

    /// A block's statements.
    fn mir_statements(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let data = &state.body.basic_blocks[block];
        let mut out = Vec::new();
        for (index, statement) in data.statements.iter().enumerate() {
            let span = statement.source_info.span;
            // A statement that runs a call made before, which may panic
            // as something's there to drop, is that call's `try`.
            let unwind = state.unwind.take();
            let acting = self.acting(state);
            let mut made = Vec::new();
            let at = mir::Location {
                block,
                statement_index: index,
            };
            self.flag_sets(state, at, &mut made)?;
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let (place, rvalue) = &**assign;
                    self.mir_assign(state, place, rvalue, span, &mut made)?;
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
            let ran = acting.iter().any(|l| !state.pending.iter().any(|(p, _)| p == l));
            self.unwinding_to(state, unwind.filter(|_| ran), made, &mut out)?;
            state.unwind = unwind.filter(|_| !self.acting(state).is_empty());
        }
        Ok(out)
    }

    /// Where a block goes.
    fn mir_terminator(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        let terminator = state.body.basic_blocks[block].terminator();
        // What's made that may panic runs before a branch, in its own `try`.
        if state.unwind.is_some() && !matches!(terminator.kind, TerminatorKind::Call { .. }) {
            self.flush(state, &mut out)?;
        }
        self.flag_sets(state, state.body.terminator_loc(block), &mut out)?;
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
                let mut always = None;
                for (value, target) in targets.iter() {
                    if reached(target) {
                        let test = self.switch_test(&subject, ty, value, span)?;
                        // A test that's a constant: its branch alone, or none.
                        match test.kind {
                            js::ExprKind::Bool(false) => {}
                            js::ExprKind::Bool(true) => always = always.or(Some(target)),
                            _ => cases.push((test, target)),
                        }
                    }
                }
                if let Some(target) = always {
                    out.extend(self.mir_branch(state, block, target)?);
                    return Ok(out);
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
                    chain = Some(early_exit(test, then, chain, self.js_span(span)));
                }
                out.extend(chain.unwrap_or_default());
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
                unwind,
            } => {
                let cond = self.mir_expr(state, cond, &mut out)?;
                let message = self.assert_message(state, msg, span, &mut out)?;
                self.flush(state, &mut out)?;
                let failed = match expected {
                    true => Expr::unary(js::UnaryOp::Not, cond),
                    false => cond,
                };
                let js_span = self.js_span(span);
                // The panic drops what's there as it unwinds, then is thrown.
                let mut panic = self.mir_unwind(state, *unwind, span)?;
                panic.push(StmtKind::Throw(Expr::new_(Expr::var("Error"), vec![message])).at(js_span));
                out.push(StmtKind::If(failed, panic, None).at(js_span));
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Drop {
                place, target, unwind, ..
            } => {
                // What's dropped runs after what's made before it.
                let mut dropped = Vec::new();
                self.mir_drop(state, block, *place, false, span, &mut dropped)?;
                if !dropped.is_empty() {
                    self.flush(state, &mut out)?;
                    let cleanup = self.mir_unwind(state, *unwind, span)?;
                    match cleanup.is_empty() {
                        true => out.extend(dropped),
                        false => self.unwinding(dropped, cleanup, self.js_span(span), &mut out),
                    }
                }
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                fn_span,
                unwind,
                ..
            } => {
                // A call that may panic as something's there to drop runs in
                // a `try` whose `catch` drops it: the statement that runs it,
                // as it may be written where it's used. What's made before
                // that unwinds elsewhere runs first.
                let unwind = self.cleans_up(state, *unwind);
                if state.unwind != unwind && !self.acting(state).is_empty() {
                    self.flush(state, &mut out)?;
                }
                state.unwind = None;
                let mut called = Vec::new();
                let value = self.mir_call(state, func, args, *destination, *fn_span, &mut called)?;
                match target {
                    Some(target) => {
                        self.mir_store(state, *destination, value, span, &mut called)?;
                        // Each box given, copied back once the call returns.
                        if !state.copy_backs.is_empty() {
                            self.flush(state, &mut called)?;
                            for (place, boxed, object) in std::mem::take(&mut state.copy_backs) {
                                let back = Expr::member(Expr::var(&boxed), "value");
                                let js_span = self.js_span(span);
                                called.push(match object {
                                    true => {
                                        self.runtime.insert(Helper::Assign);
                                        StmtKind::Expr(Expr::call(Expr::var("$assign"), vec![place, back])).at(js_span)
                                    }
                                    false => StmtKind::Assign(place, back).at(js_span),
                                });
                            }
                        }
                        self.unwinding_to(state, unwind, called, &mut out)?;
                        state.unwind = unwind.filter(|_| !self.acting(state).is_empty());
                        out.extend(self.mir_branch(state, block, *target)?);
                    }
                    // A call that never returns, a panic's: what it does is all.
                    None => {
                        let value = self.value_expr(value, span)?;
                        self.flush(state, &mut called)?;
                        if !matches!(value.kind, js::ExprKind::Undefined) {
                            called.push(StmtKind::Expr(value).at(self.js_span(span)));
                        }
                        self.unwinding_to(state, unwind, called, &mut out)?;
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
        let unwind = state.unwind.take();
        let mut made = Vec::new();
        for (local, value) in std::mem::take(&mut state.pending) {
            let expr = self.value_expr(value, Span::default())?;
            let name = state.locals.names[local].clone();
            state.locals.declared[local] = true;
            made.push(StmtKind::Const(name, expr).at(js::Span::NONE));
        }
        self.unwinding_to(state, unwind, made, out)
    }

    /// `made`, run so that a panic in it drops what `unwind`, its cleanup,
    /// drops (`drops.rs`).
    fn unwinding_to(
        &mut self,
        state: &mut State<'_, 'tcx>,
        unwind: Option<BasicBlock>,
        made: Vec<Stmt>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match unwind {
            Some(cleanup) if !made.is_empty() => {
                let cleanup = self.mir_unwind(state, mir::UnwindAction::Cleanup(cleanup), Span::default())?;
                self.unwinding(made, cleanup, js::Span::NONE, out);
            }
            _ => out.extend(made),
        }
        Ok(())
    }

    /// The temporaries made and not yet used that do something.
    fn acting(&self, state: &State<'_, 'tcx>) -> Vec<Local> {
        let acting = state.pending.iter().filter(|(_, value)| !movable(state, value));
        acting.map(|(local, _)| *local).collect()
    }

    /// A local's value: its temporary's, taken where it's used, or its
    /// variable. A temporary used before others made after it, which would
    /// change their order, has all of them made first.
    fn read_local_value(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<Value<'tcx>> {
        if state.pending.iter().any(|(l, _)| *l == local) && self.admit(state, &[local], true, out)? {
            let at = state.pending.iter().position(|(l, _)| *l == local).expect("admitted");
            return Ok(state.pending.remove(at).1);
        }
        if let Some(place) = state.refs.get(&local) {
            return Ok(Value::Ref(place.clone()));
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
                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]
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
            Value::Branch(..) => return Err(self.unsupported(span, "what `?` made, read as a value, from its MIR")),
            // Kept or given: a handle on its place, which reads and writes it
            // (ADR 0099), where the place is always the same one.
            // A handle on the `value` of what a variable holds, a box, a handle
            // or a cell, is that: what it's read and written through.
            Value::Ref(ref place)
                if let js::ExprKind::Member(holder, field) = &place.kind
                    && field == "value"
                    && matches!(holder.kind, js::ExprKind::Var(_)) =>
            {
                (**holder).clone()
            }
            Value::Ref(place) if fixed_place(&place) => Expr::handle(place),
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
        let unread = !state.locals.user[local] && state.locals.reads[local] == 0 && !state.locals.borrowed[local];
        if place.projection.is_empty() && (state.locals.names[local].is_empty() || unread) {
            // Of `()`, or a temporary nothing reads: what made it runs.
            let expr = self.value_expr(value, span)?;
            if expr.has_effects() {
                self.flush(state, out)?;
                out.push(StmtKind::Expr(expr).at(self.js_span(span)));
            }
            return Ok(());
        }
        // What `?` made: the value it tried, read where the branch is.
        if place.projection.is_empty()
            && !state.body.local_decls[local].ty.is_integral()
            && let Value::Branch(tried, ty) = &value
        {
            state.branches.insert(local, (tried.clone(), *ty));
            return Ok(());
        }
        // A `&mut` to a place that's always the same, made once: the
        // place, named where it's used, as borrowck keeps it the same.
        if place.projection.is_empty()
            && state.locals.writes[local] == 1
            && let Value::Ref(target) = &value
            && same_place(state, target)
        {
            state.refs.insert(local, target.clone());
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
        // `*r = v` of a `&mut` to an object: the object becomes `v`, so each
        // name for it sees it, as THIR's `assign` writes it.
        let decls = &state.body.local_decls;
        if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
            && matches!(
                Place::ty_from(base.local, base.projection, decls, self.tcx).ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            )
            && let pointee = place.ty(decls, self.tcx).ty
            && !self.is_cell_pointee(pointee)
            && self.is_object(pointee)
        {
            self.runtime.insert(Helper::Assign);
            let assigned = Expr::call(Expr::var("$assign"), vec![target, value]);
            out.push(StmtKind::Expr(assigned).at(js_span));
            return Ok(());
        }
        out.push(StmtKind::Assign(target, value).at(js_span));
        Ok(())
    }

    /// The arguments of a call of the crate's: a `&mut` to a value JS can't
    /// change in place is a box, named for its parameter, copied back once
    /// the call returns (ADR 0074). What's given before it that does
    /// something runs first, before the box reads the place.
    fn boxed_args(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Expr>> {
        // What's called, as THIR's `boxed_callee` says: a trait's method
        // resolved to its impl's, whose parameters say which are boxes, or
        // the crate's trait's own, a dictionary's; not a `dyn`'s, whose pair
        // is its `&mut`, nor std's.
        let fn_id = match self.tcx.trait_of_assoc(def_id) {
            None => Some(def_id),
            Some(trait_id) => match self.impl_method(def_id, generic_args)? {
                Some((method, _)) => Some(method),
                None if self.is_rust_trait(trait_id) && !matches!(generic_args.type_at(0).kind(), ty::Dynamic(..)) => {
                    Some(def_id)
                }
                None => None,
            },
        };
        let mut exprs: Vec<Expr> = Vec::new();
        for (i, value) in values.into_iter().enumerate() {
            let object = !matches!(value, Value::Ref(_));
            let place = match value {
                Value::Ref(place) => place,
                // An object given where a box goes, a generic `&mut T`'s: boxed,
                // and copied back where it's a place.
                Value::Expr(object) if fn_id.is_some_and(|fn_id| self.param_is_box(fn_id, i)) => object,
                value => {
                    exprs.push(self.value_expr(value, span)?);
                    continue;
                }
            };
            // A box this function was given, given on as it is.
            if let js::ExprKind::Member(boxed, field) = &place.kind
                && field == "value"
                && let js::ExprKind::Var(name) = &boxed.kind
                && state.boxes.contains(name)
            {
                exprs.push((**boxed).clone());
                continue;
            }
            // A handle a call made, `first_mut().unwrap()`'s: given as it is,
            // its `value` the place (ADR 0099). A box of another place that
            // isn't fixed couldn't be copied back; a `()`'s needn't be.
            if !fixed_place(&place) && !matches!(place.kind, js::ExprKind::Undefined) {
                let js::ExprKind::Member(handle, field) = &place.kind else {
                    return Err(self.unsupported(span, "a `&mut` of a number given, from its MIR"));
                };
                if field != "value" {
                    return Err(self.unsupported(span, "a `&mut` of a number given, from its MIR"));
                }
                exprs.push((**handle).clone());
                continue;
            }
            // What it returns can hold the borrow: a box would be copied back
            // before it's used.
            if self.result_borrows(def_id, i) {
                if !fixed_place(&place) {
                    return Err(self.unsupported(span, "a `&mut` of a number given back, from its MIR"));
                }
                exprs.push(Expr::handle(place));
                continue;
            }
            // A `dyn`'s, or a slice's, the callee can't replace whole: its box
            // is never written, so it isn't copied back.
            let param = self
                .tcx
                .fn_sig(def_id)
                .instantiate(self.tcx, generic_args)
                .skip_binder()
                .inputs()[i];
            if param
                .builtin_deref(true)
                .is_some_and(|pointee| !pointee.is_sized(self.tcx, self.typing_env))
            {
                exprs.push(Expr::object(vec![Prop::Field("value".into(), place)]));
                continue;
            }
            self.flush(state, out)?;
            for expr in &mut exprs {
                if expr.has_effects() {
                    let made = std::mem::replace(expr, Expr::undefined());
                    *expr = self.spill("arg", made, out);
                }
            }
            let names = self.tcx.fn_arg_idents(def_id);
            let base = names
                .get(i)
                .copied()
                .flatten()
                .map_or("value".to_string(), |i| i.name.to_string());
            let name = self.fresh(&super::camel_case(&base));
            let boxed = Expr::object(vec![Prop::Field("value".into(), place.clone())]);
            out.push(StmtKind::Const(name.clone(), boxed).at(self.js_span(span)));
            if fixed_place(&place) {
                state.copy_backs.push((place, name.clone(), object));
            }
            exprs.push(Expr::var(&name));
        }
        Ok(exprs)
    }

    /// A place, for a read or a write.
    fn mir_place(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        if let Some((tried, ty)) = state.branches.get(&place.local).cloned() {
            return match place.projection.as_slice() {
                [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] if variant.as_u32() == 0 => {
                    Ok(match self.never_fails(ty) {
                        true => Expr::undefined(),
                        false => self.tried_value(tried, ty),
                    })
                }
                // The residual, `None` or the `Err` itself.
                [PlaceElem::Downcast(..), PlaceElem::Field(..)] => Ok(match self.option_of(ty) {
                    Some(_) => Expr::undefined(),
                    None => tried,
                }),
                _ => Err(self.unsupported(span, "this part of what `?` made, from its MIR")),
            };
        }
        let mut placed = false;
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
                Value::Ref(place) => {
                    placed = true;
                    place
                }
                value => self.value_expr(value, span)?,
            }
        };
        let mut ty = mir::PlaceTy::from_ty(state.body.local_decls[place.local].ty);
        for elem in place.projection {
            // Through a handle, or a box, of a `&mut` to a number: its
            // `value`, but where the place is already named.
            if elem == PlaceElem::Deref
                && let ty::Ref(_, pointee, ty::Mutability::Mut) = ty.ty.kind()
                && self.is_cell_pointee(*pointee)
            {
                if !std::mem::take(&mut placed) {
                    value = Expr::member(value, "value");
                }
                ty = ty.projection_ty(self.tcx, elem);
                continue;
            }
            placed = false;
            value = match (ty.ty.peel_refs().kind(), elem) {
                // Of a closure's environment, `(*_1).0`: what it captured.
                (ty::Closure(..), PlaceElem::Field(field, _)) if !state.captures.is_empty() => {
                    let (capture, is_place) = state.captures[field.as_usize()].clone();
                    placed = is_place;
                    capture
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
            // A reference is what it refers to (ADR 0023), and so are a box
            // and a pointer, a `static mut`'s.
            PlaceElem::Deref if ty.ty.is_ref() || ty.ty.is_box() || ty.ty.is_raw_ptr() => base,
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
            (ty::Adt(adt, _), _) if adt.is_union() => Err(self.unsupported(span, "unions")),
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
                // Not through a `&mut` to a cell, a box's or a handle's `value`.
                PlaceElem::Deref => {
                    ty.ty.is_box()
                        || (read
                            && ty.ty.is_ref()
                            && !matches!(ty.ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if self.is_cell_pointee(*inner)))
                }
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
        // A `Copy` value a variable holds, of a type changed in place: a copy
        // of it, as THIR's `copy_if_needed` makes. A temporary's is its own,
        // but not what it refers to, a static's.
        let copied = matches!(operand, Operand::Copy(place)
            if place.is_indirect() || !state.pending.iter().any(|(l, _)| *l == place.local));
        // A copy of a closure that changes what it captured would share it,
        // as a copy of a JS function does (ADR 0246).
        if let Operand::Copy(place) = operand
            && state.locals.reads[place.local] > 1
            && self.copies_own_captures(operand.ty(&state.body.local_decls, self.tcx))
        {
            let span = state.body.local_decls[place.local].source_info.span;
            return Err(self.unsupported(span, "copying a closure that changes what it captured"));
        }
        match self.mir_operand_read(state, operand, out)? {
            Value::Expr(e) if copied => {
                let ty = operand.ty(&state.body.local_decls, self.tcx);
                Ok(Value::Expr(self.copy_if_needed(e, ty)))
            }
            value => Ok(value),
        }
    }

    fn mir_operand_read(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        match operand {
            // A local's value, read through references to it, as it is.
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => {
                match self.read_local_value(state, place.local, out)? {
                    // Through a `&mut` to a place: what the place holds.
                    Value::Ref(target) if place.is_indirect() => Ok(Value::Expr(target)),
                    value => Ok(value),
                }
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let span = state.body.local_decls[place.local].source_info.span;
                Ok(Value::Expr(self.mir_place(state, *place, span, out)?))
            }
            // A function as a value, `.map(double)` or `f as fn()`: THIR's.
            Operand::Constant(c) if let Some((def_id, args)) = fn_def(c.ty()) => {
                match self.fn_item_value(def_id, args, c.ty(), c.span, out)? {
                    Some(f) => Ok(Value::Expr(f)),
                    None => Ok(Value::Expr(self.mir_called_value(state, (def_id, args), c.span)?)),
                }
            }
            // A pointer to a `static mut`, its `{ value }`'s place (ADR 0096).
            Operand::Constant(c)
                if let Some(def_id) = self.const_static(c)
                    && self.tcx.is_mutable_static(def_id)
                    && self.krate.fns.contains_key(&def_id) =>
            {
                Ok(Value::Ref(Expr::member(self.fn_ref(def_id), "value")))
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
        let tcx = self.tcx;
        // A promoted reference to a named constant or a static, `&LOG`:
        // what it refers to, as the reference is (ADR 0023).
        if let Const::Unevaluated(uv, _) = c.const_
            && let Some(promoted) = uv.promoted
            && let Some(referred) = referred_constant(&state.promoted[promoted])
            && self.const_static(c).is_none()
        {
            return self.mir_const(state, &referred);
        }
        // A named constant, as THIR's lowering writes one (ADR 0031); a
        // `const { .. }` block, its value, or as one (ADR 0127).
        if let Const::Unevaluated(uv, _) = c.const_
            && uv.promoted.is_none()
        {
            match tcx.def_kind(uv.def) {
                DefKind::Const { .. } | DefKind::AssocConst { .. } => {
                    return self.named_const(uv.def, uv.args, ty, c.span);
                }
                DefKind::AnonConst => {
                    return match self
                        .mir_const_value(c.const_, ty, c.span)
                        .and_then(|v| const_js(tcx, v))
                    {
                        Some(value) => Ok(value),
                        None => self.named_const(uv.def, uv.args, ty, c.span),
                    };
                }
                _ => {}
            }
        }
        // A const generic parameter, its value as the function's caller
        // gives it; a constant named in a type, as one named anywhere.
        if let Const::Ty(_, ct) = c.const_ {
            match ct.kind() {
                ty::ConstKind::Param(_) => return self.const_arg(ct, c.span),
                ty::ConstKind::Alias(_, alias) => {
                    let (ty::AliasConstKind::Projection { def_id }
                    | ty::AliasConstKind::Inherent { def_id }
                    | ty::AliasConstKind::Free { def_id }
                    | ty::AliasConstKind::Anon { def_id }) = alias.kind;
                    return self.named_const(def_id, alias.args, ty, c.span);
                }
                _ => {}
            }
        }
        // A reference to a static, which is the static (ADR 0023): a JS
        // global (ADR 0021), or the crate's, its module's `const` (ADR 0096).
        if let ty::Ref(_, _, ty::Mutability::Not) = ty.kind()
            && let Some(def_id) = self.const_static(c)
            && !tcx.is_mutable_static(def_id)
        {
            if tcx.is_foreign_item(def_id) {
                return Ok(self.js_ref(&bindings::js_name(tcx, def_id)));
            }
            if self.krate.fns.contains_key(&def_id) {
                return Ok(self.fn_ref(def_id));
            }
        }
        self.mir_const_value(c.const_, ty, c.span)
            .and_then(|value| const_js(tcx, value))
            .ok_or_else(|| self.unsupported(c.span, "this constant, from its MIR"))
    }

    /// The static a constant points to the start of: `&LOG`, a promoted
    /// `&STATIC`, `&raw mut COUNT`.
    fn const_static(&self, c: &ConstOperand<'tcx>) -> Option<rustc_span::def_id::DefId> {
        let value = match c.const_ {
            Const::Val(value, _) => value,
            Const::Unevaluated(uv, _) if uv.promoted.is_some() => {
                self.tcx.const_eval_resolve(self.typing_env, uv, c.span).ok()?
            }
            _ => return None,
        };
        let mir::ConstValue::Scalar(mir::interpret::Scalar::Ptr(ptr, _)) = value else {
            return None;
        };
        let (provenance, offset) = ptr.into_raw_parts();
        match self.tcx.global_alloc(provenance.alloc_id()) {
            mir::interpret::GlobalAlloc::Static(def_id) if offset.bytes() == 0 => Some(def_id),
            _ => None,
        }
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
                if matches!(kind, BorrowKind::Mut { .. }) && self.is_cell_pointee(pointee) {
                    return Ok(Value::Ref(self.mir_place(state, *place, span, out)?));
                }
                if self.through_values(state.body, place, true) {
                    let value = self.read_local_value(state, place.local, out)?;
                    return Ok(match (kind, value) {
                        // Of what can't change meanwhile, the body's own locals: read
                        // where it's used. A temporary's read of what can, a
                        // static's, is made where it's made.
                        (BorrowKind::Shared, Value::Expr(e))
                            if e.reads_same() && movable(state, &Value::Expr(e.clone())) =>
                        {
                            Value::Place(e)
                        }
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
                let (ty, r_ty) = (l.ty(decls, tcx), r.ty(decls, tcx));
                let [l, r]: [Value<'tcx>; 2] = self.take_operands(state, &[l, r], out)?.try_into().ok().expect("two");
                let l = self.value_expr(l, span)?;
                // A shift's amount of another type: a number, `Number(n & 63n)`.
                let r = super::std_types::number::shift_amount_of(*op, self.value_expr(r, span)?, ty, r_ty);
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
            // an array's a slice's (ADR 0023). One made a `dyn` is THIR's: a
            // `dyn Iterator` the JS iterator, another its `{ value, impl }`.
            Rvalue::Cast(mir::CastKind::PointerCoercion(coercion, _), operand, to) => {
                use rustc_middle::ty::adjustment::PointerCoercion;
                // Through a reference, a `Box` or an `Rc`, as THIR's `unsize_trait` sees it.
                let to_dyn = matches!(self.pointee(*to).kind(), ty::Dynamic(..));
                match coercion {
                    PointerCoercion::Unsize if to_dyn => {
                        let from = operand.ty(decls, tcx);
                        let value = self.mir_expr(state, operand, out)?;
                        if self.recognition().is_dyn_iter(*to) {
                            let iterator = from.builtin_deref(true).unwrap_or(from);
                            return Ok(Value::Expr(self.as_js_iterator(value, iterator, span)?));
                        }
                        return Ok(Value::Expr(self.unsize_trait(from, *to, value, span, out)?));
                    }
                    PointerCoercion::Unsize | PointerCoercion::MutToConstPointer | PointerCoercion::ArrayToPointer => {
                        if *coercion == PointerCoercion::Unsize {
                            self.check_unsize(*to, span)?;
                        }
                        return self.mir_operand(state, operand, out);
                    }
                    // A function, or a closure that captures nothing, as a `fn`:
                    // a JS function already (ADR 0125).
                    PointerCoercion::ReifyFnPointer(_)
                    | PointerCoercion::UnsafeFnPointer
                    | PointerCoercion::ClosureFnPointer(_) => {
                        return self.mir_operand(state, operand, out);
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
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some((tried, ty)) = state.branches.get(&place.local) =>
            {
                return Ok(Value::Branch(tried.clone(), *ty));
            }
            Rvalue::Discriminant(place) => {
                let ty = place.ty(decls, tcx).ty;
                let subject = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Discriminant(subject, ty));
            }
            Rvalue::Aggregate(kind, operands) => return self.mir_aggregate(state, kind, operands, span, out),
            // `[x; N]`, as THIR's lowering writes it (ADR 0107); an item
            // that's a constant is a named one's, or a literal, `Copy`.
            Rvalue::Repeat(operand, count) => {
                let item_ty = operand.ty(decls, tcx);
                let constant = matches!(operand, Operand::Constant(_));
                let item = self.mir_expr(state, operand, out)?;
                self.repeat((item, item_ty), *count, constant, span, out)?
            }
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
                let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(*did), args);
                let exprs = values
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?;
                Ok(Value::Expr(self.adt_value(ty, *variant_index, exprs, span)?))
            }
            AggregateKind::Closure(def_id, _) => Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?)),
            _ => Err(self.unsupported(span, "this aggregate, from its MIR")),
        }
    }

    /// The test that `subject`, of type `ty`, is `value`: of an enum's
    /// discriminant, its variant.
    fn switch_test(&mut self, subject: &Value<'tcx>, ty: Ty<'tcx>, value: u128, span: Span) -> R<Expr> {
        match subject {
            Value::Branch(tried, tried_ty) => {
                let failed = self.branch_failed(tried.clone(), *tried_ty);
                Ok(match value {
                    0 => negate(failed),
                    _ => failed,
                })
            }
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

    /// Whether what `?` tried returns early; a write's never does (ADRs
    /// 0054, 0132).
    fn branch_failed(&self, tried: Expr, ty: Ty<'tcx>) -> Expr {
        match self.never_fails(ty) {
            true => Expr::bool(false),
            false => self.tried_failed(tried, ty),
        }
    }

    /// A write's result, which never fails (ADRs 0054, 0132).
    fn never_fails(&self, ty: Ty<'tcx>) -> bool {
        self.is_fmt_result(ty) || self.recognition().is_io_unit_result(ty)
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
            // A function pointer, which may be JS's (ADR 0330): called with
            // its arguments given to JS.
            if !matches!(func.ty(decls, self.tcx).kind(), ty::FnPtr(..)) {
                return Err(self.unsupported(span, "calling a function value, from its MIR"));
            }
            let mut operands: Vec<&Operand<'tcx>> = vec![func];
            operands.extend(args.iter().map(|a| &a.node));
            let values = self.take_operands(state, &operands, out)?;
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let callee = exprs.remove(0);
            super::calls::given_to_js(&mut exprs);
            return Ok(Value::Expr(Expr::call(callee, exprs)));
        };
        let arg_tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
        let output = destination.ty(decls, self.tcx).ty;
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let values = self.take_operands(state, &operands, out)?;
        self.mir_call_values(state, (def_id, generic_args), (values, &arg_tys), output, span, out)
    }

    /// Any other function as a value, `.map(str::len)`: the arrow that calls
    /// it, its call lowered as one from MIR is, of its parameters.
    fn mir_called_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let tcx = self.tcx;
        let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
        let sig = tcx.instantiate_bound_regions_with_erased(sig);
        let inputs: Vec<Ty<'tcx>> = sig.inputs().to_vec();
        let params: Vec<String> = match inputs.len() {
            1 => vec![self.fresh("value")],
            n => (0..n)
                .map(|i| self.fresh(["a", "b", "c", "d", "e", "f"].get(i).copied().unwrap_or("arg")))
                .collect(),
        };
        let values = params.iter().map(|p| Value::Expr(Expr::var(p))).collect();
        // What's made for the call is the arrow's, not what's around it.
        let (pending, unwind) = (std::mem::take(&mut state.pending), state.unwind.take());
        let mut body = Vec::new();
        let called = self.mir_call_values(state, (def_id, args), (values, &inputs), sig.output(), span, &mut body);
        let value = called.and_then(|value| self.value_expr(value, span));
        let flushed = self.flush(state, &mut body);
        (state.pending, state.unwind) = (pending, unwind);
        let value = value?;
        flushed?;
        body.push(StmtKind::Return(Some(value)).at(self.js_span(span)));
        Ok(Expr::arrow(params.into_iter().map(Into::into).collect(), body))
    }

    /// A call of `def_id`, of its arguments' values, `arg_tys` their types.
    fn mir_call_values(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys): (Vec<Value<'tcx>>, &[Ty<'tcx>]),
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let arg_tys = arg_tys.to_vec();
        let tcx = self.tcx;
        // A constructor called, `f(Shape::Rect)`'s: the arrow making what it
        // makes (ADR 0125), applied.
        if matches!(tcx.def_kind(def_id), DefKind::Ctor(_, rustc_hir::def::CtorKind::Fn)) {
            let made = self.constructor_value(def_id, generic_args, span)?;
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            return Ok(Value::Expr(super::calls::apply(made, exprs)));
        }
        // An `Ok` `fmt::Result` is nothing in JS (ADR 0054): its `unwrap()` is
        // `()`, after what made it ran (ADR 0148), `is_ok()` `true`.
        if arg_tys.first().is_some_and(|ty| self.is_fmt_result(ty.peel_refs()))
            && tcx.def_kind(def_id) == DefKind::AssocFn
        {
            let Some(answer) = super::recognition::fmt_result_answer(tcx, def_id) else {
                return Err(self.unsupported(span, "methods of a `fmt::Result`, from its MIR"));
            };
            if self.krate.any_failing {
                return Err(self.unsupported(span, "methods of a `fmt::Result` that may fail, from its MIR"));
            }
            for value in values {
                let value = self.value_expr(value, span)?;
                if value.has_effects() {
                    self.flush(state, out)?;
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
            }
            return Ok(Value::Expr(match answer {
                super::recognition::FmtResultAnswer::Unwrap | super::recognition::FmtResultAnswer::Expect => {
                    Expr::undefined()
                }
                super::recognition::FmtResultAnswer::Is(ok) => Expr::bool(ok),
            }));
        }
        // `x?`: `Try::branch(x)`, kept as `x` (`Value::Branch`), and what it
        // returns, `FromResidual::from_residual(r)`, THIR's (ADR 0052).
        if tcx.is_lang_item(def_id, LangItem::TryTraitBranch) {
            let ty = arg_tys[0];
            if self.is_fmt_result(ty) && self.krate.any_failing {
                return Err(self.unsupported(span, "`?` of a `fmt::Result` that may fail, from its MIR"));
            }
            if !self.never_fails(ty) && self.option_of(ty).is_none() && !self.is_std_type(ty, StdItem::Result) {
                return Err(self.unsupported(span, &format!("`?` on a `{ty}`, from its MIR")));
            }
            let tried = self.value_expr(values.into_iter().next().expect("the value tried"), span)?;
            let tried = match tried.reads_same() {
                true => tried,
                false => {
                    self.flush(state, out)?;
                    self.spill(
                        if self.option_of(ty).is_some() {
                            "value"
                        } else {
                            "result"
                        },
                        tried,
                        out,
                    )
                }
            };
            return Ok(Value::Branch(tried, ty));
        }
        if tcx.is_lang_item(def_id, LangItem::TryTraitFromResidual) {
            let ty = arg_tys[0];
            if self.option_of(ty).is_none() && !self.is_std_type(ty, StdItem::Result) {
                return Err(self.unsupported(span, &format!("`?` returning a `{ty}`, from its MIR")));
            }
            let residual = self.value_expr(values.into_iter().next().expect("the residual"), span)?;
            let convert = self.tried_conversion(ty, Some(output), span)?;
            let converted = convert.map(|conversion| conversion.of(Expr::member(residual.clone(), "_0")));
            return Ok(Value::Expr(self.tried_returned(residual, ty, converted)));
        }
        // `x.into()` is the `From::from(x)` it calls, the crate's own (ADR 0052).
        let (def_id, generic_args) = self
            .resolve_into(def_id, generic_args)
            .unwrap_or((def_id, generic_args));
        // Calling a closure, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`.
        if let Some(fn_trait) = tcx.trait_of_assoc(def_id)
            && tcx.fn_trait_kind_from_def_id(fn_trait).is_some()
        {
            let mut values = values.into_iter();
            let callee = match values.next().expect("the closure") {
                Value::Ref(closure) => closure,
                value => self.value_expr(value, span)?,
            };
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
            let rhs = super::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);
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
        // A std iterator's method: a JS iterator's, lazy as Rust's. A `&mut`
        // to one is it, an object its helpers step in place.
        if tcx.trait_of_assoc(def_id).is_some() {
            let exprs = values
                .clone()
                .into_iter()
                .map(|v| match v {
                    Value::Ref(iterator) => Ok(iterator),
                    v => self.value_expr(v, span),
                })
                .collect::<R<Vec<_>>>()?;
            if let Some(call) = self.mir_iter_call(def_id, &arg_tys, exprs, output, span)? {
                return Ok(Value::Expr(call));
            }
        }
        // A trait's method: its impl's, a dictionary's, or what rust-js
        // writes itself, `==` of a struct say (ADRs 0049, 0052).
        let known = self.recognition().classify(def_id, generic_args);
        let rust_impl = tcx.trait_of_assoc(def_id).is_some()
            && self
                .resolve_instance(def_id, generic_args)?
                .is_some_and(|i| self.is_rust_fn(i.def_id()));
        // A `&mut` to a number is boxed for the crate's own trait or impl;
        // std's, `write!` to a `String` say, takes its place below.
        let gives_ref = values.iter().any(|v| matches!(v, Value::Ref(_)));
        if !matches!(known, Some(Std::Any(_)))
            && let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && (rust_impl
                || (super::recognition::operational(tcx, self.krate.foreign, trait_id)
                    && (!gives_ref || self.is_rust_trait(trait_id))))
        {
            let exprs = self.boxed_args(state, (def_id, generic_args), values.clone(), span, out)?;
            if let Some(call) = self.trait_call(def_id, generic_args, exprs, span, out)? {
                return Ok(Value::Expr(call));
            }
        }
        // The crate's own function, given its dictionaries (ADR 0049).
        if self.is_rust_fn(def_id) && tcx.trait_of_assoc(def_id).is_none() && !bindings::is_binding(tcx, def_id) {
            let mut exprs = self.boxed_args(state, (def_id, generic_args), values, span, out)?;
            // An iterator of the crate's, or a range, where a generic iterator
            // goes: a JS iterator, as THIR's `iterator_arg` makes (ADR 0061).
            let inputs = tcx.fn_sig(def_id).instantiate_identity().skip_binder().inputs();
            for ((expr, &input), &given) in exprs.iter_mut().zip(inputs).zip(&arg_tys) {
                let (input, given) = match *input.kind() {
                    ty::Ref(_, inner, ty::Mutability::Mut) => (inner, given.peel_refs()),
                    _ => (input, given),
                };
                if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(def_id, input, given) {
                    let value = std::mem::replace(expr, Expr::undefined());
                    *expr = self.as_js_iterator(value, given, span)?;
                }
            }
            exprs.extend(self.evidence_args(def_id, generic_args, span)?);
            let called = Expr::call(self.fn_ref(def_id), exprs);
            return Ok(Value::Expr(self.fmt_result_value(def_id, generic_args, called)));
        }
        if let Some(known) = known {
            let value = self.mir_std_call(
                state,
                (known, def_id),
                generic_args,
                &arg_tys,
                values,
                output,
                span,
                out,
            )?;
            // A std iterator THIR's lowering makes an array of, `bytes()`'s:
            // a JS iterator, which MIR steps. Not one that's the string it
            // shows, an escape's.
            return Ok(match value {
                Value::Expr(e)
                    if self.implements_iterator(output)
                        && !output.is_box()
                        && self.range_kind(output).is_none()
                        && !self.is_user_iterator(output)
                        && !super::recognition::is_text_escape(tcx, output) =>
                {
                    Value::Expr(self.std_iterator(e, output))
                }
                value => value,
            });
        }
        Err(self.unsupported(
            span,
            &format!("calling `{}`, from its MIR", self.tcx.def_path_str(def_id)),
        ))
    }

    /// `mem::swap`, `mem::replace`, `mem::take`, `Option::take` and
    /// `Option::replace`, as THIR's are: of a `&mut` to a place, writing each
    /// in turn, `const t = a; a = b; b = t;`, which nothing else can see
    /// while the call has it; of a `&mut` to an object, the object changed in
    /// place, `$exchange(a, b)`, `$take(a, v)` (ADR 0147).
    fn mir_swap_or_replace(
        &mut self,
        state: &mut State<'_, 'tcx>,
        known: Std,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let pointee = arg_tys[0].peel_refs();
        let js_span = self.js_span(span);
        let mut values = values.into_iter();
        let first = values.next().expect("a `&mut`");
        self.flush(state, out)?;
        let Value::Ref(a) = first else {
            // Of an object: changed in place.
            if !matches!(known, Std::Swap | Std::Replace | Std::MemTake) || !self.is_object(pointee) {
                return Err(self.unsupported(span, "this `&mut` given to `mem::swap` and the like, from its MIR"));
            }
            let a = self.value_expr(first, span)?;
            let b = match known {
                Std::MemTake => self.default_value(pointee, span)?,
                _ => self.value_expr(values.next().expect("a second"), span)?,
            };
            let (helper, name) = match known {
                Std::Swap => (Helper::Exchange, "$exchange"),
                _ => (Helper::Take, "$take"),
            };
            self.runtime.insert(helper);
            let call = Expr::call(Expr::var(name), vec![a, b]);
            if known == Std::Swap {
                out.push(StmtKind::Expr(call).at(js_span));
                return Ok(Value::Expr(Expr::undefined()));
            }
            return Ok(Value::Expr(call));
        };
        let (b, b_place) = match known {
            Std::Swap => match values.next() {
                Some(Value::Ref(b)) => (b.clone(), Some(b)),
                _ => return Err(self.unsupported(span, "`mem::swap` of these, from its MIR")),
            },
            // `None`, and what's in a `Some`, boxed where a generic one is (ADR 0051).
            Std::OptionTake => (Expr::undefined(), None),
            Std::OptionReplace => {
                let value = self.value_expr(values.next().expect("a value"), span)?;
                let item = self.option_of(pointee).expect("an `Option` has a `T`");
                let value = match self.boxed_payload(item) {
                    true => self.some(value),
                    false => value,
                };
                (value, None)
            }
            Std::MemTake => (self.default_value(pointee, span)?, None),
            _ => (self.value_expr(values.next().expect("a value"), span)?, None),
        };
        let old = self.spill(if b_place.is_some() { "t" } else { "old" }, a.clone(), out);
        out.push(StmtKind::Assign(a, b).at(js_span));
        Ok(Value::Expr(match b_place {
            Some(b) => {
                out.push(StmtKind::Assign(b, old).at(js_span));
                Expr::undefined()
            }
            None => old,
        }))
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
        let tcx = self.tcx;
        // `format!`, `print!` and `to_string()` of what may fail: in a `try`,
        // a `fmt::Error` std's panic, as THIR's `failing_consumer` (ADR 0187).
        let fails = match known {
            Std::Format | Std::Print { .. } => matches!(values.first(), Some(Value::Fmt(Std::FmtNew, ..))),
            Std::ToString => first_ty().is_some_and(|ty| self.fmt_may_fail(self.display_trait(), ty)),
            _ => false,
        };
        if fails {
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let value = match known {
                Std::ToString => self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?,
                _ => exprs.into_iter().next().expect("the text"),
            };
            if matches!(known, Std::Print { .. }) {
                self.flush(state, out)?;
            }
            return Ok(Value::Expr(self.failing_values(known, Vec::new(), value, span, out)));
        }
        // One whose `Some` is boxed where it looks like `None` (ADR 0051).
        if self.refuses_boxed_option(known, output) {
            return Err(self.unsupported(span, "this call, for an `Option` of what could look like `None`"));
        }
        // A value with a destructor taken, refused as THIR refuses it; a chain
        // that owns its items is `mir_iter_call`'s, so none here does.
        self.check_takes_drops(known, def_id, arg_tys, false, span)?;
        // `v[i]` through `index_mut`, of a value JS can't change in place: the
        // element's place, checked as `$index` checks it (ADR 0099).
        if known == Std::Index
            && tcx
                .trait_of_assoc(def_id)
                .is_some_and(|t| tcx.is_lang_item(t, LangItem::IndexMut))
            && let ty::Ref(_, item, _) = output.kind()
            && self.is_cell_pointee(*item)
        {
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let index = exprs.pop().expect("an index");
            let items = exprs.pop().expect("the items");
            self.flush(state, out)?;
            let items = if items.reads_same() {
                items
            } else {
                self.spill("items", items, out)
            };
            let index = if index.is_constant() {
                index
            } else {
                self.spill("index", index, out)
            };
            self.runtime.insert(Helper::Index);
            let check = Expr::call(Expr::var("$index"), vec![items.clone(), index.clone()]);
            out.push(StmtKind::Expr(check).at(self.js_span(span)));
            return Ok(Value::Ref(Expr::index(items, index)));
        }
        // A `&mut` it made to an item JS can't change in place: a handle on
        // each, as THIR's `item_handles` makes (ADR 0152).
        if let Some(cell) = self.makes_items_of(output, generic_args, arg_tys) {
            let exprs = values
                .iter()
                .cloned()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let receiver = arg_tys.first().map_or(tcx.types.unit, |t| t.peel_refs());
            if let Some(handles) = self.item_handles_of(known, exprs, receiver, span, out)? {
                return Ok(Value::Expr(match self.implements_iterator(output) {
                    true => self.js_iterator(handles),
                    false => handles,
                }));
            }
            if !known.gives_its_cell() {
                let path = tcx.def_path_str(def_id);
                return Err(self.unsupported(span, &format!("a `{cell}` from `{path}` used as a value, from its MIR")));
            }
        }
        let mut values = values.into_iter();
        Ok(Value::Expr(match known {
            // `format_args!`'s parts, kept until `Arguments::new` shows them.
            Std::FmtDisplay | Std::FmtDebug | Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {
                // `{:x}`'s and the like, which `format_value` writes by its spec.
                let ty = match known {
                    Std::FmtUsize => self.tcx.types.usize,
                    _ => first_ty().expect("a type argument"),
                };
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
                            let bad = || self.unsupported(span, "this format argument, from its MIR");
                            let item = |i: usize| match items.get(i) {
                                Some(Value::Fmt(kind, ty, value)) => Some((*kind, *ty, value.clone())),
                                _ => None,
                            };
                            let (kind, ty, value) = item(i).ok_or_else(bad)?;
                            // `{:1$}`'s width, or `{:.*}`'s precision, another argument.
                            let width = match spec.width_from {
                                Some(from) => Some(item(from).ok_or_else(bad)?.2),
                                None => spec.width.map(|w| Expr::int(w.into())),
                            };
                            let precision = match spec.precision_from {
                                Some(from) => Some(item(from).ok_or_else(bad)?.2),
                                None => spec.precision.map(|p| Expr::int(p.into())),
                            };
                            self.format_value(value, (kind, ty), spec, (width, precision), span)?
                        }
                    });
                }
                // Of what may fail, its `Arguments` say so to `format!`'s or
                // `print!`'s: `fmt_may_fail` of each, by its trait (ADR 0187).
                let fails = items.iter().any(|item| match *item {
                    Value::Fmt(Std::FmtDisplay, ty, _) => self.fmt_may_fail(self.display_trait(), ty),
                    Value::Fmt(Std::FmtDebug, ty, _) => self.fmt_may_fail(self.debug_trait(), ty),
                    // A number's or a pointer's, unless a type of the crate's.
                    Value::Fmt(_, ty, _) => self.krate.any_failing && ty.peel_refs().is_adt(),
                    _ => false,
                });
                let joined = super::display::join(parts);
                if fails {
                    return Ok(Value::Fmt(Std::FmtNew, output, joined));
                }
                joined
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
            // `s.push_str(t)`, `write!(s, ..)`: JS strings don't change, so `s`
            // gets a new one, as THIR's `push_str` writes it.
            Std::PushStr => {
                let place = match values.next() {
                    Some(Value::Ref(place)) => place,
                    Some(Value::Expr(place))
                        if matches!(place.kind, js::ExprKind::Var(_) | js::ExprKind::Member(..)) =>
                    {
                        place
                    }
                    _ => return Err(self.unsupported(span, "`push_str` on this, from its MIR")),
                };
                if self.krate.any_failing {
                    return Err(self.unsupported(span, "a `write!` here of what may fail, from its MIR"));
                }
                let value = self.value_expr(values.next().expect("what's pushed"), span)?;
                self.flush(state, out)?;
                super::display::append_written(&place, value, false, self.js_span(span), out);
                Expr::undefined()
            }
            // `&v[a..b]`, `s.get(a..)`, `v.drain(..b)`: of the items and the range's
            // value. Not `&mut v[a..b]`, a view of them, not a copy.
            Std::Text(op @ (TextOp::Slice | TextOp::StrSlice | TextOp::StrGet | TextOp::SliceGet | TextOp::Drain))
                if !self
                    .tcx
                    .trait_of_assoc(def_id)
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::IndexMut)) =>
            {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                self.slice_range_values(op, items, (range, arg_tys[1]), span, out)?
            }
            // An `Option`'s or a `Result`'s combinator: of its arguments' values,
            // the subject one just made where it's a call's.
            Std::Comb(comb) => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                let made = matches!(exprs[0].kind, js::ExprKind::Call(..));
                self.comb_values(comb, (exprs, arg_tys), made, generic_args, span, out)?
            }
            // A map's or a set's method, but an entry's value or a range, which
            // THIR takes apart from its places.
            Std::Map(op)
                if !matches!(
                    op,
                    MapOp::OrInsert
                        | MapOp::OrInsertWith
                        | MapOp::OrDefault
                        | MapOp::TreeRange { .. }
                        | MapOp::ExtractIf
                ) =>
            {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.map_values(op, (exprs, arg_tys), generic_args, false, span, out)?
            }
            // An iterator's combinator: THIR's, of a JS iterator, lazy as Rust's;
            // one a `&mut` lends, stepped through what can't close it.
            Std::IterComb(comb) => {
                let iter_ty = arg_tys[0].peel_refs();
                // Items with a destructor a combinator may leave undropped.
                if self
                    .iterator_item(iter_ty)
                    .is_some_and(|item| self.drops(item) != super::drops::Drops::Nothing)
                {
                    return Err(self.unsupported(
                        span,
                        "an iterator's combinator of items with a destructor, from its MIR",
                    ));
                }
                let lent = matches!(arg_tys[0].kind(), ty::Ref(_, _, ty::Mutability::Mut));
                let mut exprs = values
                    .map(|v| match v {
                        Value::Ref(place) => Ok(place),
                        v => self.value_expr(v, span),
                    })
                    .collect::<R<Vec<_>>>()?;
                let receiver = exprs.remove(0);
                let it = self.as_js_iterator(receiver, iter_ty, span)?;
                let it = match lent {
                    true => self.lent_iterator(it),
                    false => it,
                };
                self.iter_comb(comb, it, exprs.into_iter(), generic_args, iter_ty, true, span, out)?
            }
            // `v.sort()`, `v.sort_by_key(key)`: in place, THIR's.
            Std::Sort | Std::SortByKey => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let key = match known {
                    Std::SortByKey => Some(self.value_expr(values.next().expect("the key"), span)?),
                    _ => None,
                };
                self.sort_values(key, items, arg_tys[0], generic_args, span, out)?
            }
            Std::Swap | Std::Replace | Std::MemTake | Std::OptionTake | Std::OptionReplace => {
                let values: Vec<Value<'tcx>> = values.collect();
                return self.mir_swap_or_replace(state, known, values, arg_tys, span, out);
            }
            // `&mut v[a..b]`, `get_mut(a..b)`: a view of those items, THIR's (ADR 0335).
            Std::Slice(SliceOp::View { checked }) => {
                let items = self.value_expr(values.next().expect("the items"), span)?;
                let range = self.value_expr(values.next().expect("the range"), span)?;
                let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                // `&mut v[..]`: all of it, which is `v`.
                if !checked && start.as_int() == Some(0) && end.is_none() {
                    items
                } else {
                    let mut list = vec![items, start];
                    list.extend(end);
                    self.view_call(if checked { "$viewGet" } else { "$view" }, list)
                }
            }
            // An `Rc`'s or a `Weak`'s, but `make_mut`, which writes its place.
            Std::Rc(op) if op != RcOp::MakeMut => {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.rc_values(op, (exprs, arg_tys), span, out)?
            }
            // A string's or a slice's method, but one of a range or a part of
            // it, which THIR lowers from its place.
            Std::Text(op)
                if !matches!(
                    op,
                    TextOp::Slice
                        | TextOp::StrSlice
                        | TextOp::StrGet
                        | TextOp::SliceGet
                        | TextOp::Drain
                        | TextOp::Splice
                        | TextOp::ExtendFromWithin
                        | TextOp::StrPart { .. }
                        | TextOp::StrGetMut
                        | TextOp::StrSplitAtMut { .. }
                ) =>
            {
                let exprs = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.text_values(op, exprs, arg_tys, generic_args, span, out)?
            }
            // A `Peekable`'s or a slice iterator's own: of a `$iter`, which
            // knows where it is (ADR 0071), as THIR's `step_call` of one.
            Std::Step(op @ (StepOp::Peek | StepOp::NextIf | StepOp::NextIfEq | StepOp::AsSlice))
                if arg_tys
                    .first()
                    .is_some_and(|t| self.array_source(t.peel_refs()).is_some() || self.is_peekable(t.peel_refs())) =>
            {
                let mut exprs = values
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?
                    .into_iter();
                let it = exprs.next().expect("the iterator");
                let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
                match op {
                    StepOp::Peek if boxed => self.helper(Helper::PeekSome, "$peekSome", vec![it]),
                    StepOp::Peek => self.helper(Helper::Peek, "$peek", vec![it]),
                    StepOp::NextIf => {
                        let f = exprs.next().expect("a test");
                        self.helper(Helper::NextIf, "$nextIf", vec![it, f])
                    }
                    StepOp::NextIfEq => {
                        let item = arg_tys[1];
                        if !self.eq_is_identity(item) {
                            return Err(self.unsupported(span, &format!("`next_if_eq` of `{}`s", item.peel_refs())));
                        }
                        let x = exprs.next().expect("a value");
                        let x = if x.reads_same() {
                            x
                        } else {
                            self.spill("expected", x, out)
                        };
                        let same = Expr::arrow(
                            vec!["item".into()],
                            vec![StmtKind::Return(Some(Expr::bin(Op::Eq, Expr::var("item"), x))).at(js::Span::NONE)],
                        );
                        self.helper(Helper::NextIf, "$nextIf", vec![it, same])
                    }
                    _ => {
                        let it = if it.reads_same() { it } else { self.spill("it", it, out) };
                        Expr::call(
                            Expr::member(Expr::member(it.clone(), "items"), "slice"),
                            vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],
                        )
                    }
                }
            }
            // std's `size_hint()`, of an iterator of the crate's that keeps it:
            // `(0, None)`, as THIR's `size_hint` (ADR 0170).
            Std::SizeHint(false) => {
                let it = self.value_expr(values.next().expect("the iterator"), span)?;
                if it.has_effects() {
                    self.flush(state, out)?;
                    out.push(StmtKind::Expr(it).at(self.js_span(span)));
                }
                Expr::array(vec![Expr::int(0), Expr::undefined()])
            }
            // `x.to_int_unchecked()`: the cast `as` is, where it's in range.
            Std::Number(NumOp::ToIntUnchecked) => {
                let value = self.value_expr(values.next().expect("the float"), span)?;
                self.cast(value, arg_tys[0], output, span)?
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
                self.std_by_values(known, def_id, generic_args, arg_tys, exprs, output, span, out)?
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
        Value::Expr(e) | Value::Fmt(_, _, e) | Value::Discriminant(e, _) | Value::Branch(e, _) => {
            let mut stable = true;
            e.visit_vars(&mut |name| {
                stable &= !state.borrowed_names.contains(name) && state.own_names.contains(name);
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

/// The constant a promoted body refers to, `_1 = const LOG; _0 = &_1`, if
/// that's all it is.
fn referred_constant<'tcx>(body: &mir::Body<'tcx>) -> Option<ConstOperand<'tcx>> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let data = &body.basic_blocks[mir::START_BLOCK];
    let mut constant = None;
    let mut referred = None;
    for statement in &data.statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Use(Operand::Constant(c), ..)) if place.projection.is_empty() => {
                    constant = Some((place.local, **c));
                }
                (place, Rvalue::Ref(_, BorrowKind::Shared, target))
                    if place.local == mir::RETURN_PLACE && target.projection.is_empty() =>
                {
                    referred = Some(target.local);
                }
                _ => return None,
            },
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            _ => return None,
        }
    }
    let (local, c) = constant?;
    (Some(local) == referred).then_some(c)
}

/// Whether `place` names the same place wherever it's read: variables and
/// their fields, and elements at an index that's a constant, or a variable
/// assigned once.
fn same_place(state: &State<'_, '_>, place: &Expr) -> bool {
    match &place.kind {
        js::ExprKind::Var(_) => true,
        js::ExprKind::Member(object, _) => same_place(state, object),
        js::ExprKind::Index(object, index) => {
            same_place(state, object)
                && (index.is_constant()
                    || matches!(&index.kind, js::ExprKind::Var(name)
                        if state.locals.names.iter_enumerated().any(|(l, n)| n == name && state.locals.writes[l] == 1)))
        }
        _ => false,
    }
}

/// `!test`, of a test that's a `bool`: `!!t` is `t`.
fn negate(test: Expr) -> Expr {
    match test.kind {
        js::ExprKind::Unary(js::UnaryOp::Not, inner) => *inner,
        _ => Expr::unary(js::UnaryOp::Not, test),
    }
}

/// `if (test) { then } else { otherwise }`, written with the branch that
/// returns or throws first and the other after it, as a person writes an
/// early exit: the same, as that branch never goes on.
fn early_exit(test: Expr, then: Vec<Stmt>, otherwise: Option<Vec<Stmt>>, span: js::Span) -> Vec<Stmt> {
    let exits = |stmts: &[Stmt]| {
        matches!(
            stmts.last().map(|s| &s.kind),
            Some(StmtKind::Return(_) | StmtKind::Throw(_))
        )
    };
    match otherwise {
        Some(otherwise) if exits(&then) && !exits(&otherwise) => {
            let mut out = vec![StmtKind::If(test, then, None).at(span)];
            out.extend(otherwise);
            out
        }
        Some(otherwise) if exits(&otherwise) && (!exits(&then) || otherwise.len() < then.len()) => {
            let mut out = vec![StmtKind::If(negate(test), otherwise, None).at(span)];
            out.extend(then);
            out
        }
        otherwise => vec![StmtKind::If(test, then, otherwise).at(span)],
    }
}

/// Whether `block` only returns, through blocks that do nothing but go on.
fn is_return(body: &mir::Body<'_>, mut block: BasicBlock) -> bool {
    // A `loop {}` goes on forever.
    for _ in 0..body.basic_blocks.len() {
        let data = &body.basic_blocks[block];
        let quiet = data.statements.iter().all(|s| {
            matches!(
                s.kind,
                StatementKind::StorageDead(_) | StatementKind::StorageLive(_) | StatementKind::Nop
            )
        });
        match data.terminator().kind {
            TerminatorKind::Return => return quiet,
            TerminatorKind::Goto { target } if quiet => block = target,
            _ => return false,
        }
    }
    false
}

/// Whether `place` is one a handle can read and write wherever it's read:
/// variables, their fields, and elements at a constant index or one a
/// variable holds, which an index is read into where it's checked.
fn fixed_place(place: &Expr) -> bool {
    match &place.kind {
        js::ExprKind::Var(_) => true,
        js::ExprKind::Member(object, _) => fixed_place(object),
        js::ExprKind::Index(object, index) => {
            fixed_place(object) && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)))
        }
        _ => false,
    }
}

/// Whether a closure's `body` writes what it captured `i`th: assigns it,
/// or takes a `&mut` to it, through its environment `_1`.
fn writes_capture(body: &mir::Body<'_>, i: usize) -> bool {
    let env = Local::from_usize(1);
    let captured = |place: &Place<'_>| {
        place.local == env
            && place
                .projection
                .iter()
                .find(|elem| !matches!(elem, PlaceElem::Deref))
                .is_some_and(|elem| matches!(elem, PlaceElem::Field(field, _) if field.as_usize() == i))
    };
    body.basic_blocks.iter().any(|data| {
        data.statements.iter().any(|statement| match &statement.kind {
            StatementKind::Assign(assign) => {
                let (place, rvalue) = &**assign;
                captured(place)
                    || matches!(rvalue, Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) if captured(borrowed))
            }
            _ => false,
        }) || matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if captured(destination))
    })
}

/// The locals a terminator reads, as its operands.
fn terminator_reads(terminator: &mir::Terminator<'_>) -> Vec<Local> {
    let operand = |o: &Operand<'_>| o.place().map(|p| p.local);
    match &terminator.kind {
        TerminatorKind::SwitchInt { discr, .. } => operand(discr).into_iter().collect(),
        TerminatorKind::Call { func, args, .. } => std::iter::once(func)
            .chain(args.iter().map(|a| &a.node))
            .filter_map(operand)
            .collect(),
        TerminatorKind::Assert { cond, .. } => operand(cond).into_iter().collect(),
        TerminatorKind::Drop { place, .. } => vec![place.local],
        _ => Vec::new(),
    }
}

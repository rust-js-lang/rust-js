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

mod awaits;
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
use rustc_middle::thir;
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_mir_dataflow::move_paths::MovePathIndex;
use rustc_span::Span;

use super::combinators::StepOp;
use super::jsx::JsxArg;
use super::recognition::{Std, StdItem};
use super::representation::{const_js, variant_field};
use super::std_types::map::MapOp;
use super::std_types::number::NumOp;
use super::std_types::rc::RcOp;
use super::std_types::slice::SliceOp;
use super::std_types::text::{StringEdit, TextOp};
use super::{Body, FnCx, LoweredFn, R, Shape, bindings, fn_def};
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::names::js_ident;
use crate::runtime::Helper;

/// A body's MIR, as borrowck reads it (ADR 0364), and its promoted constants.
pub struct Mir<'tcx> {
    pub(super) body: mir::Body<'tcx>,
    pub(super) promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    /// Of a coroutine's, an `async` body's: the blocks whose call is an
    /// `.await` (`awaits.rs`).
    pub(super) awaits: std::collections::HashSet<BasicBlock>,
}

impl<'tcx> Mir<'tcx> {
    /// `body`, its `.await`s each one call.
    pub(super) fn new(
        tcx: ty::TyCtxt<'tcx>,
        mut body: mir::Body<'tcx>,
        promoted: IndexVec<Promoted, mir::Body<'tcx>>,
    ) -> Mir<'tcx> {
        let awaits = match body.coroutine.is_some() {
            true => awaits::awaits(tcx, &mut body),
            false => Default::default(),
        };
        Mir { body, promoted, awaits }
    }
}

/// Whether bodies are lowered from their MIR: `RUST_JS_MIR=1`, while the
/// proof of concept runs (ADR 0364).
pub fn mir_mode() -> bool {
    std::env::var_os(MIR).is_some_and(|v| v == "1")
}

/// The variable that says which lowering: `1`, MIR's.
pub const MIR: &str = "RUST_JS_MIR";

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

/// A parameter taken apart: its index, THIR's pattern, and the local MIR
/// binds each name the pattern binds to.
type ParamPattern = (usize, js::Pattern, Vec<(Local, String)>);

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
    /// The fields each is read by, as an operand, `copy _5.0`.
    fields: IndexVec<Local, Vec<usize>>,
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
    /// The blocks whose call is an `.await` (`awaits.rs`).
    awaits: &'m std::collections::HashSet<BasicBlock>,
    promoted: &'m IndexVec<Promoted, mir::Body<'tcx>>,
    graph: cfg::Graph,
    locals: Locals,
    /// Temporaries made and not yet used, in the order they were made.
    pending: Vec<(Local, Value<'tcx>)>,
    /// Each list read by its fields, once for each taken.
    taken: Vec<Local>,
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
    /// Each local holding a read whose `Option` only `.flatten()` reads,
    /// the JS read itself (ADR 0311).
    flattened: std::collections::HashSet<Local>,
    /// Each local holding what `?`'s `Try::branch` made (`Value::Branch`).
    branches: std::collections::HashMap<Local, (Expr, Ty<'tcx>)>,
    /// Each local holding an update's derived `Default` base, its fields'
    /// defaults, each read where its field is (`default_base_at`).
    defaults: std::collections::HashMap<Local, Vec<Expr>>,
    /// The locals a `let`'s pattern binds a props' rest to, and whether a
    /// variable is being given its value, a `match`'s binding, where a read
    /// of a `Rest` is one taken apart there.
    rests: std::collections::HashSet<Local>,
    binding_var: bool,
    /// Values made, each written where it's read: a constant, or what the
    /// module names, which reads the same anywhere, and an element, inside another or given its props,
    /// as JSX writes it, what it's made of read where it was made (ADR 0040).
    in_place: std::collections::HashMap<Local, Expr>,
    /// Each local read by its name already, a subject a branch tests: made
    /// as a variable, never written where it's read.
    named: std::collections::HashSet<Local>,
    /// Each local a parameter's pattern binds, and the parameter it's moved
    /// out of, which the pattern takes apart already.
    parts: std::collections::HashMap<Local, Local>,
    /// Whether what the call being lowered gives back is read by nothing,
    /// `map.insert(k, v);`'s, as THIR's `Call::discarded` says.
    discarding: bool,
    /// Where each `return` the program wrote is, of the body THIR has.
    returns: Vec<js::Span>,
    /// Where each range pattern is, `2..=5`, whose tests MIR writes bound
    /// first, `2 <= k`.
    ranges: Vec<Span>,
    /// Each shared reference to a place written where it's read, and each
    /// tuple of them: the place, read as one, which a closure captures as it
    /// is, or the tuple's list of them.
    places: std::collections::HashMap<Local, Value<'tcx>>,
    /// The indexes whose bounds check is where the element is read, `false`,
    /// `$index(xs, i)`, or written, `true`, `xs[$at(xs, i)]`, as THIR checks
    /// them (`deferred_check`).
    deferred: std::collections::HashMap<Local, bool>,
    /// The block whose statements are being lowered.
    at: BasicBlock,
    /// Each variable a pattern binds in an enum's variant, `Some(title)`:
    /// what it's bound to, where that reads the same anywhere, is it, as THIR
    /// reads it.
    payloads: std::collections::HashSet<Local>,
    /// The temporaries a `let`'s pattern takes apart, `let S { a, .. } =
    /// f();`'s: kept, for tidy to destructure, as THIR's pattern reads.
    patterned: std::collections::HashSet<Local>,
    /// The `Option`s made only `Some(..)` or `None` and read only by
    /// `#[rust_js::nullable]` fields: their `None` is `null` where it's
    /// made, `c ? v : null`, as THIR's `nullable` makes it (ADR 0275).
    null_nones: std::collections::HashSet<Local>,
    /// Each struct JSX gives a component as its props, and those of their
    /// flattened fields: JSX takes them apart (ADR 0213).
    jsx_props: std::collections::HashSet<Local>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn lower_fn_mir(&mut self, body: &Body<'tcx>, mir: &Mir<'tcx>) -> R<LoweredFn> {
        let def_id = body.def_id.to_def_id();
        let span = self.tcx.def_span(def_id);
        if self.krate.fns.get(&def_id).is_none() {
            return Err(self.unsupported(span, "this item from its MIR"));
        }
        // A component's props with a default taken apart, as THIR's are.
        for param in &self.thir.params {
            self.check_props_taken_apart(param.ty, param.pat.as_deref(), param.ty_span.unwrap_or(span))?;
        }
        // Its dictionaries, for its bounds, after its parameters (ADR 0049).
        let evidence = self.evidence_params(def_id);
        let (mut params, mut out) = self.mir_function(mir, None)?;
        params.extend(evidence);
        // An `async fn`'s body is its coroutine's, an async arrow called
        // at once: the async function's own (ADR 0029).
        // An `on_load!` body is its module's top, where JS has no `return`:
        // each a `break` out of a labeled block of it (ADR 0267).
        if bindings::is_on_load(self.tcx, def_id) {
            let label = self.fresh("load");
            let mut returns = false;
            js::statement_lists(&mut out, &mut |stmts| {
                for stmt in stmts.iter_mut() {
                    if let StmtKind::Return(None) = stmt.kind {
                        stmt.kind = StmtKind::Break(Some(label.clone()));
                        returns = true;
                    }
                }
            });
            if returns {
                out = vec![StmtKind::Labeled(label, out).at(js::Span::NONE)];
            }
        }
        let is_async = self.tcx.asyncness(def_id).is_async();
        if is_async {
            let Some(body) = called_async_body(&mut out) else {
                return Err(self.unsupported(span, "this `async fn`, from its MIR"));
            };
            out = body;
            // Its parameters as its body takes them apart, `{ params }`.
            tidy::destructured_params(&mut params, &mut out);
        }
        // The functions written in its body, each declared before what's
        // written after it, as THIR's (ADR 0308); a closure's in its own,
        // where `mir_closure` put them, unless it's lowered otherwise.
        let mut placed = std::collections::HashSet::new();
        js::each_expr_mut(&mut out, &mut |e| {
            if let js::ExprKind::FunctionHole(index) = e.kind {
                placed.insert(index);
            }
        });
        let written: Vec<rustc_span::def_id::DefId> = (self.krate.local_functions.keys())
            .filter(|&&id| {
                let mut parent = self.tcx.parent(id);
                while self.tcx.is_closure_like(parent) {
                    parent = self.tcx.parent(parent);
                }
                parent == def_id && !placed.contains(&id.index.as_u32())
            })
            .copied()
            .collect();
        self.place_written(written, &mut out);
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
                is_async,
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
        let outer_falsy = std::mem::take(&mut self.locals.falsy);
        let outer_tested = std::mem::take(&mut self.locals.tested);
        let outer_captured = std::mem::take(&mut self.locals.captured);
        let drops = self.mir_elaboration(mir_body);
        let patterns = match captures {
            None => self.mir_param_patterns(mir_body),
            Some(_) => Vec::new(),
        };
        let named: Vec<(Local, String)> = patterns.iter().flat_map(|(_, _, named)| named.clone()).collect();
        let mut locals = self.mir_locals(mir_body, &named);
        // A drop reads what it drops more than once, so it's a variable.
        let dropped: Vec<Local> = drops.iter().flat_map(|drops| drops.read(mir_body)).collect();
        for &local in &dropped {
            locals.reads[local] += 2;
        }
        let mut state = State {
            body: mir_body,
            awaits: &mir.awaits,
            promoted: &mir.promoted,
            graph: cfg::Graph::of(mir_body),
            locals,
            pending: Vec::new(),
            taken: Vec::new(),
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
            defaults: Default::default(),
            rests: (self.let_pattern_bindings(mir_body).into_iter())
                .filter_map(|(local, rest)| rest.then_some(local))
                .collect(),
            binding_var: false,
            flattened: Default::default(),
            in_place: Default::default(),
            named: Default::default(),
            parts: Default::default(),
            places: Default::default(),
            deferred: Default::default(),
            at: BasicBlock::from_usize(0),
            discarding: false,
            ranges: self.range_patterns(),
            returns: (self.thir.exprs.iter())
                .filter(|e| matches!(e.kind, thir::ExprKind::Return { .. }))
                .map(|e| self.js_span(e.span))
                .collect(),
            payloads: self.mir_payloads(mir_body),
            patterned: self.mir_patterned(mir_body),
            jsx_props: self.mir_jsx_props(mir_body),
            null_nones: Default::default(),
        };
        state.null_nones = self.mir_null_nones(mir_body, &state.locals);
        // A closure's first argument is its environment, read through its fields.
        // A coroutine's are that and what it's resumed with, which an async
        // function has neither of.
        let skip = match (captures.is_some(), mir_body.coroutine.is_some()) {
            (_, true) => mir_body.arg_count,
            (true, false) => 1,
            (false, false) => 0,
        };
        let mut params: Vec<js::Pattern> = mir_body
            .args_iter()
            .skip(skip)
            .map(|local| {
                state.locals.declared[local] = true;
                // One the program names nothing and reads nowhere, `_: Ref`,
                // is `_`, as THIR's.
                if !state.locals.user[local] && state.locals.reads[local] == 0 && !state.locals.borrowed[local] {
                    state.locals.names[local] = String::new();
                }
                let name = match state.locals.names[local].as_str() {
                    "" => self.fresh("_"),
                    name => name.to_string(),
                };
                state.locals.names[local].clone_from(&name);
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
        // Where each part a pattern binds is a local MIR moves out of the
        // parameter, once, and that's all it reads of it: the pattern.
        for (i, pattern, named) in patterns {
            let arg = Local::from_usize(i + 1);
            let moved_out = named.iter().all(|&(local, _)| {
                mir_body.basic_blocks.iter().flat_map(|data| &data.statements).any(|statement| {
                    matches!(&statement.kind, StatementKind::Assign(assign)
                        if assign.0 == Place::from(local)
                            && matches!(&assign.1, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _) if p.local == arg))
                })
            });
            // Its drops, of what's moved out, as its parts', read it too.
            let drops = 2 * dropped.iter().filter(|&&local| local == arg).count();
            if !moved_out || state.locals.reads[arg] - drops != named.len() {
                continue;
            }
            for (local, _) in named {
                state.locals.declared[local] = true;
                state.parts.insert(local, arg);
            }
            params[i] = pattern;
        }
        state.borrowed_names = (state.locals.borrowed.iter_enumerated())
            .filter(|&(_, &borrowed)| borrowed)
            .map(|(local, _)| state.locals.names[local].clone())
            .collect();
        state.own_names = state.locals.names.iter().filter(|n| !n.is_empty()).cloned().collect();
        // What reads the same wherever it's read, as THIR's variables say.
        for (local, name) in state.locals.names.iter_enumerated() {
            if name.is_empty()
                // A variable the program names, assigned once each way, as a
                // `match`'s arms each assign `let variant = match ..`: not
                // MIR's, a `vec!`'s box, say, filled through it after.
                || state.locals.writes[local] > 1 && !(state.locals.user[local] && once_each_way(mir_body, local))
                || state.locals.borrowed[local]
                || state.locals.changed[local]
            {
                continue;
            }
            self.locals.alike.insert(name.clone());
            let is_plain = |ty: Ty<'tcx>| {
                ty.walk().all(|arg| match arg.kind() {
                    ty::GenericArgKind::Type(t) => {
                        !matches!(t.kind(), ty::Ref(_, _, ty::Mutability::Mut) | ty::RawPtr(..))
                            // Inside a binder, a function pointer's parameter, unknown.
                            && !t.has_escaping_bound_vars()
                            && t.is_freeze(self.tcx, self.typing_env)
                            && !self.is_js_object(t)
                    }
                    _ => true,
                })
            };
            let plain = is_plain(mir_body.local_decls[local].ty);
            // What's to through a `&mut` changes by what it's given to.
            let through_mut = matches!(
                mir_body.local_decls[local].ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            );
            // Of a struct not all plain, a `Cell` in it, each field that is,
            // `q.title`, which nothing changes either.
            let fields: Vec<String> = match mir_body.local_decls[local].ty.peel_refs().kind() {
                ty::Adt(adt, _) if !plain && !through_mut && adt.is_struct() => {
                    match self.shape(mir_body.local_decls[local].ty.peel_refs()) {
                        Shape::Object(fields) => (fields.into_iter())
                            .filter(|(_, ty)| is_plain(*ty))
                            .map(|(field, _)| format!("{name}.{field}"))
                            .collect(),
                        _ => Vec::new(),
                    }
                }
                _ => Vec::new(),
            };
            if plain {
                self.locals.plain.insert(name.clone());
            }
            self.locals.plain.extend(fields);
            // Of an array, of what may be JS's, its items, which only a
            // `&mut` changes: `...items` (`read_past_calls`).
            let ty = mir_body.local_decls[local].ty.peel_refs();
            if !plain && !through_mut && (ty.is_slice() || ty.is_array() || self.is_vec_like(ty)) {
                self.locals.plain.insert(format!("...{name}"));
            }
        }
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
                && !state.locals.declared[local]
                && !state.locals.names[local].is_empty()
                && (state.locals.writes[local] > 1 || flagged[local.as_usize()])
            {
                out.push(StmtKind::Let(state.locals.names[local].clone(), None).at(js::Span::NONE));
                state.locals.declared[local] = true;
            }
        }
        let start = BasicBlock::from_usize(0);
        out.extend(self.mir_tree(&mut state, start)?);
        // Each bounds check moved to where its element is read, made there.
        if !state.deferred.is_empty() {
            return Err(self.unsupported(mir_body.span, "a bounds check this read doesn't make, from its MIR"));
        }
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
        // A closure's own, which it named, apart from its body's.
        let falsy = std::mem::replace(&mut self.locals.falsy, outer_falsy);
        let tested = std::mem::replace(&mut self.locals.tested, outer_tested);
        // Where each `match` is written whose arms are each a variant of its
        // own, with no field, guard or binding: one that may read a table by
        // what's matched (ADR 0233), as tidy's `table_reads` tells.
        let tables: Vec<js::Span> = (self.thir.exprs.iter())
            .filter_map(|e| {
                let thir::ExprKind::Match { scrutinee, ref arms, .. } = e.kind else {
                    return None;
                };
                let ty::Adt(adt, _) = self.thir[scrutinee].ty.peel_refs().kind() else {
                    return None;
                };
                let each = (arms.iter()).all(|&arm| {
                    let arm = &self.thir[arm];
                    arm.guard.is_none()
                        && matches!(&arm.pattern.kind, thir::PatKind::Variant { subpatterns, .. } if subpatterns.is_empty())
                });
                (adt.is_enum() && arms.len() >= 2 && each)
                    .then(|| self.js_span(e.span))
            })
            .collect();
        // Where each `if`'s condition is written, of the body THIR has:
        // tests in one are `&&`'s; and a `match` and the guard of an arm of
        // it, whose pattern's test and the guard are.
        let conditions: Vec<(js::Span, js::Span)> = (self.thir.exprs.iter())
            .flat_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } => {
                    let cond = self.js_span(self.thir[cond].span);
                    vec![(cond, cond)]
                }
                // From `match` to its scrutinee's end, where MIR tests it.
                thir::ExprKind::Match {
                    scrutinee, ref arms, ..
                } => (arms.iter())
                    .filter_map(|&arm| {
                        let guard = self.thir[arm].guard?;
                        let tested = js::Span {
                            lo: self.js_span(e.span).lo,
                            hi: self.js_span(self.thir[scrutinee].span).hi,
                        };
                        Some((tested, self.js_span(self.thir[guard].span)))
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        // Where each `&&` or `||` the program wrote is.
        let logicals: Vec<js::Span> = (self.thir.exprs.iter())
            .filter(|e| matches!(e.kind, thir::ExprKind::LogicalOp { .. }))
            .map(|e| self.js_span(e.span))
            .collect();
        // Where each `if` the program wrote of a `continue` tests: one alone,
        // or one that ends what it does.
        let continues: Vec<js::Span> = (self.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::If { cond, then, .. } => {
                    let mut then = then;
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[then].kind
                    {
                        then = inner;
                    }
                    let thir::ExprKind::Block { block } = self.thir[then].kind else {
                        return None;
                    };
                    let block = &self.thir[block];
                    let only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => Some(e),
                        (Some(s), None) => match self.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => Some(expr),
                            _ => None,
                        },
                        _ => None,
                    }?;
                    let mut only = only;
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[only].kind
                    {
                        only = inner;
                    }
                    matches!(self.thir[only].kind, thir::ExprKind::Continue { .. })
                        .then(|| self.js_span(self.thir[cond].span))
                }
                _ => None,
            })
            .chain(self.let_else_continues())
            .collect();
        // The closures a `let` names that a hook's function alone is, by
        // their names (ADR 0297).
        let named: Vec<rustc_span::Span> = (self.thir.stmts.iter())
            .filter_map(|stmt| match &stmt.kind {
                thir::StmtKind::Let { pattern, .. }
                    if let thir::PatKind::Binding { var, .. } = pattern.kind
                        && self.body_facts.named_callbacks.contains(&var) =>
                {
                    Some(pattern.span)
                }
                _ => None,
            })
            .collect();
        let callbacks: std::collections::HashSet<String> = (mir_body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty() && named.contains(&info.source_info.span) =>
                {
                    Some(state.locals.names[place.local].clone())
                }
                _ => None,
            })
            .collect();
        // Where each loop is written, and whether as a `while`.
        // Each loop's label the program wrote, `'rows`, by the loop.
        let labels: std::collections::HashMap<thir::ExprId, String> = (self.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Scope { value, hir_id, .. } => match self.tcx.hir_expect_expr(hir_id).kind {
                    rustc_hir::ExprKind::Loop(_, Some(label), ..) => {
                        Some((value, label.ident.name.as_str().trim_start_matches('\'').to_string()))
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let loops: Vec<(js::Span, bool, Option<String>)> = (self.thir.exprs.iter_enumerated())
            .filter_map(|(id, e)| match e.kind {
                thir::ExprKind::Loop { .. } => {
                    let written = e.span.source_callsite();
                    // Where it's written, not the body it's in, which a closure's
                    // span holds too.
                    let whiles = (self.thir.exprs.iter()).any(|w| {
                        w.span.is_desugaring(rustc_span::DesugaringKind::WhileLoop)
                            && w.span.source_callsite().source_equal(written)
                    });
                    Some((self.js_span(written), whiles, labels.get(&id).cloned()))
                }
                _ => None,
            })
            .collect();
        let captured = std::mem::replace(&mut self.locals.captured, outer_captured);
        // A name each of whose locals is a `bool`.
        let mut booleans = std::collections::HashSet::new();
        let mut others = std::collections::HashSet::new();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            let bool = match ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),
                _ => ty.is_bool(),
            };
            match bool {
                true => booleans.insert(name.clone()),
                false => others.insert(name.clone()),
            };
        }
        // And a closure's, of what it captures: its parent's.
        let env = mir_body
            .args_iter()
            .next()
            .map(|local| mir_body.local_decls[local].ty.peel_refs());
        if let Some(ty::Closure(_, args)) = env.map(|ty| ty.kind()) {
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(&state.captures) {
                if let js::ExprKind::Var(name) = &capture.kind {
                    match ty.peel_refs().is_bool() {
                        true => booleans.insert(name.clone()),
                        false => others.insert(name.clone()),
                    };
                }
            }
        }
        booleans.retain(|name| !others.contains(name));
        // And each field path that's a `bool` each time it's read.
        booleans.extend(
            (self.locals.paths.iter())
                .filter(|&(_, &b)| b)
                .map(|(path, _)| path.clone()),
        );
        // And the module's imports, whose fields JS's modules never change
        // either, `HighlightStyle.define`, as THIR's `reads_alike` has them.
        let mut plain = self.locals.plain.clone();
        plain.extend(
            self.krate
                .imports
                .get(&self.module)
                .into_iter()
                .flat_map(|imports| imports.values().cloned()),
        );
        tidy::tidy(
            &mut out,
            &tidy::Hints {
                result: &state.locals.names[mir::RETURN_PLACE],
                falsy: &falsy,
                tested: &tested,
                conditions: &conditions,
                loops: &loops,
                continues: &continues,
                logicals: &logicals,
                callbacks: &callbacks,
                tables: &tables,
                closure: matches!(env.map(|ty| ty.kind()), Some(ty::Closure(..))),
                patterns: &(self.let_pattern_bindings(mir_body).into_iter())
                    .map(|(l, _)| state.locals.names[l].clone())
                    .collect(),
                returns: &state.returns,
                captured: &captured,
                booleans: &booleans,
                plain: &plain,
                forwarding: &self.locals.forwarding.clone(),
                concise: &self.locals.concise.clone(),
            },
        );
        tidy::destructured_params(&mut params, &mut out);
        tidy::readable_temporaries(&params, &mut out);
        Ok((params, out))
    }

    /// The locals that patterns bind in an enum's variant, by THIR's
    /// patterns: `if let`s', `match`es' and `let`s'.
    fn mir_payloads(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let mut spans = Vec::new();
        let mut bound = |pat: &thir::Pat<'tcx>| {
            pat.walk(|p| match &p.kind {
                // Not a `for` loop's `Some(x)`: its item, a variable.
                thir::PatKind::Variant { subpatterns, .. }
                    if !p.span.is_desugaring(rustc_span::DesugaringKind::ForLoop) =>
                {
                    for field in subpatterns {
                        field.pattern.walk_always(|b| {
                            if let thir::PatKind::Binding { .. } = b.kind {
                                spans.push(b.span);
                            }
                        });
                    }
                    false
                }
                _ => true,
            })
        };
        for stmt in self.thir.stmts.iter() {
            if let thir::StmtKind::Let { pattern, .. } = &stmt.kind {
                bound(pattern);
            }
        }
        for expr in self.thir.exprs.iter() {
            if let thir::ExprKind::Let { pat, .. } = &expr.kind {
                bound(pat);
            }
        }
        for arm in self.thir.arms.iter() {
            bound(&arm.pattern);
        }
        // An arm's of the value whole, `x if x.len() > 1`, as a variant's part.
        spans.extend((self.thir.arms.iter()).filter_map(|arm| match arm.pattern.kind {
            thir::PatKind::Binding { .. } => Some(arm.pattern.span),
            _ => None,
        }));
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty() && spans.contains(&info.source_info.span) =>
                {
                    Some(place.local)
                }
                _ => None,
            })
            .collect()
    }

    /// The locals `State::null_nones` says: each write a `Some(..)` or a
    /// `None`, one at least, and each read a nullable field's.
    fn mir_null_nones(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
        let mut made: std::collections::HashMap<Local, (usize, bool)> = Default::default();
        let mut given: std::collections::HashMap<Local, usize> = Default::default();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let Rvalue::Aggregate(kind, operands) = &assign.1 else {
                continue;
            };
            let AggregateKind::Adt(did, variant, ..) = **kind else {
                continue;
            };
            if assign.0.projection.is_empty() && self.tcx.is_lang_item(did, LangItem::Option) {
                let (writes, none) = made.entry(assign.0.local).or_default();
                *writes += 1;
                *none |= variant.as_u32() == 0;
            }
            let fields = &self.tcx.adt_def(did).variant(variant).fields;
            for (field, operand) in operands.iter_enumerated() {
                if let Some(read) = operand.place()
                    && read.projection.is_empty()
                    && bindings::is_nullable(self.tcx, &fields[field])
                {
                    *given.entry(read.local).or_default() += 1;
                }
            }
        }
        (made.into_iter())
            .filter(|&(local, (writes, none))| {
                none && writes == locals.writes[local] && given.get(&local) == Some(&locals.reads[local])
            })
            .map(|(local, _)| local)
            .collect()
    }

    /// The locals a `let`'s struct pattern binds its fields to, through a
    /// reference too, `let S { a } = use_context(..)`'s; each whether it's
    /// the rest, a `Rest` or a flattened struct's (ADR 0195, 0204).
    fn let_pattern_bindings(&self, body: &mir::Body<'tcx>) -> Vec<(Local, bool)> {
        let mut spans = Vec::new();
        for stmt in self.thir.stmts.iter() {
            let thir::StmtKind::Let { pattern, .. } = &stmt.kind else {
                continue;
            };
            let mut pattern = &**pattern;
            while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {
                pattern = subpattern;
            }
            if let thir::PatKind::Leaf { subpatterns } = &pattern.kind {
                for field in subpatterns {
                    if let thir::PatKind::Binding { subpattern: None, .. } = field.pattern.kind {
                        let rest = bindings::is_rest_field(self.tcx, pattern.ty, field.field.as_usize());
                        spans.push((field.pattern.span, rest));
                    }
                }
            }
        }
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => (spans.iter())
                    .find(|(span, _)| *span == info.source_info.span)
                    .map(|&(_, rest)| (place.local, rest)),
                _ => None,
            })
            .collect()
    }

    /// The temporaries a `let`'s struct or tuple pattern takes apart: what
    /// its bindings, of its fields, are read from.
    fn mir_patterned(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let bound: Vec<Local> = self.let_pattern_bindings(body).into_iter().map(|(l, _)| l).collect();
        let mut patterned = std::collections::HashSet::new();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            // Bound by value or, through a reference, by one: `&(*_2).0`.
            if let StatementKind::Assign(assign) = &statement.kind
                && let (
                    to,
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                    | Rvalue::Ref(_, BorrowKind::Shared, from),
                ) = &**assign
                && to.as_local().is_some_and(|l| bound.contains(&l))
                && let [derefs @ .., PlaceElem::Field(..)] = from.projection.as_slice()
                && derefs.iter().all(|p| matches!(p, PlaceElem::Deref))
                && body.local_kind(from.local) == LocalKind::Temp
            {
                patterned.insert(from.local);
            }
        }
        patterned
    }

    /// Whether struct pattern `pat` binds each field that has a destructor,
    /// by value, whole: what it leaves has none to drop.
    fn binds_dropped_fields(&self, pat: &thir::Pat<'tcx>) -> bool {
        let (thir::PatKind::Leaf { subpatterns }, ty::Adt(adt, args)) = (&pat.kind, pat.ty.kind()) else {
            return false;
        };
        adt.is_struct()
            && (adt.non_enum_variant().fields.iter_enumerated()).all(|(i, field)| {
                !self.has_drops(field.ty(self.tcx, args).skip_normalization())
                    || subpatterns.iter().any(|sub| {
                        sub.field == i
                            && matches!(
                                sub.pattern.kind,
                                thir::PatKind::Binding {
                                    mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, _),
                                    subpattern: None,
                                    ..
                                }
                            )
                    })
            })
    }

    /// Parameters taken apart where they're given, as THIR takes them: a
    /// props struct's, its fields' defaults and its rest among them (ADRs
    /// 0195, 0212). Each its index, THIR's pattern, which names what it
    /// binds, and the local MIR binds each to, by value.
    fn mir_param_patterns(&mut self, body: &mir::Body<'tcx>) -> Vec<ParamPattern> {
        let mut patterns = Vec::new();
        for (i, param) in self.thir.params.iter().enumerate() {
            let Some(pat) = param.pat.as_deref() else { continue };
            if !matches!(pat.kind, thir::PatKind::Leaf { .. }) {
                continue;
            }
            // One with a destructor, only where each part that has one is
            // bound, moved out whole: none is left of it to drop.
            if self.has_drops(pat.ty) && !self.binds_dropped_fields(pat) {
                continue;
            }
            // Each binding, and the local MIR binds it to, by its span.
            let mut bound = Vec::new();
            let mut by_value = true;
            pat.walk_always(|p| match p.kind {
                thir::PatKind::Binding {
                    var,
                    mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, _),
                    subpattern: None,
                    ..
                } => bound.push((var, p.span)),
                thir::PatKind::Binding { .. } => by_value = false,
                _ => {}
            });
            let locals: Option<Vec<Local>> = (bound.iter())
                .map(|&(_, span)| {
                    body.var_debug_info.iter().find_map(|info| match info.value {
                        mir::VarDebugInfoContents::Place(place)
                            if place.projection.is_empty() && info.source_info.span == span =>
                        {
                            Some(place.local)
                        }
                        _ => None,
                    })
                })
                .collect();
            let Some(locals) = locals.filter(|_| by_value) else {
                continue;
            };
            let Some((pattern, _)) = self.js_pattern(pat) else {
                continue;
            };
            let named: Option<Vec<(Local, String)>> = (bound.iter().zip(locals))
                .map(
                    |(&(var, _), local)| match self.locals.vars.get(&var).map(|v| &v.place.kind) {
                        Some(js::ExprKind::Var(name)) => Some((local, name.clone())),
                        _ => None,
                    },
                )
                .collect();
            if let Some(named) = named {
                patterns.push((i, pattern, named));
            }
        }
        patterns
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
        // What it shares a reference to nothing changes while it runs
        // (borrowck says so): there, it reads alike.
        let mut shared = Vec::new();
        let upvars = match self.tcx.type_of(def_id).skip_binder().kind() {
            ty::Closure(_, args) => args.as_closure().upvar_tys().to_vec(),
            _ => Vec::new(),
        };
        for (i, value) in captured.into_iter().enumerate() {
            if let Value::Place(place) = &value
                && let js::ExprKind::Var(name) = &place.kind
                && self.locals.alike.insert(name.clone())
            {
                shared.push(name.clone());
            }
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
                            && state.locals.writes[l] > 1
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
        // Its own THIR's shape hints, and what's known of it, where its body
        // is written.
        let outer = std::mem::replace(&mut self.thir, &body.thir);
        let outer_facts = std::mem::replace(&mut self.body_facts, &body.facts);
        let lowered = self.mir_function(mir, Some(captures));
        self.thir = outer;
        self.body_facts = outer_facts;
        for name in shared {
            self.locals.alike.remove(&name);
        }
        let (mut params, mut stmts) = lowered?;
        // The functions written in it, in its body, as THIR's (ADR 0308).
        let written: Vec<rustc_span::def_id::DefId> = (self.krate.local_functions.keys())
            .filter(|&&id| self.tcx.parent(id) == def_id)
            .copied()
            .collect();
        // JS ignores extra arguments, so `|_| ..` is `() => ..`, as THIR's,
        // unless the closure drops what it's given.
        let given = &body.thir.params.raw[1.min(body.thir.params.len())..];
        if given.len() == params.len() {
            while let Some(last) = given.get(params.len().wrapping_sub(1))
                && last
                    .pat
                    .as_deref()
                    .is_some_and(|p| matches!(p.kind, thir::PatKind::Wild))
                && !self.has_drops(last.ty)
            {
                params.pop();
            }
        }
        // An `async` closure's `|_|`, which its MIR leaves unnamed, and its
        // THIR binds as `__arg0`: read nowhere, none either.
        while let Some(js::Pattern::Name(name)) = params.last()
            && (name == "_" || name.starts_with("_$"))
            && js::mentions_in(&stmts, name) == 0
        {
            params.pop();
        }
        // An `async` body runs as soon as it's made: an async arrow, called
        // at once (ADR 0029).
        if self.tcx.coroutine_is_async(def_id) {
            self.place_written(written, &mut stmts);
            return Ok(Expr::call(Expr::async_arrow(params, stmts), Vec::new()));
        }
        // An `async` closure's body is the one it makes: its async arrow's.
        if matches!(self.tcx.type_of(def_id).skip_binder().kind(), ty::CoroutineClosure(..))
            && let Some(mut body) = called_async_body(&mut stmts)
        {
            self.place_written(written, &mut body);
            return Ok(Expr::async_arrow(params, body));
        }
        // A closure of `()` that only spawns an async block is that block's
        // async arrow, as THIR's: it starts the same, and gives back a
        // promise where nothing reads its `()` (ADR 0257).
        if let ty::Closure(_, args) = self.tcx.type_of(def_id).skip_binder().kind()
            && written.is_empty()
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
        let forwarding = self.closure_calls_rust(body, false);
        let concise = self.closure_calls_for_nothing(body);
        if forwarding || concise {
            let span = self.js_span(span);
            if forwarding {
                self.locals.forwarding.push(span);
            }
            if concise {
                self.locals.concise.push(span);
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
    /// declaration's, which JS hoists, as THIR places it (ADR 0308).
    fn place_written(&self, written: Vec<rustc_span::def_id::DefId>, out: &mut Vec<Stmt>) {
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

    /// Each local's name, the user's where it has one, and how it's used.
    fn mir_locals(&mut self, body: &mir::Body<'tcx>, named: &[(Local, String)]) -> Locals {
        let n = body.local_decls.len();
        let mut locals = Locals {
            names: IndexVec::from_elem_n(String::new(), n),
            reads: IndexVec::from_elem_n(0, n),
            writes: IndexVec::from_elem_n(0, n),
            borrowed: IndexVec::from_elem_n(false, n),
            fields: IndexVec::from_elem_n(Vec::new(), n),
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
            // one the program names, nor `.await`'s `result`.
            if let mir::VarDebugInfoContents::Place(place) = info.value
                && place.projection.is_empty()
                && user[place.local].is_none()
                && !info.source_info.span.in_external_macro(sm)
                && !info.source_info.span.is_desugaring(rustc_span::DesugaringKind::Await)
                && !continued.contains(&place.local)
            {
                // A method's `self`, named after its type, `counter` for a
                // `Counter`, as THIR names it.
                let name = match body.local_decls[place.local].ty.peel_refs().kind() {
                    ty::Adt(adt, _) if info.name.as_str() == "self" => {
                        super::lower_first(self.tcx.item_name(adt.did()).as_str())
                    }
                    _ => info.name.to_string(),
                };
                user[place.local] = Some(super::camel_case(&name));
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
                        // A shared borrow of a part nothing inside can change, a
                        // match guard's `&(k as Warn).level`: the value it is.
                        let frozen = matches!(rvalue, Rvalue::Ref(_, BorrowKind::Shared | BorrowKind::Fake(_), borrowed)
                            if borrowed.ty(body, self.tcx).ty.is_freeze(self.tcx, self.typing_env));
                        count_rvalue(&mut locals, rvalue, frozen);
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
                // A check THIR knows passes reads nothing: it's left out; nor
                // one made where the element is read.
                TerminatorKind::Assert { msg, target, .. }
                    if self.known_in_bounds(msg, data.terminator().source_info.span)
                        || checked_by_division(msg)
                        || deferred_check(body, msg, *target).is_some() => {}
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
            // Bound by a parameter's pattern, as THIR names it.
            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local) {
                locals.names[local] = name.clone();
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
            && (state.locals.reads[local] == 1 || split(&state.locals, local))
            && !state.locals.borrowed[local]
            && !state.patterned.contains(&local)
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
            // Where it starts, what the program wrote: a `while`'s test.
            let data = &state.body.basic_blocks[block];
            let start = (data.statements.iter())
                .find(|s| {
                    !matches!(
                        s.kind,
                        StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop
                    )
                })
                .map_or(data.terminator().source_info.span, |s| s.source_info.span);
            return Ok(vec![
                StmtKind::While {
                    label: Some(label(block)),
                    cond: Expr::bool(true),
                    body,
                }
                .at(self.js_span(start.source_callsite())),
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
        // where a person writes it; at a `return` the program wrote, where
        // what jumps there is, if it's one.
        if is_return(state.body, to) {
            let mut out = self.mir_terminator(state, to)?;
            // Back through the drops on the way, to where the value is made.
            let mut blocks = vec![from];
            let preds = state.body.basic_blocks.predecessors();
            while let Some(&block) = blocks.last()
                && !(state.body.basic_blocks[block].statements.iter())
                    .any(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.local == mir::RETURN_PLACE))
                && let [only] = preds[block].as_slice()
                && !blocks.contains(only)
            {
                blocks.push(*only);
            }
            // What each does, not a temporary's storage ending, which a `?`'s
            // has where it's written, on its way on too.
            let spans = (blocks.iter())
                .flat_map(|&b| {
                    let data = &state.body.basic_blocks[b];
                    (data.statements.iter())
                        .filter(|s| !matches!(s.kind, StatementKind::StorageLive(_) | StatementKind::StorageDead(_)))
                        .map(|s| s.source_info.span)
                        .chain([data.terminator().source_info.span])
                })
                .map(|span| self.js_span(span));
            let written = spans
                .filter_map(|s| (state.returns.iter()).find(|w| !s.is_none() && w.lo <= s.lo && s.hi <= w.hi))
                .next()
                .copied();
            if let Some(written) = written
                && let Some(stmt) = out.last_mut().filter(|s| matches!(s.kind, StmtKind::Return(_)))
            {
                stmt.span = written;
            }
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

    /// A block's statements.
    fn mir_statements(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        state.at = block;
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
                let mut otherwise = Some(targets.otherwise()).filter(|&b| reached(b));
                // Values that go to one block, an or-pattern's `'+' | '-'`:
                // one test of either, the block once, as the program wrote it.
                let mut grouped: Vec<(Expr, BasicBlock)> = Vec::new();
                for (test, target) in cases {
                    match grouped.iter_mut().find(|(_, t)| *t == target) {
                        Some((either, _)) => {
                            *either = Expr::bin(Op::Or, std::mem::replace(either, Expr::undefined()), test);
                        }
                        None => grouped.push((test, target)),
                    }
                }
                let mut cases = grouped;
                // Each case tested, an enum's `match`'s: its arms in the order
                // the program wrote them, not its variants'.
                if otherwise.is_none()
                    && let Some(at) = (cases.iter())
                        .map(|&(_, b)| written_at(state.body, b))
                        .collect::<Option<Vec<_>>>()
                {
                    let mut ordered: Vec<_> = at.into_iter().zip(std::mem::take(&mut cases)).collect();
                    ordered.sort_by_key(|&(at, _)| at);
                    cases = ordered.into_iter().map(|(_, case)| case).collect();
                }
                // `if c { a } else { b }`, MIR's test of `false`, `!c`, first:
                // the program's, `c`, its branch `a` first. Not `if !c { a }
                // else { b }`'s, which MIR tests as `c`, `a` its `false`.
                if ty.is_bool()
                    && targets.iter().all(|(value, _)| value == 0)
                    && let [(test, target)] = cases.as_mut_slice()
                    && let Some(yes) = otherwise
                    && !matches!((written_at(state.body, *target), written_at(state.body, yes)), (Some(no), Some(yes)) if no < yes)
                {
                    *test = negate(std::mem::replace(test, Expr::undefined()));
                    otherwise = Some(std::mem::replace(target, yes));
                }
                let mut chain_panics = otherwise.is_some_and(|b| panics(state.body, b));
                let mut chain: Option<Vec<Stmt>> = match otherwise {
                    Some(b) => Some(self.mir_branch(state, block, b)?),
                    None => None,
                };
                // Without an `otherwise`, the last case is what's left.
                if chain.is_none()
                    && let Some((_, last)) = cases.pop()
                {
                    chain_panics = panics(state.body, last);
                    chain = Some(self.mir_branch(state, block, last)?);
                }
                for (test, target) in cases.into_iter().rev() {
                    let then = self.mir_branch(state, block, target)?;
                    let branches_panic = [panics(state.body, target), chain_panics];
                    chain = Some(early_exit(test, then, chain, branches_panic, self.js_span(span)));
                    chain_panics = false;
                }
                out.extend(chain.unwrap_or_default());
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
                unwind,
            } if self.known_in_bounds(msg, span) || checked_by_division(msg) => {
                // `args[i]` THIR's facts know is in bounds, of `0..args.len()`:
                // unchecked, as THIR writes it.
                let _ = (cond, expected, unwind);
                out.extend(self.mir_branch(state, block, *target)?);
            }
            // A bounds check, where the element's read or written next.
            TerminatorKind::Assert { msg, target, .. }
                if let Some((index, write)) = deferred_check(state.body, msg, *target) =>
            {
                state.deferred.insert(index, write);
                out.extend(self.mir_branch(state, block, *target)?);
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
                let local = destination.local;
                state.discarding = destination.projection.is_empty()
                    && (state.locals.names[local].is_empty()
                        || (!state.locals.user[local]
                            && state.locals.reads[local] == 0
                            && !state.locals.borrowed[local]));
                let value = match state.awaits.contains(&block) {
                    true => self.mir_await(state, args, *fn_span, &mut called),
                    false => self.mir_call(state, func, args, *destination, *fn_span, &mut called),
                };
                state.discarding = false;
                let value = value?;
                // What the call writes, a setter's `n.textContent = "x"` or a
                // cell's `count = 5`, after what was made before it, which
                // may read what it writes.
                if called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {
                    let mut before = Vec::new();
                    self.flush(state, &mut before)?;
                    before.append(&mut called);
                    called = before;
                }
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
            let mut expr = self.value_expr(value, Span::default())?;
            let named = state.named.contains(&local);
            if self.reads_same_anywhere(&expr, state.body.local_decls[local].ty) && !named {
                state.in_place.insert(local, expr);
                continue;
            }
            // An element, or an object, what it's made of read here, made
            // where it's used.
            let before = made.len();
            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {
                for stmt in &made[before..] {
                    match &stmt.kind {
                        StmtKind::Const(name, _) => {
                            self.locals.captured.insert(name.clone());
                            self.locals.alike.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        // What a branch's element reads, set where its test
                        // holds: no `const`, but MIR's too.
                        StmtKind::Let(name, None) => {
                            self.locals.captured.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        _ => {}
                    }
                }
                state.in_place.insert(local, expr);
                continue;
            }
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
        if let Some(value) = state.in_place.get(&local) {
            return Ok(Value::Expr(value.clone()));
        }
        if let Some(place) = state.refs.get(&local) {
            return Ok(Value::Ref(place.clone()));
        }
        if let Some(value) = state.places.get(&local) {
            return Ok(value.clone());
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

    /// Whether `e` reads the same wherever it's read, so it's written there:
    /// a constant, what the module names, a function or a static, a variable
    /// nothing changes or a field of plain data in one, of type `ty`.
    fn reads_same_anywhere(&self, e: &Expr, ty: Ty<'tcx>) -> bool {
        let alike = |e: &Expr| match &e.kind {
            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.alike.contains(name),
            js::ExprKind::Symbol(_) => true,
            js::ExprKind::Member(..) => {
                let mut root = e;
                while let js::ExprKind::Member(object, _) = &root.kind {
                    root = object;
                }
                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.plain.contains(name))
            }
            _ => e.is_constant(),
        };
        // A context's `Provider` or `Consumer`, `react::provider`'s, which
        // React never changes.
        let context_member = super::recognition::is_context_member(self.tcx, ty);
        match &e.kind {
            js::ExprKind::Member(..) if context_member => true,
            _ => alike(e),
        }
    }

    /// Whether an `Assert` at `span` checks an index THIR's facts know is in
    /// bounds: the index there, `a[i]`, is among them.
    fn known_in_bounds(&self, msg: &mir::AssertMessage<'tcx>, span: Span) -> bool {
        matches!(msg, mir::AssertKind::BoundsCheck { .. })
            && (self.thir.exprs.iter_enumerated()).any(|(id, e)| {
                matches!(e.kind, thir::ExprKind::Index { .. })
                    && e.span == span
                    && self.body_facts.in_bounds.contains(&id)
            })
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
        // A `None` only nullable fields read: `null` (ADR 0275).
        if place.projection.is_empty()
            && state.null_nones.contains(&place.local)
            && let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(_, variant, ..) = **kind
            && variant.as_u32() == 0
        {
            return self.mir_store(state, *place, Value::Expr(Expr::null()), span, out);
        }
        // One with a flattened field, made outside JSX, an object of its
        // fields and the flattened one's (ADR 0204).
        if let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(..) = **kind
            && !state.jsx_props.contains(&place.local)
        {
            let ty = place.ty(&state.body.local_decls, self.tcx).ty;
            if bindings::has_flatten(self.tcx, ty) {
                let made = self.mir_rvalue(state, rvalue, span, out)?;
                let made = self.value_expr(made, span)?;
                let made = Value::Expr(self.flattened_object(made, ty));
                return self.mir_store(state, *place, made, span, out);
            }
        }
        // A part of a parameter its pattern binds, bound there already.
        if let Some(&arg) = state.parts.get(&place.local)
            && place.projection.is_empty()
            && matches!(rvalue, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _) if p.local == arg)
        {
            return Ok(());
        }
        // A `let`'s pattern's rest, `let P { href, rest } = p` (ADR 0291).
        if place.projection.is_empty() && state.rests.contains(&place.local) {
            return self.mir_let_rest(state, place.local, rvalue, span, out);
        }
        state.binding_var = place.projection.is_empty() && state.locals.user[place.local];
        // A flattened struct a `match`'s pattern binds, which JS's props hold
        // flat, among their other fields, as THIR says.
        if state.binding_var
            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue
            && let Some((base, PlaceElem::Field(field, _))) = read.as_ref().last_projection()
            && bindings::is_flatten_field(
                self.tcx,
                Place::ty_from(base.local, base.projection, &state.body.local_decls, self.tcx).ty,
                field.as_usize(),
            )
        {
            return Err(self.unsupported(
                span,
                "flattened props taken apart here: take them apart where they're given, `fn f(Props { a, anchor }: Props)`",
            ));
        }
        let value = self.mir_rvalue(state, rvalue, span, out);
        state.binding_var = false;
        self.mir_store(state, *place, value?, span, out)
    }

    /// `rest = p.rest` of a `let`'s pattern, `let P { href, rest, .. } = p`:
    /// what's left of the props, each of their other fields named, as THIR
    /// names them, `const { href: _href, title: _title, ...rest } = p;`,
    /// whose names tidy makes the pattern's where it binds them.
    fn mir_let_rest(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let decls = &state.body.local_decls;
        let props = match rvalue {
            Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) => match read.as_ref().last_projection() {
                Some((base, PlaceElem::Field(field, _))) => {
                    let ty = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
                    let place = Place {
                        local: base.local,
                        projection: self.tcx.mk_place_elems(base.projection),
                    };
                    bindings::is_rest_field(self.tcx, ty, field.as_usize()).then_some((place, ty))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((place, ty)) = props.filter(|_| !state.locals.declared[local]) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let Shape::Object(fields) = self.shape(ty) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let value = self.mir_place(state, place, span, out)?;
        let tcx = self.tcx;
        let named = (fields.iter().enumerate())
            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))
            .map(|(_, (key, _))| {
                let unused = self.fresh(&format!("_{}", super::camel_case(&js_ident(key))));
                (key.clone(), js::Pattern::Name(unused), None)
            })
            .collect();
        self.flush(state, out)?;
        state.locals.declared[local] = true;
        out.push(
            StmtKind::Destructure {
                pattern: js::Pattern::Object(named, Some(state.locals.names[local].clone())),
                value,
                mutable: false,
            }
            .at(self.js_span(span)),
        );
        Ok(())
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
        // A field's path stored in a local, a getter's `element.isContentEditable`,
        // or a call's, `isItem()` of `isItem(item)`: whether it's a `bool`, as
        // of a place read (`mir_place`).
        if place.projection.is_empty()
            && let Value::Expr(e) = &value
            && let Some(path) = tidy::field_path(e).or_else(|| tidy::call_path(e))
        {
            let bool = state.body.local_decls[local].ty.is_bool();
            *self.locals.paths.entry(path).or_insert(bool) &= bool;
        }
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
            let mut tried = tried.clone();
            // An `Option`'s held, which is what a `let` binds of it, `let f =
            // xs.find(..)?;`: held under that name, `f`, bound there already.
            if let js::ExprKind::Var(held) = &tried.kind
                && self.option_of(*ty).is_some_and(|inner| !self.boxed_payload(inner))
                && let Some((bound, from)) = continued_binding(state, local)
                && let Some(Stmt {
                    kind: StmtKind::Const(name, _),
                    ..
                }) = (out.iter_mut().rev()).find(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == held))
            {
                *name = state.locals.names[bound].clone();
                tried = Expr::var(name);
                state.parts.insert(bound, from);
                state.locals.declared[bound] = true;
            }
            state.branches.insert(local, (tried, *ty));
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
        // A shared reference a temporary or a variant's binding holds, made
        // once, to a place of a variable nothing changes, `&route.title`: the
        // place, read where it's used, as THIR reads a reference (ADR 0023);
        // borrowck keeps it the same meanwhile.
        // And a tuple of them, `format_args!`'s.
        let held = |value: &Value<'tcx>| match value {
            Value::Place(target)
                if same_place(state, target)
                    && root_var(target).is_some_and(|root| self.locals.alike.contains(root)) =>
            {
                Some(target.clone())
            }
            _ => None,
        };
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `named_temporaries`).
        let of_temporary = matches!(&value, Value::Place(target) if matches!(&target.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l])));
        let one = place.projection.is_empty()
            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local];
        let places = match &value {
            Value::List(items) => items
                .iter()
                .map(held)
                .collect::<Option<Vec<_>>>()
                .map(|items| Value::List(items.into_iter().map(Value::Place).collect())),
            value => held(value).map(Value::Place),
        };
        if one && let Some(places) = places {
            state.places.insert(local, places);
            return Ok(());
        }
        // One read by its fields, each once, a `format_args!`'s tuple: a list
        // whose items are each read where its field is.
        let parts = match &value {
            Value::List(items) => {
                let mut read = state.locals.fields[local].clone();
                read.sort();
                read == (0..items.len()).collect::<Vec<_>>()
            }
            _ => false,
        };
        if place.projection.is_empty() && self.foldable(state, local) && (state.locals.reads[local] == 1 || parts) {
            state.pending.push((local, value));
            return Ok(());
        }
        let expr = self.value_expr(value, span)?;
        let js_span = self.js_span(span);
        // A temporary made once of what reads the same anywhere, however
        // often it's read, or what a variant's pattern binds to it: written
        // where it is.
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `aliases`).
        let of_temporary = matches!(&expr.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l]));
        if place.projection.is_empty()
            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local]
            && !state.named.contains(&local)
            && self.reads_same_anywhere(&expr, state.body.local_decls[local].ty)
        {
            state.in_place.insert(local, expr);
            return Ok(());
        }
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
            // An `Rc` or a `Weak` is a pointer its owners share: the place
            // given another, not the one it holds changed, which every
            // owner would see.
            if self.counted_rc(pointee).is_some() || self.weak_of(pointee).is_some() {
                return Err(self.unsupported(span, "an `Rc` or a `Weak` written through a `&mut`, from its MIR"));
            }
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
        let e = self.place_expr(state, place, span, out)?;
        // A field's path, `element.editable`: whether it's a `bool`, which
        // tidy writes an `||` of as one.
        if let Some(path) = tidy::field_path(&e) {
            let bool = place.ty(state.body, self.tcx).ty.is_bool();
            *self.locals.paths.entry(path).or_insert(bool) &= bool;
        }
        Ok(e)
    }

    fn place_expr(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A field of an update's derived `Default` base: its default.
        if let Some(defaults) = state.defaults.get(&place.local)
            && let [PlaceElem::Field(field, _)] = place.projection.as_slice()
        {
            return Ok(defaults[field.as_usize()].clone());
        }
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
            match (
                state.in_place.get(&place.local),
                state.locals.names[place.local].as_str(),
            ) {
                (Some(value), _) => value.clone(),
                (None, "") => Expr::undefined(),
                (None, name) => {
                    state.named.insert(place.local);
                    Expr::var(name)
                }
            }
        } else if matches!(
            state.body.local_decls[place.local].ty.peel_refs().kind(),
            ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..)
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
        let last = place.projection.len();
        for (at, elem) in place.projection.iter().enumerate() {
            // Its bounds checked here: `$index(xs, i)` where it's read,
            // `xs[$at(xs, i)]` where it's the place written.
            if let PlaceElem::Index(local) = elem
                && let Some(write) = state.deferred.remove(&local)
            {
                let items = match value.reads_same() {
                    true => value,
                    false => self.spill("items", value, out),
                };
                let index = self.read_local(state, local, out)?;
                value = match write && at + 1 == last {
                    true => {
                        self.runtime.insert(Helper::At);
                        Expr::index(items.clone(), Expr::call(Expr::var("$at"), vec![items, index]))
                    }
                    false => {
                        self.runtime.insert(Helper::Index);
                        Expr::call(Expr::var("$index"), vec![items, index])
                    }
                };
                placed = false;
                ty = ty.projection_ty(self.tcx, elem);
                continue;
            }
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
                (ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..), PlaceElem::Field(field, _))
                    if !state.captures.is_empty() =>
                {
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
            // JS's props have no `rest`: what's left of them is taken apart
            // from the rest, `{ href, ...rest }` (ADR 0195), as THIR says.
            PlaceElem::Field(_, field_ty) if bindings::is_rest(self.tcx, field_ty) => {
                // Bound by a pattern other than a `let`'s, a `match`'s.
                return Err(self.unsupported(
                    span,
                    match state.binding_var {
                        true => {
                            "a `Rest` of props taken apart here: take them apart where they're given, `fn f(Props { a, rest }: Props)`"
                        }
                        false => "reading a `Rest` of props: take it apart from them, `LinkProps { href, rest }: LinkProps`",
                    },
                ));
            }
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
            // Counted from the end, a slice pattern's suffix: `xs[xs.length -
            // 1]`, or of an array, its index, as THIR's patterns read it.
            PlaceElem::ConstantIndex {
                offset, from_end: true, ..
            } if base.reads_same() => {
                let index = match self.array_length(ty.ty) {
                    Some(n) => Expr::int(n - i128::from(offset)),
                    None => Expr::bin(Op::Sub, Expr::member(base.clone(), "length"), Expr::int(offset.into())),
                };
                Expr::index(base, index)
            }
            // A slice pattern's rest, `xs.slice(1, xs.length - 1)`: a copy, as
            // `&v[a..b]` is (ADR 0063), which a `&mut` to can't be (above).
            PlaceElem::Subslice { from, to, from_end } if base.reads_same() => {
                let mut range = vec![Expr::int(from.into())];
                match (from_end, self.array_length(ty.ty)) {
                    (true, _) if to == 0 => {}
                    (true, Some(n)) => range.push(Expr::int(n - i128::from(to))),
                    (true, None) => range.push(Expr::bin(
                        Op::Sub,
                        Expr::member(base.clone(), "length"),
                        Expr::int(to.into()),
                    )),
                    (false, _) => range.push(Expr::int(to.into())),
                }
                Expr::call(Expr::member(base, "slice"), range)
            }
            _ => return Err(self.unsupported(span, "this place, from its MIR")),
        })
    }

    /// An array type's length, rustc's.
    fn array_length(&self, ty: Ty<'tcx>) -> Option<i128> {
        match ty.kind() {
            ty::Array(_, len) => len.try_to_target_usize(self.tcx).map(i128::from),
            _ => None,
        }
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

    /// `read`, `place`'s value, of a props field with a default, what JS
    /// takes apart with it (ADR 0212): the default where it's `undefined`,
    /// `o.k === undefined ? d : o.k`, which a pattern of it writes `{ k = d }`.
    fn with_default(&mut self, state: &State<'_, 'tcx>, place: Place<'tcx>, read: Expr, span: Span) -> Expr {
        let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() else {
            return read;
        };
        let base_ty = Place::ty_from(base.local, base.projection, &state.body.local_decls, self.tcx);
        if base_ty.variant_index.is_some() || !matches!(base_ty.ty.kind(), ty::Adt(adt, _) if adt.is_struct()) {
            return read;
        }
        match self.prop_default(base_ty.ty, field.as_usize(), span) {
            Some(default) => Expr::cond(Expr::bin(js::Op::Eq, read.clone(), Expr::undefined()), default, read),
            None => read,
        }
    }

    /// Where each range pattern of the body is, in a `match`'s arms, a
    /// `let`'s or an `if let`'s.
    fn range_patterns(&self) -> Vec<Span> {
        let mut ranges = Vec::new();
        let mut find = |pat: &thir::Pat<'tcx>| {
            pat.walk_always(|p| {
                if let thir::PatKind::Range(_) = p.kind {
                    ranges.push(p.span);
                }
            })
        };
        for arm in self.thir.arms.iter() {
            find(&arm.pattern);
        }
        for expr in self.thir.exprs.iter() {
            if let thir::ExprKind::Let { pat, .. } = &expr.kind {
                find(pat);
            }
        }
        for stmt in self.thir.stmts.iter() {
            if let thir::StmtKind::Let { pattern, .. } = &stmt.kind {
                find(pattern);
            }
        }
        ranges
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

    /// A field of a list made and not yet read, `copy _5.0`, or a reborrow
    /// of what it refers to, `&(*(_5.0))`, a `format_args!`'s argument: its
    /// item, where those before it are taken already, or move freely.
    fn take_field(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let (PlaceElem::Field(field, _), [] | [PlaceElem::Deref]) = (
            place.projection.first().copied().unwrap_or(PlaceElem::Deref),
            place.projection.get(1..).unwrap_or(&[]),
        ) else {
            return Ok(None);
        };
        let field = field.as_usize();
        let pending = state.pending.iter().find(|(l, _)| *l == place.local).map(|(_, v)| v);
        let Some(Value::List(items)) = pending else {
            return Ok(None);
        };
        // Read out of order, or one that does something before others that
        // do, which made after it would run first: the list made whole
        // first, its fields read by its name.
        let mut in_order = items[..field].iter().all(|item| movable(state, item))
            && (movable(state, &items[field]) || items[field + 1..].iter().all(|item| movable(state, item)));
        // None read yet: each that does something held first, in order, as
        // THIR holds them, the rest read where they are: `_arg`, named for
        // what it's given as once tidied, `const key = …`, or `arg`.
        let acting: Vec<usize> = (items.iter().enumerate())
            .filter(|(_, item)| !movable(state, item) && matches!(item, Value::Expr(e) if e.has_effects()))
            .map(|(k, _)| k)
            .collect();
        if !in_order && !acting.is_empty() && !state.taken.contains(&place.local) {
            let at = state
                .pending
                .iter()
                .position(|(l, _)| *l == place.local)
                .expect("pending");
            let rest = state.pending.split_off(at);
            self.flush(state, out)?;
            state.pending = rest;
            let span = state.body.local_decls[place.local].source_info.span;
            for k in acting {
                let Value::List(items) = &mut state.pending[0].1 else {
                    unreachable!("a list")
                };
                let item = std::mem::replace(&mut items[k], Value::Expr(Expr::undefined()));
                let item = self.value_expr(item, span)?;
                let held = self.spill("_arg", item, out);
                let Value::List(items) = &mut state.pending[0].1 else {
                    unreachable!("a list")
                };
                items[k] = Value::Expr(held);
            }
            in_order = true;
        }
        if !in_order || !self.admit(state, &[place.local], true, out)? {
            if let Some(at) = state.pending.iter().position(|(l, _)| *l == place.local) {
                let rest = state.pending.split_off(at + 1);
                self.flush(state, out)?;
                state.pending = rest;
            }
            return Ok(None);
        }
        let at = state
            .pending
            .iter()
            .position(|(l, _)| *l == place.local)
            .expect("admitted");
        let Value::List(items) = &mut state.pending[at].1 else {
            unreachable!("a list")
        };
        let item = std::mem::replace(&mut items[field], Value::Expr(Expr::undefined()));
        state.taken.push(place.local);
        if state.taken.iter().filter(|&&l| l == place.local).count() == items.len() {
            state.pending.remove(at);
        }
        Ok(Some(item))
    }

    fn mir_operand_read(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if let Operand::Copy(place) | Operand::Move(place) = operand
            && let Some(item) = self.take_field(state, place, out)?
        {
            return Ok(item);
        }
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
                let read = self.mir_place(state, *place, span, out)?;
                Ok(Value::Expr(self.with_default(state, *place, read, span)))
            }
            // A function given by reference, `&hello`, a promoted constant:
            // the function, as a reference is what it refers to (ADR 0023).
            Operand::Constant(c)
                if let Const::Unevaluated(uv, _) = c.const_
                    && let Some(promoted) = uv.promoted
                    && let Some(referred) = referred_constant(&state.promoted[promoted])
                    && fn_def(referred.ty()).is_some() =>
            {
                self.mir_operand_read(state, &Operand::Constant(Box::new(referred)), out)
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
            // A promoted reference to a closure that captures nothing, `&|n|
            // LAST.set(n)`: the closure, as a reference is what it refers to.
            Operand::Constant(c)
                if let Const::Unevaluated(uv, _) = c.const_
                    && let Some(promoted) = uv.promoted
                    && let Some(def_id) = referred_closure(&state.promoted[promoted]) =>
            {
                Ok(Value::Expr(self.mir_closure(state, def_id, Vec::new(), c.span, out)?))
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
        // A promoted reference to a variant without fields, `&None` of generic
        // code, which can't be evaluated: the variant, as it's made.
        if let Const::Unevaluated(uv, _) = c.const_
            && let Some(promoted) = uv.promoted
            && let Some((made, variant)) = referred_variant(tcx, &state.promoted[promoted])
        {
            return self.adt_value(made, variant, Vec::new(), &[], c.span);
        }
        // A promoted reference to a call of nothing, `&size_of::<T>()` of
        // generic code: the call, a std function's of nothing it's given, or
        // the crate's const function's.
        if let Const::Unevaluated(uv, _) = c.const_
            && let Some(promoted) = uv.promoted
            && let Some((def_id, args, output)) = referred_call(&state.promoted[promoted])
        {
            if self.is_rust_fn(def_id) {
                let given = self.evidence_args(def_id, args, c.span)?;
                return Ok(Expr::call(self.fn_ref(def_id), given));
            }
            if let Some(known) = self.recognition().classify(def_id, args) {
                let mut made = Vec::new();
                let value = self.std_by_values(known, def_id, args, &[], Vec::new(), output, c.span, &mut made)?;
                if made.is_empty() {
                    return Ok(value);
                }
            }
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
        if let ty::Ref(_, _, ty::Mutability::Not) | ty::RawPtr(_, ty::Mutability::Not) = ty.kind()
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
        let value = (self.mir_const_value(c.const_, ty, c.span))
            .and_then(|value| const_js(tcx, value))
            .ok_or_else(|| self.unsupported(c.span, "this constant, from its MIR"))?;
        // One written across lines keeps them, a template literal, as THIR's.
        Ok(match &value.kind {
            js::ExprKind::Str(text) if text.contains('\n') && super::literal_across_lines(tcx, c.span) => {
                Expr::lines(text.as_str())
            }
            _ => value,
        })
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
                // A variant without fields, of an enum whose value isn't a
                // tree, `None` of an `Option<Box<T>>`: its index, as a tree of
                // an enum starts.
                if ty.is_enum()
                    && let Some(made) = self.tcx.try_destructure_mir_constant_for_user_output(value, ty)
                    && made.fields.is_empty()
                    && let Some(variant) = made.variant
                {
                    let index = ty::ValTree::from_scalar_int(self.tcx, variant.as_u32().into());
                    let index = ty::Const::new_value(self.tcx, index, self.tcx.types.u32);
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_branches(self.tcx, [index]),
                    });
                }
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
                // What has no fields, `Marker`'s, a tree of none.
                if let mir::ConstValue::ZeroSized = value {
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::zst(self.tcx),
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
                // A part of a slice's copy, a slice pattern's rest, can't be
                // written through.
                if matches!(kind, BorrowKind::Mut { .. })
                    && place
                        .projection
                        .iter()
                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))
                {
                    return Err(self.unsupported(span, "a `&mut` to part of a slice, from its MIR"));
                }
                // `&(*(_5.0))`, the reference `_5.0` is, a list's item.
                if matches!(kind, BorrowKind::Shared)
                    && let Some(item) = self.take_field(state, place, out)?
                {
                    return Ok(item);
                }
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
                if matches!(kind, BorrowKind::Shared) {
                    let read = self.with_default(state, *place, place_expr.clone(), span);
                    if !matches!(read.kind, js::ExprKind::Cond(..)) && place_expr.reads_same() {
                        return Ok(Value::Place(place_expr));
                    }
                    return Ok(Value::Expr(read));
                }
                place_expr
            }
            Rvalue::BinaryOp(op, operands) => {
                let (l, r) = &**operands;
                let (ty, r_ty) = (l.ty(decls, tcx), r.ty(decls, tcx));
                // A range pattern's test, `2 <= k`: the value first, `k >= 2`,
                // as THIR writes it; of a bound the type has anyway, `0 <= k`
                // of a `u8`, none.
                let flipped = match op {
                    BinOp::Le => Some(BinOp::Ge),
                    BinOp::Lt => Some(BinOp::Gt),
                    _ => None,
                };
                if let Some(flipped) = flipped
                    && let Operand::Constant(bound) = l
                    && !matches!(r, Operand::Constant(_))
                    && state.ranges.iter().any(|range| range.contains(span))
                {
                    if *op == BinOp::Le
                        && ty.is_integral()
                        && !ty.is_signed()
                        && bound.const_.try_to_scalar_int().is_some_and(|n| n.is_null())
                    {
                        return Ok(Value::Expr(Expr::bool(true)));
                    }
                    let [r, l]: [Value<'tcx>; 2] =
                        self.take_operands(state, &[r, l], out)?.try_into().ok().expect("two");
                    let (r, l) = (self.value_expr(r, span)?, self.value_expr(l, span)?);
                    return Ok(Value::Expr(self.binary(flipped, r, l, None, ty, span)?));
                }
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
                        // A `&dyn Debug` is the string it shows (ADR 0060), made
                        // where it's read: by `{:#?}`, as that shows it.
                        if self.is_dyn_debug(*to) && !self.is_dyn_debug(from) {
                            return Ok(Value::Fmt(Std::FmtDebug, self.pointee(from), value));
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
                // An enum's discriminant, `ordering as i32`'s: the enum made a
                // number, as THIR's `cast` makes it.
                let v = match self.mir_operand(state, operand, out)? {
                    Value::Discriminant(value, enum_ty) => {
                        return Ok(Value::Expr(self.cast(value, enum_ty, *to, span)?));
                    }
                    v => self.value_expr(v, Span::default())?,
                };
                if from == *to {
                    v
                } else if to.is_char() {
                    Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![v])
                } else if self.whole_cast_at(span) {
                    v
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
            // A borrow only for a slice's length, a slice pattern's test: the
            // slice, whose `PtrMetadata` is its `length`.
            Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, place) => self.mir_place(state, *place, span, out)?,
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
        // `P { x, ..base }`, where THIR says it is (`written_base`): its
        // other fields `base`'s, `{ ...base, x }`, as THIR writes it (ADR 0250).
        if let AggregateKind::Adt(did, VariantIdx::ZERO, args, None, None) = *kind
            && self.tcx.adt_def(did).is_struct()
            && let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(did), args)
            && let Some(spread) = self.written_base(state, ty, operands, span)
        {
            let Shape::Object(names) = self.shape(ty) else {
                unreachable!("a struct of named fields")
            };
            let base = self.mir_place(state, spread, span, out)?;
            let of_base = |i: usize, operand: &Operand<'tcx>| {
                operand.place().is_some_and(|p| {
                    p.local == spread.local
                        && p.projection.split_last().is_some_and(|(last, rest)| {
                            matches!(last, PlaceElem::Field(f, _) if f.as_usize() == i)
                                && rest == spread.projection.as_slice()
                        })
                })
            };
            let given: Vec<&Operand<'tcx>> = (operands.iter().enumerate())
                .filter(|&(i, o)| !of_base(i, o))
                .map(|(_, o)| o)
                .collect();
            let values = self.take_operands(state, &given, out)?;
            let mut props = vec![Prop::Spread(base)];
            let mut values = values.into_iter();
            for (i, operand) in operands.iter().enumerate() {
                if of_base(i, operand) {
                    continue;
                }
                let value = self.value_expr(values.next().expect("given"), span)?;
                props.push(Prop::Field(names[i].0.clone(), self.holding(value, names[i].1)));
            }
            return Ok(Value::Expr(Expr::object(props)));
        }
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
                let nulled: Vec<bool> = operands.iter().map(|o| self.nulled(state, o)).collect();
                Ok(Value::Expr(self.adt_value(ty, *variant_index, exprs, &nulled, span)?))
            }
            AggregateKind::Closure(def_id, _) => Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?)),
            // An `async` body, what it holds as a closure's environment.
            AggregateKind::Coroutine(def_id, _) if self.tcx.coroutine_is_async(*def_id) => {
                Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?))
            }
            // An `async` closure, which makes its body each call.
            AggregateKind::CoroutineClosure(def_id, _) => {
                Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?))
            }
            _ => Err(self.unsupported(span, "this aggregate, from its MIR")),
        }
    }

    /// Whether `operand`, an `Option`, is `null` or a value already, as a
    /// `#[rust_js::nullable]` field is (ADR 0275): such a field, moved
    /// where it's read, or bound to a variable, or a `Some(..)`, as THIR's
    /// `nullable` says.
    fn nulled(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(place) = operand.place() else {
            return false;
        };
        if let Some((PlaceElem::Field(field, _), parent)) = place.projection.split_last() {
            let parent = mir::PlaceRef {
                local: place.local,
                projection: parent,
            };
            let mir::PlaceTy { ty, variant_index } = parent.ty(state.body, self.tcx);
            return match ty.kind() {
                ty::Adt(adt, _) if !adt.is_enum() || variant_index.is_some() => {
                    let variant = adt.variant(variant_index.unwrap_or(VariantIdx::ZERO));
                    bindings::is_nullable(self.tcx, &variant.fields[*field])
                }
                _ => false,
            };
        }
        if !place.projection.is_empty() {
            return false;
        }
        if state.null_nones.contains(&place.local) {
            return true;
        }
        if state.locals.writes[place.local] != 1 {
            return false;
        }
        // What it's assigned, once: another such, or `Some(..)`.
        let assigned = (state.body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
            mir::StatementKind::Assign(assign) if assign.0 == place.local.into() => Some(&assign.1),
            _ => None,
        });
        // A clone of such, `m.rule_id.clone()`, of an item a clone gives as
        // it is, a string's, a number's or a reference's: its JS value.
        let itself = self
            .option_of(state.body.local_decls[place.local].ty)
            .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item));
        let cloned = (state.body.basic_blocks.iter()).find_map(|b| match &b.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if *destination == place.local.into() => Some((func, args)),
            _ => None,
        });
        if itself
            && let Some((func, args)) = cloned
            && let Some((def_id, _)) = fn_def(func.ty(state.body, self.tcx))
            && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone))
            && let [arg] = &args[..]
            && let Some(reference) = arg.node.place()
            && reference.projection.is_empty()
            && state.locals.writes[reference.local] == 1
            && let Some(Rvalue::Ref(_, _, borrowed)) = (state.body.basic_blocks.iter().flat_map(|b| &b.statements))
                .find_map(|s| match &s.kind {
                    mir::StatementKind::Assign(assign) if assign.0 == reference.local.into() => Some(&assign.1),
                    _ => None,
                })
        {
            return self.nulled(state, &Operand::Copy(*borrowed));
        }
        match assigned {
            Some(Rvalue::Use(operand, _)) => self.nulled(state, operand),
            Some(Rvalue::Aggregate(kind, fields)) => {
                fields.len() == 1
                    && matches!(**kind, AggregateKind::Adt(did, variant, ..)
                    if variant.as_u32() == 1 && self.tcx.is_lang_item(did, LangItem::Option))
            }
            _ => false,
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
                Ok(self.variant_test(enum_value.clone(), *enum_ty, *adt, variant))
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
    pub(super) fn variant_test(
        &mut self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
    ) -> Expr {
        let def = adt.variant(variant);
        if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
            let inner = self.option_of(ty).expect("an `Option`");
            return match self.tcx.is_lang_item(def.def_id, LangItem::OptionSome) {
                true => self.present(subject, inner),
                false => self.absent(subject, inner),
            };
        }
        // An untagged enum's variant, told by its value's kind (ADR 0214).
        if bindings::is_untagged(self.tcx, adt.did()) {
            let test = self.untagged_variant_test(ty, def, &subject);
            // A variant's test function it calls, `isValidElement(child)`, gives
            // a `bool`, which tidy writes a test of as one.
            test.clone().each_mut(&mut |e| {
                if let Some(path) = tidy::call_path(e) {
                    self.locals.paths.entry(path).or_insert(true);
                }
            });
            return test;
        }
        // A discriminated union's `otherwise`: an object whose tag is none
        // of the others' (ADR 0284).
        if bindings::is_tagged_otherwise(self.tcx, adt.did(), def) {
            let key = bindings::tag_key(self.tcx, adt.did());
            let tests = (adt.variants().iter())
                .filter(|other| other.def_id != def.def_id)
                .map(|other| {
                    let name = bindings::variant_tag(self.tcx, other);
                    Expr::bin(Op::Ne, Expr::member(subject.clone(), &key), name)
                })
                .reduce(|a, b| Expr::bin(Op::And, a, b));
            return tests.unwrap_or_else(|| Expr::bool(true));
        }
        if let Some(n) = super::recognition::ordering_value(self.tcx, adt.did(), def.name) {
            return Expr::bin(Op::Eq, subject, Expr::int(n));
        }
        let name = bindings::variant_tag(self.tcx, def);
        let unit = def.fields.is_empty() && bindings::declared_tag(self.tcx, adt.did()).is_none();
        match unit {
            true => Expr::bin(Op::Eq, subject, name),
            false => Expr::bin(
                Op::Eq,
                Expr::member(subject, bindings::tag_key(self.tcx, adt.did())),
                name,
            ),
        }
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
        // `..Default::default()` of a struct whose `Default` is derived, an
        // update's base: as THIR makes it, each default the update reads,
        // none it replaces that does nothing, a component's `children: C`
        // among them, which needs no dictionary then (`update_default`).
        if args.is_empty()
            && let Some(fields) = self.default_base_at(span)?
        {
            // Each field read where it's read, `{ size, tags: [] }`, of one
            // held whole; else the object of them.
            if destination.projection.is_empty() {
                let values = fields.into_iter().map(|(_, value)| value).collect();
                state.defaults.insert(destination.local, values);
                return Ok(Value::Expr(Expr::undefined()));
            }
            let props = fields
                .into_iter()
                .map(|(name, value)| Prop::Field(name, value))
                .collect();
            return Ok(Value::Expr(Expr::object(props)));
        }
        // A function a variable holds, `js::import!(f).await`'s: called as
        // that variable, which may be all that holds it, not as its import.
        // One a JS function, or the crate's that JS calls as it is, given no
        // dictionaries nor boxes.
        let called_as_js = match bindings::is_binding(self.tcx, def_id) {
            true => {
                !super::bindings::is_method(self.tcx, def_id)
                    && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Call(_))
            }
            false => {
                self.krate.fns.contains_key(&def_id)
                    && self.tcx.generics_of(def_id).count() == 0
                    && (0..args.len()).all(|i| !self.param_is_box(def_id, i))
            }
        };
        if let Operand::Copy(_) | Operand::Move(_) = func
            && called_as_js
        {
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
        }
        // `js::import!(f)`: what `f` names imported, not its value.
        if bindings::is_binding(self.tcx, def_id)
            && let bindings::JsForm::Import { module } = bindings::js_form(self.tcx, def_id)
        {
            let Some((item, _)) = args.first().and_then(|a| fn_def(a.node.ty(decls, self.tcx))) else {
                return Err(self.unsupported(span, "importing what isn't a function"));
            };
            return Ok(Value::Expr(self.dynamic_import_of(item, module, span)?));
        }
        let arg_tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
        let output = destination.ty(decls, self.tcx).ty;
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let known = self.recognition().classify(def_id, generic_args);
        // A read of items that may be `None` whose `Option` only `.flatten()`
        // reads: the JS read itself, which `flatten` gives as it is (ADR 0311).
        if let Some(known) = known
            && super::std_types::vec::reads_flattened(known)
            && self.flattened_by(state, destination)
        {
            let values = self.take_operands(state, &operands, out)?;
            let values = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            state.flattened.insert(destination.local);
            return Ok(Value::Expr(super::std_types::vec::flattened_read_values(known, values)));
        }
        // `flatten()` of it, and a copy that's the item itself: the read.
        if matches!(known, Some(Std::OptionFlatten | Std::OptionCloned))
            && let [Operand::Move(read)] = operands[..]
            && read.projection.is_empty()
            && state.flattened.contains(&read.local)
        {
            state.flattened.insert(destination.local);
            return Ok(self.take_operands(state, &operands, out)?.remove(0));
        }
        let values = self.take_operands(state, &operands, out)?;
        self.mir_call_values(state, (def_id, generic_args), (values, &arg_tys), output, span, out)
    }

    /// The locals each struct JSX gives a component as its props is made in,
    /// `<*>`'s second, and those of their flattened fields, made there.
    fn mir_jsx_props(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let decls = &body.local_decls;
        let mut made: Vec<Local> = (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call { func, args, .. } => {
                    let (def_id, _) = fn_def(func.ty(decls, self.tcx))?;
                    let component = bindings::is_binding(self.tcx, def_id)
                        && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Jsx(tag) if tag == "*");
                    match (component, &args[..]) {
                        (true, [_, given]) => given.node.place().filter(|p| p.projection.is_empty()).map(|p| p.local),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();
        let mut props = std::collections::HashSet::new();
        while let Some(local) = made.pop() {
            if !props.insert(local) {
                continue;
            }
            for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
                if let StatementKind::Assign(assign) = &statement.kind
                    && assign.0 == Place::from(local)
                    && let Rvalue::Aggregate(kind, operands) = &assign.1
                    && let AggregateKind::Adt(..) = **kind
                {
                    let ty = assign.0.ty(decls, self.tcx).ty;
                    for (i, operand) in operands.iter_enumerated() {
                        if bindings::is_flatten_field(self.tcx, ty, i.as_usize())
                            && let Some(field) = operand.place().filter(|p| p.projection.is_empty())
                        {
                            made.push(field.local);
                        }
                    }
                }
            }
        }
        props
    }

    /// Whether what a call makes, `destination`, is read only by `.flatten()`,
    /// or by a copy that's the item itself, `.copied()` of a reference, a
    /// number or text, which only `.flatten()` reads.
    fn flattened_by(&self, state: &State<'_, 'tcx>, destination: Place<'tcx>) -> bool {
        let decls = &state.body.local_decls;
        if !destination.projection.is_empty() || state.locals.reads[destination.local] != 1 {
            return false;
        }
        state.body.basic_blocks.iter().any(|data| {
            let TerminatorKind::Call {
                func,
                args,
                destination: made,
                ..
            } = &data.terminator().kind
            else {
                return false;
            };
            let reads = matches!(&args[..], [arg]
                if matches!(&arg.node, Operand::Move(read) if *read == Place::from(destination.local)));
            let known = fn_def(func.ty(decls, self.tcx))
                .and_then(|(def_id, generic_args)| self.recognition().classify(def_id, generic_args));
            let item_itself = || {
                self.option_of(made.ty(decls, self.tcx).ty)
                    .and_then(|inner| self.option_of(inner))
                    .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item))
            };
            reads
                && match known {
                    Some(Std::OptionFlatten) => true,
                    Some(Std::OptionCloned) => item_itself() && self.flattened_by(state, *made),
                    _ => false,
                }
        })
    }

    /// The base of a struct update, THIR's at `span`, which THIR spreads
    /// whole, `..*item` of a shared reference or `..hit` of a variable (ADR
    /// 0250): its place, the variable's by its debug info. MIR reads a field
    /// of it at most, `Props { title, ..props }`, or none, every field given,
    /// `Hit { url, ..*item }`, which MIR alone can't tell from `level: a.level`.
    fn written_base(
        &self,
        state: &State<'_, 'tcx>,
        ty: Ty<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        span: Span,
    ) -> Option<Place<'tcx>> {
        let strip = |e| super::body_queries::strip(self.thir, e);
        let base = (self.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Adt(adt) if e.span == span => match &adt.base {
                thir::AdtExprBase::Base(fru) => Some(strip(fru.base)),
                _ => None,
            },
            _ => None,
        })?;
        let (var, deref) = match self.thir[base].kind {
            thir::ExprKind::Deref { arg } if matches!(self.thir[arg].ty.kind(), ty::Ref(_, _, ty::Mutability::Not)) => {
                (strip(arg), true)
            }
            thir::ExprKind::VarRef { .. } | thir::ExprKind::UpvarRef { .. }
                if !bindings::has_flatten(self.tcx, ty) && (!self.is_copy(ty) || !self.contains_mutated(ty)) =>
            {
                (base, false)
            }
            _ => return None,
        };
        let hir_id = match self.thir[var].kind {
            thir::ExprKind::VarRef { id } => id.0,
            thir::ExprKind::UpvarRef { var_hir_id, .. } => var_hir_id.0,
            _ => return None,
        };
        let binding = self.tcx.hir_span(hir_id);
        let place = (state.body.var_debug_info.iter()).find_map(|info| match info.value {
            mir::VarDebugInfoContents::Place(place) if info.source_info.span == binding => Some(place),
            _ => None,
        })?;
        let base = match deref {
            true => place.project_deeper(&[PlaceElem::Deref], self.tcx),
            false => place,
        };
        // Each field read of it, the spread's, its own, as THIR's: one that
        // needs no copy of its own.
        let Shape::Object(fields) = self.shape(ty) else {
            return None;
        };
        for (i, operand) in operands.iter_enumerated() {
            let Some(read) = operand.place().filter(|p| p.local == base.local) else {
                continue;
            };
            let field = matches!(read.projection.split_last(), Some((PlaceElem::Field(f, _), rest))
                if *f == i && rest == base.projection.as_slice());
            let t = fields[i.as_usize()].1;
            if !field || self.contains_mutated(t) && self.is_copy(t) {
                return None;
            }
        }
        Some(base)
    }

    /// A JSX binding's arguments, each with what THIR has of it, where THIR
    /// has the call too, for its shape. `component` says a component's
    /// tag, where it's one, in place of its value.
    fn jsx_args(
        &mut self,
        def_id: rustc_span::def_id::DefId,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        component: impl Fn(&Self, usize, Ty<'tcx>) -> Option<Expr>,
    ) -> R<Vec<JsxArg<'tcx>>> {
        let exprs = self.thir_call_args(def_id, span);
        let mut given = Vec::new();
        for (i, (value, &ty)) in values.into_iter().zip(arg_tys).enumerate() {
            let value = match component(self, i, ty) {
                Some(tag) => tag,
                None => self.value_expr(value, span)?,
            };
            let expr = exprs.as_ref().and_then(|exprs| exprs.get(i).copied());
            given.push(JsxArg {
                value,
                ty,
                expr,
                span: expr.map_or(span, |e| self.thir[e].span),
            });
        }
        Ok(given)
    }

    /// The arguments of THIR's call of `def_id` at `fn_span`, MIR's call
    /// there, where it's the one.
    fn thir_call_args(&self, def_id: rustc_span::def_id::DefId, fn_span: Span) -> Option<Vec<thir::ExprId>> {
        self.thir_call(def_id, fn_span).map(|(_, args)| args)
    }

    /// Whether THIR's call of `def_id` at `span`, MIR's there, gives a
    /// `&mut` to a variable as its `i`th argument, `&mut current`, which a
    /// closure may have captured: a place JS assigns.
    fn borrows_variable(&self, def_id: rustc_span::def_id::DefId, span: Span, i: usize) -> bool {
        self.thir_call_args(def_id, span)
            .and_then(|args| self.mut_borrowed(*args.get(i)?))
            .is_some_and(|place| {
                matches!(
                    self.thir[self.strip(place)].kind,
                    thir::ExprKind::VarRef { .. } | thir::ExprKind::UpvarRef { .. }
                )
            })
    }

    /// THIR's update whose base is the call at `span`, MIR's there, of a
    /// derived `Default`, `P { a, ..Default::default() }`: the base's fields
    /// as THIR makes them, each default the update reads. `None` of another, or
    /// where a default the update replaces is made still, for its effects.
    fn default_base_at(&mut self, span: Span) -> R<Option<Vec<(String, Expr)>>> {
        let strip = |e| super::body_queries::strip(self.thir, e);
        let found = (self.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Adt(adt) => match &adt.base {
                thir::AdtExprBase::Base(fru)
                    if matches!(self.thir[strip(fru.base)].kind,
                        thir::ExprKind::Call { fn_span, .. } if fn_span.source_equal(span)) =>
                {
                    Some((
                        strip(fru.base),
                        e.ty,
                        adt.fields.iter().map(|f| f.name.as_usize()).collect::<Vec<_>>(),
                    ))
                }
                _ => None,
            },
            _ => None,
        });
        let Some((base, ty, given)) = found else {
            return Ok(None);
        };
        let Some(fields) = self.derived_default_fields(base, ty) else {
            return Ok(None);
        };
        let (props, kept) = self.update_default(fields, &given, span)?;
        Ok((!kept).then_some(props))
    }

    /// Whether THIR's cast at `span`, MIR's there, is of a float it knows
    /// whole and in the target's range: the value as it is, as THIR writes
    /// it (`whole_casts`).
    fn whole_cast_at(&self, span: Span) -> bool {
        let mut casts = (self.thir.exprs.iter_enumerated())
            .filter(|(_, e)| matches!(e.kind, thir::ExprKind::Cast { .. }) && e.span.source_equal(span));
        matches!((casts.next(), casts.next()), (Some((id, _)), None) if self.body_facts.whole_casts.contains(&id))
    }

    /// THIR's call of `def_id` at `fn_span`, MIR's call there, where it's
    /// the one: its function and its arguments.
    fn thir_call(&self, def_id: rustc_span::def_id::DefId, fn_span: Span) -> Option<(thir::ExprId, Vec<thir::ExprId>)> {
        let mut calls = self.thir.exprs.iter().filter_map(|e| match &e.kind {
            thir::ExprKind::Call {
                fun, args, fn_span: at, ..
            } if *at == fn_span && matches!(*self.thir[*fun].ty.kind(), ty::FnDef(id, _) if id == def_id) => {
                Some((*fun, args.to_vec()))
            }
            _ => None,
        });
        let call = calls.next()?;
        calls.next().is_none().then_some(call)
    }

    /// `.await` of what's given, `into_future(x)`'s (`awaits.rs`).
    fn mir_await(
        &mut self,
        state: &mut State<'_, 'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let operands: Vec<&Operand<'tcx>> = args.iter().map(|a| &a.node).collect();
        let awaited = self.take_operands(state, &operands, out)?.remove(0);
        Ok(Value::Expr(Expr::await_(self.value_expr(awaited, span)?)))
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
            // Named by its type, as THIR's: `(s) => ..` of a string.
            1 => vec![self.fresh(self.parameter_name(inputs[0].peel_refs()))],
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
            && !tcx.is_lang_item(def_id, LangItem::TryTraitBranch)
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
        // Calling a closure, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`, and an
        // `async` one's, `AsyncFn::async_call`, which gives its promise.
        if let Some(fn_trait) = tcx.trait_of_assoc(def_id)
            && (tcx.fn_trait_kind_from_def_id(fn_trait).is_some()
                || tcx.async_fn_trait_kind_from_def_id(fn_trait).is_some())
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
            if !matches!(
                arg_tys[0].peel_refs().kind(),
                ty::Closure(..) | ty::CoroutineClosure(..)
            ) {
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
        // Not one that's a binding, a default method's `get ruleId` called
        // through a generic's bound: the binding, below, as THIR's.
        if !matches!(known, Some(Std::Any(_)))
            && !bindings::is_binding(tcx, def_id)
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
        // A `From` into an untagged enum, or `into()` to one: the value (ADR 0214).
        if values.len() == 1 && self.recognition().converts_to_untagged(def_id, generic_args) {
            return Ok(values.into_iter().next().expect("one"));
        }
        // A prop `jsx!` isn't given: none, which JSX leaves out (ADR 0213).
        if bindings::is_omitted(tcx, def_id) {
            return Ok(Value::Expr(Expr::undefined()));
        }
        // A binding: the JS function, method, property or operator it names
        // (ADRs 0019, 0020), given its arguments as THIR gives them.
        if bindings::is_binding(tcx, def_id) {
            if rustc_hir::find_attr!(tcx, def_id, RustcEiiForeignItem) {
                return Err(self.unsupported(span, "externally implementable items, `#[eii]`,"));
            }
            match bindings::js_form(tcx, def_id) {
                bindings::JsForm::Jsx(tag) => {
                    let given = self.jsx_args(def_id, values, &arg_tys, span, |cx, i, ty| match (tag.as_str(), i) {
                        // A JS module's component is its import, as a tag must be.
                        ("*", 0) => match *ty.peel_refs().kind() {
                            ty::FnDef(id, _) => cx.binding_component_of(id),
                            _ => None,
                        },
                        _ => None,
                    })?;
                    // Where it's written, as THIR's element is: its tag's line.
                    let element = self.jsx_values(&tag, given, span, out)?;
                    return Ok(Value::Expr(element.or_at(self.js_span(span))));
                }
                bindings::JsForm::Prop(name) => {
                    let mut given = self
                        .jsx_args(def_id, values, &arg_tys, span, |_, _, _| None)?
                        .into_iter();
                    let element = given.next().expect("the element");
                    let name = match name {
                        Some(name) => name,
                        None => match given.next().map(|name| name.value.kind) {
                            Some(js::ExprKind::Str(name)) => name,
                            _ => return Err(self.unsupported(span, "an attribute's name other than a string literal")),
                        },
                    };
                    let value = given.next().expect("the value");
                    let mut element = element.value;
                    // `style.color("red")` on an object being built: one more field.
                    if let js::ExprKind::Object(fields) = &mut element.kind {
                        fields.push(match name.as_str() {
                            "..." => Prop::Spread(value.value),
                            _ => Prop::Field(name, value.value),
                        });
                        return Ok(Value::Expr(element));
                    }
                    if !matches!(element.kind, js::ExprKind::Jsx(_)) {
                        return Err(self.one_expression(span));
                    }
                    self.jsx_spread_checked(&name, value.ty, value.span)?;
                    let made = self.jsx_prop_values(name, element, value, (Vec::new(), true), out)?;
                    return Ok(Value::Expr(made));
                }
                bindings::JsForm::Object(keys) => {
                    if keys.len() != values.len() {
                        return Err(self.unsupported(span, "an object binding whose fields don't match its arguments"));
                    }
                    let mut fields = Vec::new();
                    for (key, value) in keys.into_iter().zip(values) {
                        fields.push(Prop::Field(key, self.value_expr(value, span)?));
                    }
                    return Ok(Value::Expr(Expr::object(fields)));
                }
                bindings::JsForm::Import { .. } => {
                    return Err(self.unsupported(span, "a dynamic import, from its MIR"));
                }
                _ => {}
            }
            let nullable = bindings::nullable_params(tcx, def_id);
            let idents = tcx.fn_arg_idents(def_id);
            let mut exprs = Vec::new();
            for (i, (value, &ty)) in values.into_iter().zip(&arg_tys).enumerate() {
                let value = self.value_expr(value, span)?;
                exprs.push(match () {
                    // `()` given to JS is what a tuple is, an array (ADR 0020).
                    () if ty.is_unit() => Expr::array(Vec::new()),
                    // A parameter it names `#[rust_js::nullable(..)]`: its
                    // `None` is `null` (ADR 0275).
                    () if idents
                        .get(i)
                        .copied()
                        .flatten()
                        .is_some_and(|ident| nullable.contains(&ident.name)) =>
                    {
                        match value.kind {
                            js::ExprKind::Undefined => Expr::null(),
                            _ if value.is_constant() => value,
                            _ => Expr::bin(Op::Coalesce, value, Expr::null()),
                        }
                    }
                    // A `&dyn Any` given to JS is its value (ADR 0331).
                    () => self.any_given_to_js(value, ty),
                });
            }
            let js_span = self.js_span(span);
            return Ok(Value::Expr(self.binding_values(
                (def_id, generic_args),
                exprs,
                js_span,
                span,
                out,
            )?));
        }
        // A function giving each variant its own name gives what it's given,
        // `section.as_str()` is `section` (ADR 0264).
        if values.len() == 1 && self.gives_own_name(def_id) {
            return Ok(values.into_iter().next().expect("one"));
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
            // shows, an escape's; nor a `T` it gives, `unwrap_or`'s, its value.
            return Ok(match value {
                Value::Expr(e)
                    if self.implements_iterator(output)
                        && !matches!(output.kind(), ty::Param(_))
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
        (known, def_id): (Std, rustc_span::def_id::DefId),
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let pointee = arg_tys[0].peel_refs();
        let js_span = self.js_span(span);
        let values = values.into_iter();
        self.flush(state, out)?;
        // Of a `&mut` to a variable, `&mut current`, an object's too: the
        // variable, given its new value, as THIR's; a swap's, of two. One
        // of this body's own, which holds the value itself: a closure's
        // capture may be a name for a `&mut` to it, which assigning would
        // only rebind.
        let borrowed = if known == Std::Swap { 2 } else { 1 };
        let variables = (0..borrowed).all(|i| self.borrows_variable(def_id, span, i));
        let own = |e: &Expr| {
            matches!(&e.kind, js::ExprKind::Var(name) if (state.locals.names.iter_enumerated())
                .any(|(local, n)| n == name && !state.body.local_decls[local].ty.is_ref()))
                || (state.captures.iter()).any(|(capture, place)| {
                    *place && matches!((&capture.kind, &e.kind), (js::ExprKind::Var(a), js::ExprKind::Var(b)) if a == b)
                })
        };
        let mut values = values.enumerate().map(|(i, value)| match value {
            Value::Expr(e) | Value::Place(e) if i < borrowed && variables && own(&e) => Value::Ref(e),
            value => value,
        });
        let first = values.next().expect("a `&mut`");
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
        // `v[i]` THIR knows is in bounds, `v[0]` after `v.is_empty()`'s test
        // fails, or of an array long enough: unchecked, as THIR writes it.
        if known == Std::Index
            && let Some((fun, args)) = self.thir_call(def_id, span)
            && let [items_id, _] = args[..]
            && let [Value::Expr(_) | Value::Place(_), Value::Expr(index)] = &values[..]
            && (self.body_facts.in_bounds.contains(&fun) || self.in_bounds(items_id, index))
        {
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let index = exprs.pop().expect("an index");
            let items = exprs.pop().expect("the items");
            return Ok(Value::Expr(Expr::index(items, index)));
        }
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
            // A variable that reads the same wherever it's read, a parameter,
            // read where it's set too.
            let index = if index.is_constant()
                || matches!(&index.kind, js::ExprKind::Var(v) if self.locals.alike.contains(v))
            {
                index
            } else {
                self.spill("index", index, out)
            };
            self.runtime.insert(Helper::Index);
            // And `$at`, the place tidy writes where it's set next.
            self.runtime.insert(Helper::At);
            let check = Expr::call(Expr::var("$index"), vec![items.clone(), index.clone()]);
            out.push(StmtKind::Expr(check).at(self.js_span(span)));
            return Ok(Value::Ref(Expr::index(items, index)));
        }
        // `m.entry(k).or_insert(v)`: the value put in if the key's missing,
        // THIR's `$orInsert`, which is the value, an object; of a number,
        // `*m.entry(k).or_insert(0) += 1`, a handle on the key's value,
        // `$mutGet`'s (ADR 0152). The entry is `$entry(m, k)`, `{ TAG, _0: [m, k] }`.
        if let Std::Map(op @ (MapOp::OrInsert | MapOp::OrInsertWith | MapOp::OrDefault)) = known {
            let mut exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?
                .into_iter();
            let entry = exprs.next().expect("the entry");
            let (map, key) = match entry.kind {
                js::ExprKind::Call(callee, parts)
                    if parts.len() == 2 && matches!(&callee.kind, js::ExprKind::Var(v) if v == "$entry") =>
                {
                    let [map, key]: [Expr; 2] = parts.try_into().ok().expect("two");
                    (map, key)
                }
                kind => {
                    let entry = self.spill(
                        "entry",
                        Expr {
                            reads: false,
                            kind,
                            span: entry.span,
                        },
                        out,
                    );
                    let parts = Expr::member(entry, "_0");
                    (
                        Expr::index(parts.clone(), Expr::int(0)),
                        Expr::index(parts, Expr::int(1)),
                    )
                }
            };
            let map = if map.reads_same() {
                map
            } else {
                self.spill("map", map, out)
            };
            let key = if key.reads_same() {
                key
            } else {
                self.spill("key", key, out)
            };
            let default = match op {
                MapOp::OrDefault => {
                    let value = generic_args
                        .types()
                        .nth(1)
                        .ok_or_else(|| self.unsupported(span, "this entry"))?;
                    let value = self.default_value(value, span)?;
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)])
                }
                _ => exprs.next().expect("the value"),
            };
            let (helper, name) = match op {
                MapOp::OrInsert => (Helper::OrInsert, "$orInsert"),
                _ => (Helper::OrInsertWith, "$orInsertWith"),
            };
            self.runtime.insert(helper);
            let inserted = Expr::call(Expr::var(name), vec![map.clone(), key.clone(), default]);
            if !self.is_cell(output) {
                return Ok(Value::Expr(inserted));
            }
            self.flush(state, out)?;
            out.push(StmtKind::Expr(inserted).at(self.js_span(span)));
            self.runtime.insert(Helper::MutGet);
            return Ok(Value::Expr(Expr::call(Expr::var("$mutGet"), vec![map, key])));
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
                    true => self.std_iterator(handles, output),
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
                let value = match values.next().expect("an argument") {
                    // A `&dyn Debug`, shown as what it's made of is.
                    Value::Fmt(Std::FmtDebug, made, value) if known == Std::FmtDebug => {
                        return Ok(Value::Fmt(known, made, value));
                    }
                    value => self.value_expr(value, span)?,
                };
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
                // Written across lines, its line breaks as they are (ADR 0066).
                let lines = bytes.contains(&b'\n') && self.format_string_across_lines(span);
                let joined = super::display::join(parts).with_lines(lines);
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
            // A B-tree's range of its keys, THIR's (ADR 0325).
            Std::Map(MapOp::TreeRange { mutable: false }) => {
                let [m, range]: [Value<'tcx>; 2] =
                    values.collect::<Vec<_>>().try_into().ok().expect("a tree and a range");
                let (m, range) = (self.value_expr(m, span)?, self.value_expr(range, span)?);
                self.tree_range_values((m, arg_tys[0]), (range, arg_tys[1]), false, span, out)?
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
                self.map_values(op, (exprs, arg_tys), generic_args, state.discarding, span, out)?
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
                return self.mir_swap_or_replace(state, (known, def_id), values, arg_tys, span, out);
            }
            // `v.sort_unstable_by(f)` and a selection, THIR's (ADR 0342).
            Std::Slice(op @ (SliceOp::SortUnstable { with } | SliceOp::SelectNth { with })) => {
                let values = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.unstable_sort_values(op, with, values, generic_args, span, out)?
            }
            // The rest of a slice's own that their operands say all of, THIR's:
            // not one giving a `&mut`, a view, nor `remainder`, which asks
            // whether its iterator is stepped.
            Std::Slice(op)
                if !matches!(
                    op,
                    SliceOp::Remainder
                        | SliceOp::View { .. }
                        | SliceOp::DequeRangeMut
                        | SliceOp::ExtractIf { .. }
                        | SliceOp::SplitOff { .. }
                        | SliceOp::UninitWrite { .. }
                        | SliceOp::UninitDrop
                        | SliceOp::CopyWithin
                ) && !output.walk().any(|arg| {
                    arg.as_type()
                        .is_some_and(|t| matches!(t.kind(), ty::Ref(_, _, ty::Mutability::Mut)))
                }) =>
            {
                let values = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                self.slice_values(op, values, false, generic_args, span, out)?
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
            // `Rc::make_mut(&mut rc)` of a local `Rc` of this body: the local
            // given the one `$makeMut` gives. Through a reference the place
            // isn't here, which a variable given another would leave.
            Std::Rc(RcOp::MakeMut) => {
                let place = self.value_expr(values.next().expect("the `Rc`"), span)?;
                let item = self.counted_rc(arg_tys[0].peel_refs());
                let local = match &place.kind {
                    js::ExprKind::Var(name) => state.locals.names.iter_enumerated().find(|(_, n)| *n == name),
                    _ => None,
                };
                let (Some(item), Some((local, _))) = (item, local) else {
                    return Err(self.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                };
                if state.body.local_kind(local) == LocalKind::Arg || state.body.local_decls[local].ty.is_ref() {
                    return Err(self.unsupported(span, "`Rc::make_mut` of an `Rc` that isn't a place here"));
                }
                self.counted_here(item, span)?;
                self.runtime.insert(Helper::Rc);
                self.flush(state, out)?;
                self.make_mut_values(place, item, span, out)?
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
            // A `Peekable`'s or a slice's or string's iterator's own: of a `$iter`, which
            // knows where it is (ADR 0071), as THIR's `step_call` of one.
            Std::Step(op @ (StepOp::Peek | StepOp::NextIf | StepOp::NextIfEq | StepOp::AsSlice | StepOp::AsStr))
                if op == StepOp::AsStr
                    || arg_tys.first().is_some_and(|t| {
                        self.array_source(t.peel_refs()).is_some() || self.is_peekable(t.peel_refs())
                    }) =>
            {
                let mut exprs = values
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?
                    .into_iter();
                let it = exprs.next().expect("the iterator");
                let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
                match op {
                    StepOp::Peek if boxed => self.helper(Helper::PeekSome, "$peekSome", vec![it]),
                    // A string's `chars()`' rest, as a string.
                    StepOp::AsStr => self.helper(Helper::RestStr, "$restStr", vec![it]),
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
            // `vec![item; n]`: each a clone, as THIR's of what it can't remake.
            Std::FromElem => {
                let item = self.value_expr(values.next().expect("the item"), span)?;
                let n = self.value_expr(values.next().expect("the count"), span)?;
                self.copies_values((item, arg_tys[0]), n, false, span, out)?
            }
            Std::NonZeroNew => {
                let n = self.value_expr(values.next().expect("the number"), span)?;
                self.non_zero_new(n, arg_tys[0], span, out)?
            }
            // `s.insert(i, c)` and the rest: `s` given its new text, as THIR's
            // `string_edit` writes it; what it took out, its value.
            Std::StringEdit(edit) => {
                let place = match values.next().expect("the string") {
                    Value::Ref(place) if fixed_place(&place) => place,
                    _ => return Err(self.unsupported(span, "changing this string, from its MIR")),
                };
                let mut given = values.map(|v| self.value_expr(v, span)).collect::<R<Vec<_>>>()?;
                if matches!(
                    edit,
                    StringEdit::Drain | StringEdit::ReplaceRange | StringEdit::ExtendFromWithin
                ) {
                    let range = given.remove(0);
                    let (start, end) = self.range_value_bounds(range, arg_tys[1], span, out)?;
                    given.splice(0..0, [start, end.unwrap_or_else(Expr::undefined)]);
                }
                self.flush(state, out)?;
                let (edited, taken) = self.string_edit_values(edit, place.clone(), given, out);
                out.push(StmtKind::Assign(place, edited).at(self.js_span(span)));
                taken.unwrap_or_else(Expr::undefined)
            }
            // rustc's name for the type; of a type parameter, the one its
            // caller gave (ADR 0145). What `type_name_of_val` is given runs.
            Std::TypeName { .. } => {
                for value in values {
                    let value = self.value_expr(value, span)?;
                    if value.has_effects() {
                        self.flush(state, out)?;
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                let ty = generic_args.type_at(0);
                self.type_fact_value(ty, super::recognition::TypeFact::Name, span)?
            }
            // Its capacity is the engine's: what it's given runs, for what it does.
            Std::StringWithCapacity => {
                for value in values {
                    let value = self.value_expr(value, span)?;
                    if value.has_effects() {
                        self.flush(state, out)?;
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                Expr::str("")
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Where each `let … else { continue; }` the program wrote is, which MIR
    /// tests as an `if` of the `continue` in its place.
    fn let_else_continues(&self) -> Vec<js::Span> {
        (self.thir.stmts.iter())
            .filter_map(|stmt| match stmt.kind {
                thir::StmtKind::Let {
                    else_block: Some(block),
                    span,
                    ..
                } => {
                    let block = &self.thir[block];
                    let mut only = match (block.stmts.last(), block.expr) {
                        (_, Some(e)) => e,
                        (Some(s), None) => match self.thir[*s].kind {
                            thir::StmtKind::Expr { expr, .. } => expr,
                            _ => return None,
                        },
                        _ => return None,
                    };
                    while let thir::ExprKind::Scope { value: inner, .. }
                    | thir::ExprKind::NeverToAny { source: inner } = self.thir[only].kind
                    {
                        only = inner;
                    }
                    matches!(self.thir[only].kind, thir::ExprKind::Continue { .. }).then(|| self.js_span(span))
                }
                _ => None,
            })
            .collect()
    }
}

/// Whether `local` is assigned once each way through `body`, as a `match`'s
/// arms each assign what it gives: no assignment goes on to another, nor to
/// itself again, as one in a loop would.
fn once_each_way(body: &mir::Body<'_>, local: Local) -> bool {
    let mut at: Vec<BasicBlock> = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                let (place, rvalue) = &**assign;
                if place.local == local && place.projection.is_empty() {
                    at.push(block);
                }
                // Borrowed mutably whole, it may be assigned through the borrow.
                if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue
                    && borrowed.local == local
                {
                    return false;
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &data.terminator().kind
            && destination.local == local
            && destination.projection.is_empty()
        {
            at.push(block);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (i, &block) in at.iter().enumerate() {
        if at[i + 1..].contains(&block) {
            return false;
        }
        seen.clear();
        let mut next: Vec<BasicBlock> = body.basic_blocks[block].terminator().successors().collect();
        while let Some(b) = next.pop() {
            if !seen.insert(b) {
                continue;
            }
            if at.contains(&b) {
                return false;
            }
            next.extend(body.basic_blocks[b].terminator().successors());
        }
    }
    true
}

/// The body of the async arrow `stmts` only call and return, `return (async
/// () => { .. })()`, an `async` body made: the async function's own.
fn called_async_body(stmts: &mut [Stmt]) -> Option<Vec<Stmt>> {
    let [
        Stmt {
            kind: StmtKind::Return(Some(called)),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    let js::ExprKind::Call(callee, args) = &mut called.kind else {
        return None;
    };
    let js::ExprKind::AsyncArrow(params, body) = &mut callee.kind else {
        return None;
    };
    (args.is_empty() && params.is_empty()).then(|| std::mem::take(body))
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

/// The variable the program binds what `?`'s `branch`, `tried`, holds to
/// where it goes on, `f`, of `val = copy ((tried as Continue).0); f = copy
/// val;`, given nothing else, and the local its binding reads.
fn continued_binding(state: &State<'_, '_>, tried: Local) -> Option<(Local, Local)> {
    // Each variable given just `from`, which holds what's held.
    let given = |from: &dyn Fn(&Place<'_>) -> bool| {
        let mut found = None;
        for data in state.body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let (place, Rvalue::Use(Operand::Copy(read) | Operand::Move(read), ..)) = &**assign
                    && from(read)
                    && place.projection.is_empty()
                    && state.locals.writes[place.local] == 1
                    && found.replace(place.local).is_some()
                {
                    return None;
                }
            }
        }
        found
    };
    let val = given(&|read| {
        read.local == tried
            && matches!(read.projection.as_slice(), [PlaceElem::Downcast(_, variant), PlaceElem::Field(field, _)]
                if variant.as_u32() == 0 && field.as_u32() == 0)
    })?;
    // And what it's read from, `tried` or `val`, where it's bound.
    match state.locals.user[val] {
        true => Some((val, tried)),
        false => given(&|read| read.local == val && read.projection.is_empty())
            .filter(|&l| state.locals.user[l])
            .map(|l| (l, val)),
    }
}

/// Labels, by block.
fn label(block: BasicBlock) -> String {
    format!("bb{}", block.as_usize())
}

/// Whether `block` does nothing but say it's never reached.
/// Of a bounds check `msg`, the index whose element `target` reads or
/// writes first, and whether it writes it: there, `$index(xs, i)` or
/// `xs[$at(xs, i)]`, the check, as THIR makes it.
fn deferred_check<'tcx>(
    body: &mir::Body<'tcx>,
    msg: &mir::AssertMessage<'tcx>,
    target: BasicBlock,
) -> Option<(Local, bool)> {
    let mir::AssertKind::BoundsCheck {
        index: Operand::Copy(index) | Operand::Move(index),
        ..
    } = msg
    else {
        return None;
    };
    let local = index.as_local()?;
    let first = body.basic_blocks[target].statements.iter().find(|s| {
        !matches!(
            s.kind,
            StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop
        )
    })?;
    let StatementKind::Assign(assign) = &first.kind else {
        return None;
    };
    let (written, rvalue) = &**assign;
    let indexes = |p: &Place<'tcx>| p.projection.contains(&PlaceElem::Index(local));
    match rvalue {
        Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _)
        | Rvalue::CopyForDeref(read)
        | Rvalue::Ref(_, BorrowKind::Shared, read)
            if indexes(read) && !indexes(written) =>
        {
            Some((local, false))
        }
        _ if written.projection.last() == Some(&PlaceElem::Index(local)) => Some((local, true)),
        _ => None,
    }
}

/// A module's own `const X = { value: init }`, a thread-local's cell MIR
/// reads by its `value`, that its module reads and sets only so, by name,
/// and doesn't export: `let X = init`, `X` where it's read, `const` where
/// nothing sets it, as THIR keeps such a thread-local (ADR 0270).
pub(super) fn plain_statics(items: &mut [js::Item]) {
    let cells: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c)
                if !c.export
                    && matches!(&c.value.kind, js::ExprKind::Object(props)
                        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == "value")) =>
            {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    for name in cells {
        let (mut reads, mut values, mut sets) = (0, 0, 0);
        js::each_item_expr_mut(items, &mut |e| match &e.kind {
            js::ExprKind::Var(v) if *v == name => reads += 1,
            js::ExprKind::Member(of, key)
                if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name) =>
            {
                values += 1;
            }
            _ => {}
        });
        // A function's own of that name, or one it binds, may be read so.
        let mut bound = false;
        for item in items.iter_mut() {
            for function in item.functions_mut() {
                bound |= function.params.iter().any(|p| p.names().contains(&name.as_str()));
                js::each_block_mut(&mut function.body, &mut |stmts| {
                    for stmt in stmts.iter() {
                        bound |= match &stmt.kind {
                            StmtKind::Const(n, _) | StmtKind::Let(n, _) => *n == name,
                            StmtKind::Destructure { pattern, .. } => pattern.names().contains(&name.as_str()),
                            _ => false,
                        };
                    }
                });
                js::each_expr_mut(&mut function.body, &mut |e| {
                    if let js::ExprKind::Arrow(params, _) | js::ExprKind::AsyncArrow(params, _) = &e.kind {
                        bound |= params.iter().any(|p| p.names().contains(&name.as_str()));
                    }
                });
            }
        }
        if bound || reads != values {
            continue;
        }
        for item in items.iter_mut() {
            for function in item.functions_mut() {
                js::each_block_mut(&mut function.body, &mut |stmts| {
                    for stmt in stmts.iter() {
                        if let StmtKind::Assign(target, _) = &stmt.kind
                            && matches!(&target.kind, js::ExprKind::Member(of, key)
                                if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
                        {
                            sets += 1;
                        }
                    }
                });
            }
        }
        // A handle on it, lent, may set it too.
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Handle(place) = &e.kind
                && matches!(&place.kind, js::ExprKind::Member(of, key)
                    if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
            {
                sets += 1;
            }
        });
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Member(of, key) = &e.kind
                && key == "value"
                && matches!(&of.kind, js::ExprKind::Var(v) if *v == name)
            {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&name)
                };
            }
        });
        for item in items.iter_mut() {
            if let js::Item::Const(c) = item
                && c.name == name
                && let js::ExprKind::Object(props) = &mut c.value.kind
                && let Some(js::Prop::Field(_, init)) = props.pop()
            {
                c.value = init;
                c.mutable = sets > 0;
            }
        }
    }
}

/// Whether `local` is assigned in `from`, or a block it goes on to.
fn written_from(body: &mir::Body<'_>, from: BasicBlock, local: Local) -> bool {
    let mut seen = std::collections::HashSet::new();
    let mut next = vec![from];
    while let Some(block) = next.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body.basic_blocks[block];
        let writes = (data.statements.iter())
            .any(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.as_local() == Some(local)))
            || matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if destination.as_local() == Some(local));
        if writes {
            return true;
        }
        next.extend(data.terminator().successors());
    }
    false
}

/// Where the program wrote what `block` does first: a macro's, where it
/// called it.
fn written_at(body: &mir::Body<'_>, block: BasicBlock) -> Option<rustc_span::BytePos> {
    written_within(body, block, 4)
}

/// `written_at`'s, of a block that only tests further, a pattern's field,
/// `Done { ok: true }`'s: where the first of what it goes to is, its arms,
/// not the `match` its test is spanned by; `depth` such tests at most.
fn written_within(body: &mir::Body<'_>, block: BasicBlock, depth: usize) -> Option<rustc_span::BytePos> {
    let data = &body.basic_blocks[block];
    let first = (data.statements.iter()).find(|s| {
        !matches!(
            s.kind,
            StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Nop
                | StatementKind::PlaceMention(_)
        )
    });
    if first.is_none()
        && depth > 0
        && let TerminatorKind::SwitchInt { targets, .. } = &data.terminator().kind
    {
        return (targets.all_targets().iter())
            .filter(|&&b| !is_unreachable(body, b))
            .filter_map(|&b| written_within(body, b, depth - 1))
            .min();
    }
    let first = first
        .map_or(data.terminator().source_info.span, |s| s.source_info.span)
        .source_callsite();
    (!first.is_dummy()).then(|| first.lo())
}

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

/// A check of a `%` or `/`, which the division makes itself where it can
/// fail, `$rem(a, b, min)`, and can't where it's inline, `i % 2`: none, as
/// THIR writes none.
fn checked_by_division(msg: &mir::AssertMessage<'_>) -> bool {
    matches!(
        msg,
        AssertKind::DivisionByZero(_)
            | AssertKind::RemainderByZero(_)
            | AssertKind::Overflow(BinOp::Div | BinOp::Rem, ..)
    )
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
        if let [PlaceElem::Field(field, _)] = place.projection[..] {
            locals.fields[place.local].push(field.as_usize());
        }
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

fn count_rvalue(locals: &mut Locals, rvalue: &Rvalue<'_>, frozen: bool) {
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
            if let [PlaceElem::Field(field, _), PlaceElem::Deref] = place.projection[..]
                && matches!(kind, BorrowKind::Shared)
            {
                locals.fields[place.local].push(field.as_usize());
            }
            // The whole borrowed mutably may be written through the borrow: a
            // `let`, which a closure that captured it writes.
            if matches!(kind, BorrowKind::Mut { .. }) && place.projection.is_empty() {
                locals.writes[place.local] += 1;
            }
            if !place.projection.is_empty() && !place.is_indirect() && !frozen {
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

/// The variable a place is of, `route` of `route.title[0]`.
fn root_var(place: &Expr) -> Option<&str> {
    match &place.kind {
        js::ExprKind::Var(name) => Some(name),
        js::ExprKind::Member(object, _) | js::ExprKind::Index(object, _) => root_var(object),
        _ => None,
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
/// early exit: the same, as that branch never goes on. Of two that do, the
/// program's first. Of the branches,
/// `panics` says which panic: one that does is first, as a person writes a
/// check.
fn early_exit(
    test: Expr,
    then: Vec<Stmt>,
    otherwise: Option<Vec<Stmt>>,
    panics: [bool; 2],
    span: js::Span,
) -> Vec<Stmt> {
    let exits = |stmts: &[Stmt]| {
        matches!(
            stmts.last().map(|s| &s.kind),
            Some(StmtKind::Return(_) | StmtKind::Throw(_))
        )
    };
    match otherwise {
        Some(otherwise) if panics == [false, true] => {
            early_exit(negate(test), otherwise, Some(then), [true, false], span)
        }
        Some(otherwise) if panics == [true, false] || exits(&then) => {
            let mut out = vec![StmtKind::If(test, then, None).at(span)];
            out.extend(otherwise);
            out
        }
        Some(otherwise) if exits(&otherwise) => {
            let mut out = vec![StmtKind::If(negate(test), otherwise, None).at(span)];
            out.extend(then);
            out
        }
        otherwise => vec![StmtKind::If(test, then, otherwise).at(span)],
    }
}

/// Whether `block` goes on to a call that never returns, a panic's, through
/// blocks that only go on: the message it's given made on the way.
fn panics(body: &mir::Body<'_>, mut block: BasicBlock) -> bool {
    // A `loop {}` goes on forever.
    for _ in 0..body.basic_blocks.len() {
        match body.basic_blocks[block].terminator().kind {
            TerminatorKind::Call { target: None, .. } => return true,
            TerminatorKind::Goto { target }
            | TerminatorKind::Call {
                target: Some(target), ..
            }
            | TerminatorKind::Drop { target, .. } => block = target,
            _ => return false,
        }
    }
    false
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

/// Whether `local` is read only by its fields, each once, as an operand.
fn split(locals: &Locals, local: Local) -> bool {
    let fields = &locals.fields[local];
    let mut distinct = fields.clone();
    distinct.sort();
    distinct.dedup();
    !fields.is_empty() && fields.len() == locals.reads[local] && distinct.len() == fields.len()
}

/// The variant without fields a promoted body refers to, `_1 =
/// Option::<T>::None; _0 = &_1`, which generic code can't evaluate: its
/// type and variant.
fn referred_variant<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> Option<(Ty<'tcx>, VariantIdx)> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let mut made = None;
    let mut referred = None;
    for statement in &body.basic_blocks[mir::START_BLOCK].statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Aggregate(kind, operands)) if place.projection.is_empty() && operands.is_empty() => {
                    let AggregateKind::Adt(did, variant, args, _, _) = **kind else {
                        return None;
                    };
                    made = Some((place.local, Ty::new_adt(tcx, tcx.adt_def(did), args), variant));
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
    let (local, ty, variant) = made?;
    (Some(local) == referred).then_some((ty, variant))
}

/// The closure that captures nothing a promoted body refers to, `_1 =
/// {closure}; _0 = &_1`.
fn referred_closure(body: &mir::Body<'_>) -> Option<rustc_span::def_id::DefId> {
    if body.basic_blocks.len() != 1 {
        return None;
    }
    let mut made = None;
    let mut referred = None;
    for statement in &body.basic_blocks[mir::START_BLOCK].statements {
        match &statement.kind {
            StatementKind::Assign(assign) => match &**assign {
                (place, Rvalue::Aggregate(kind, operands)) if place.projection.is_empty() && operands.is_empty() => {
                    let AggregateKind::Closure(def_id, _) = **kind else {
                        return None;
                    };
                    made = Some((place.local, def_id));
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
    let (local, def_id) = made?;
    (Some(local) == referred).then_some(def_id)
}

/// The call of nothing a promoted body refers to the result of, `_1 =
/// size_of::<T>() -> bb1; bb1: _0 = &_1`: its function, its arguments and
/// what it gives.
fn referred_call<'tcx>(
    body: &mir::Body<'tcx>,
) -> Option<(rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>, Ty<'tcx>)> {
    if body.basic_blocks.len() != 2 {
        return None;
    }
    let (first, second) = (
        &body.basic_blocks[mir::START_BLOCK],
        &body.basic_blocks[BasicBlock::from_usize(1)],
    );
    let quiet = |data: &mir::BasicBlockData<'_>| {
        data.statements.iter().all(|s| {
            matches!(
                s.kind,
                StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop
            )
        })
    };
    let TerminatorKind::Call {
        func: Operand::Constant(func),
        args,
        destination,
        target: Some(target),
        ..
    } = &first.terminator().kind
    else {
        return None;
    };
    let (def_id, generic_args) = fn_def(func.ty())?;
    if !quiet(first) || !args.is_empty() || !destination.projection.is_empty() || target.as_usize() != 1 {
        return None;
    }
    let referred = second.statements.iter().find_map(|s| match &s.kind {
        StatementKind::Assign(assign) => match &**assign {
            (place, Rvalue::Ref(_, BorrowKind::Shared, target))
                if place.local == mir::RETURN_PLACE && target.projection.is_empty() =>
            {
                Some(target.local)
            }
            _ => None,
        },
        _ => None,
    })?;
    let only = second
        .statements
        .iter()
        .filter(|s| matches!(s.kind, StatementKind::Assign(_)))
        .count()
        == 1;
    let output = body.local_decls[destination.local].ty;
    (only && referred == destination.local && matches!(second.terminator().kind, TerminatorKind::Return)).then_some((
        def_id,
        generic_args,
        output,
    ))
}

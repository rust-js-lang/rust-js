//! What a body does with its values that have destructors that MIR's drop
//! elaboration can't settle, found before it's lowered (ADR 0098): what's
//! refused of it, and the borrows a guard holds only for a moment (ADR
//! 0328). A walk of the THIR that asks what types drop of a `DropQuery`,
//! and sees nothing else of the function.

use crate::collections::{IndexMap, IndexSet};

use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{
    AdtExprBase, Block, Expr as ThirExpr, ExprId, ExprKind, LocalVarId, Pat, PatKind, StmtId, StmtKind as ThirStmt,
    Thir,
};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::super::recognition::{HandleStep, Std, StdItem, handle_step, is_as_deref};
use super::super::{fn_def, strip};
use rustc_middle::middle::region;

use super::super::analysis::{Borrow, CellBorrows, resolve_runs_nothing};
use super::types::{DropQuery, Drops, describe};

mod quiet;
use quiet::{Quiet, ended};

/// What a body does with its values that have destructors: what's
/// refused of it, and its momentary borrows, which destructors ask.
#[derive(Default)]
pub(in crate::lower) struct Facts {
    /// What this body does that isn't supported yet, each where and what:
    /// a body lowered from its MIR is refused it too (`check_drop_facts`).
    pub(super) problems: Vec<(Span, String)>,
    /// Each borrow, by its call's `fun`, whose guard is held only while
    /// nothing can ask whether its cell is borrowed: checked, and not
    /// counted, nor dropped (ADR 0328).
    pub(in crate::lower) momentary: IndexSet<ExprId>,
    /// Each borrow, by its call's `fun`, whose guard a `let` holds, never
    /// moved but into `drop`, while nothing after it in its block can ask
    /// whether its cell is borrowed: not counted (ADR 0863).
    pub(in crate::lower) let_held: IndexSet<ExprId>,
    /// Each other, by its call's `fun`, whose guard is held only while code
    /// of the crate's or std's runs, which rust-js sees, while it's held: no
    /// library's code, nor JS's (ADR 0863).
    pub(in crate::lower) held_while_seen: IndexSet<ExprId>,
}

/// Find what's refused of a body, and its momentary borrows, before it's lowered.
pub(super) fn find_facts<'tcx>(
    cx: &DropQuery<'_, 'tcx>,
    borrows: &CellBorrows<'tcx>,
    thir: &Thir<'tcx>,
    body_owner: DefId,
) -> Facts {
    let mut finder = Finder {
        cx,
        borrows,
        thir,
        body_owner,
        ids: thir
            .exprs
            .iter_enumerated()
            .map(|(id, e)| (std::ptr::from_ref(e) as usize, id))
            .collect(),
        stack: Vec::new(),
        statements: Vec::new(),
        lets: IndexMap::default(),
        facts: Facts::default(),
    };
    for param in &thir.params {
        if let Some(pat) = &param.pat {
            finder.visit_pat(pat);
        }
    }
    // THIR is built from the leaves up, so the body's own expression is
    // the last, and every other is reached from it.
    if let Some(root) = thir.exprs.last_index() {
        finder.visit_expr(&thir[root]);
    }
    finder.facts
}

struct Finder<'c, 'q, 'a, 'tcx> {
    cx: &'c DropQuery<'q, 'tcx>,
    /// What each body may borrow (ADR 0580).
    borrows: &'c CellBorrows<'tcx>,
    thir: &'a Thir<'tcx>,
    /// The function or closure whose body this is.
    body_owner: DefId,
    ids: IndexMap<usize, ExprId>,
    /// The expressions being walked, outermost first.
    stack: Vec<ExprId>,
    /// The statements being walked, each its temporaries' scope and what's
    /// in it, a `let` with an `else` nothing; and the arms of a `match`,
    /// whose temporaries live while its body runs, one with a guard nothing.
    statements: Vec<(region::Scope, Option<ExprId>)>,
    /// Each `let` statement's value, and its pattern.
    lets: IndexMap<ExprId, &'a Pat<'tcx>>,
    facts: Facts,
}

impl<'c, 'q, 'a, 'tcx> Finder<'c, 'q, 'a, 'tcx> {
    fn id(&self, e: &ThirExpr<'tcx>) -> ExprId {
        self.ids[&(std::ptr::from_ref(e) as usize)]
    }

    /// What `e`, the top of the walk, is used for: the expression it's in,
    /// past what only passes it on, and which of its parts `e` is.
    fn context(&self) -> (Option<ExprId>, ExprId) {
        let at = self.stack.len().checked_sub(1).expect("an expression being walked");
        let (parent, child) = self.context_at(at);
        (parent.map(|p| self.stack[p]), child)
    }

    /// What the expression at `at` in the walk is used for, as `context`
    /// says of its top: where in the walk the expression it's in is, and
    /// which of its parts the one at `at` is.
    fn context_at(&self, at: usize) -> (Option<usize>, ExprId) {
        let mut child = self.stack[at];
        for parent in (0..at).rev() {
            // A `let`'s value is its statement's, not the block's around it:
            // its pattern says what it takes.
            if self.lets.contains_key(&child) {
                return (None, child);
            }
            let expr = self.stack[parent];
            match self.thir[expr].kind {
                ExprKind::Scope { .. }
                | ExprKind::Use { .. }
                | ExprKind::ValueTypeAscription { .. }
                | ExprKind::PlaceTypeAscription { .. } => child = expr,
                // A type that's the value it holds, `AssertUnwindSafe(f)`, is
                // its value's place (ADR 0660).
                ExprKind::Adt(..) if self.cx.recognition.transparent(self.thir[expr].ty).is_some() => child = expr,
                _ => return (Some(parent), child),
            }
        }
        (None, child)
    }

    /// What dropping `ty`, a type the THIR names, runs, as the body has it:
    /// a copied default's in its impl's terms (`DropQuery::thir_ty`, #828).
    fn drops_of(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.cx.drops(self.cx.thir_ty(ty))
    }

    fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.drops_of(ty) == Drops::Runs
    }

    /// Whether a value made here has a destructor to run: `None` of an
    /// `Option` that could hold one doesn't, nor does a variant of an enum
    /// without a `Drop` of its own whose fields have none.
    fn holds_drops(&self, expr: &ThirExpr<'tcx>) -> bool {
        if !self.has_drops(expr.ty) {
            return false;
        }
        match &expr.kind {
            ExprKind::Adt(adt) => {
                self.cx
                    .recognition
                    .tcx
                    .adt_destructor(adt.adt_def.did())
                    .is_some_and(|d| self.cx.runs_drop(d.did))
                    || adt.fields.iter().any(|f| self.has_drops(self.thir[f.expr].ty))
                    || !matches!(adt.base, AdtExprBase::None)
            }
            _ => true,
        }
    }

    fn problem(&mut self, span: Span, what: &str) {
        self.facts.problems.push((span, what.to_string()));
    }

    /// A closure that holds a value with a destructor drops it through the
    /// variables it took, which JS sees only where they're seen: it's made
    /// for a `let` or a call, and written over nowhere. One its own body
    /// doesn't call once isn't given to a call by value, which may consume it.
    fn closure_checked(&mut self, expr: &ThirExpr<'tcx>) {
        match expr.kind {
            ExprKind::Closure(_) => match self.drops_of(expr.ty) {
                Drops::Nothing => {}
                Drops::Unsupported(t, what) => self.problem(expr.span, &describe(t, what)),
                Drops::Runs => {
                    let placed = self.placed(self.stack.len() - 1);
                    // One its maker gives back or makes a `dyn Fn` carries its drop (ADRs
                    // 0484, 0521).
                    let carried = matches!(expr.kind, ExprKind::Closure(ref c)
                        if self.cx.closure_facts.carries_its_drop(self.cx.recognition.tcx, c.closure_id.to_def_id()));
                    if !placed && !carried {
                        self.problem(expr.span, "a closure that holds a value with a destructor, made here");
                    }
                }
            },
            // One a binding calls once, made where it's given, drops what it
            // took as that call ends (ADR 1020).
            ExprKind::Call { ref args, ty, .. }
                if !self.drops_given(ty)
                    && args.iter().any(|&a| {
                        let ty = self.thir[a].ty;
                        self.borrowing_closure(ty)
                            && !matches!(*ty.kind(), ty::Closure(id, _) if self.cx.closure_facts.called_once(id))
                    }) =>
            {
                self.problem(
                    expr.span,
                    "a closure that holds a value with a destructor, given away without being called once",
                );
            }
            ExprKind::Assign { lhs, .. }
                if self.thir[lhs].ty.walk().any(|part| {
                    part.as_type()
                        .is_some_and(|t| matches!(t.kind(), ty::Closure(..)) && self.drops_of(t) != Drops::Nothing)
                }) =>
            {
                self.problem(expr.span, "assigning a closure that holds a value with a destructor");
            }
            _ => {}
        }
    }

    /// Whether the closure at `at` in the walk is where JS sees what it
    /// took: made for a `let` or given to a call, or the value of a block
    /// that is, `{ let r = r.clone(); move || *r }` or `fold(0, { let g =
    /// D(1); move |a, x| .. })` (#759), whose variables are the function's
    /// in the JS, seen where the block is.
    fn placed(&self, at: usize) -> bool {
        let (parent, child) = self.context_at(at);
        match parent.map(|p| (p, &self.thir[self.stack[p]].kind)) {
            None => self.lets.contains_key(&child),
            Some((_, ExprKind::Call { args, .. })) => args.contains(&child),
            Some((block, ExprKind::Block { block: b })) if self.thir[*b].expr == Some(child) => self.placed(block),
            _ => false,
        }
    }

    /// Whether a call of `callee`, a function's type, may drop a closure
    /// it's given that's called by `&` or `&mut`: std's, which drops it as
    /// it returns, or the crate's own function's, as its caller's drop says,
    /// each where its lowering says (ADR 0470), and a library's the crate
    /// compiles itself, its own then (ADR 0914, #1051). Not a binding's,
    /// which JS holds, nor another crate's JS.
    fn drops_given(&self, callee: Ty<'tcx>) -> bool {
        let tcx = self.cx.recognition.tcx;
        let Some((def_id, _)) = fn_def(callee) else {
            return false;
        };
        !crate::lower::bindings::is_binding(tcx, def_id)
            && (def_id.is_local()
                || crate::lower::recognition::is_std_crate(tcx, def_id)
                || self.cx.recognition.foreign.is_instance(def_id))
    }

    /// Whether `ty` is a closure that holds a value with a destructor, which
    /// a call it's given by value may consume, though it isn't called once
    /// by its own body: what that drops, the call doesn't (ADR 0098).
    fn borrowing_closure(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Closure(_, args) if args.as_closure().kind() != ty::ClosureKind::FnOnce)
            && self.has_drops(ty)
    }

    /// A value with a destructor, made here, at the top of the walk: one
    /// borrowed, a guard, may be momentary. One taken apart, `let (a, _) =
    /// (x, y);`, is moved out of part by part as MIR's lowering moves it, the
    /// rest dropped as the statement ends, or its pattern's arm (#1109, #1337).
    fn value_made(&mut self, e: ExprId) {
        let (parent, child) = self.context();
        let used_in_place = match parent.map(|p| &self.thir[p].kind) {
            None => {
                let Some(&pat) = self.lets.get(&child) else {
                    return;
                };
                if matches!(pat.kind, PatKind::Wild | PatKind::Binding { subpattern: None, .. }) {
                    return;
                }
                true
            }
            Some(ExprKind::Let { expr, .. }) if *expr == child => true,
            Some(ExprKind::Match { scrutinee, .. }) if *scrutinee == child => true,
            Some(
                ExprKind::Borrow { arg, .. }
                | ExprKind::RawBorrow { arg, .. }
                | ExprKind::Field { lhs: arg, .. }
                | ExprKind::Index { lhs: arg, .. },
            ) => *arg == child,
            _ => false,
        };
        if used_in_place {
            self.momentary(e);
        }
    }

    /// A `RefCell`'s `replace_with(f)`, `call`, which holds its borrow while
    /// `f` runs: momentary where nothing `f` runs can ask of it (#687), as a
    /// guard's held only while nothing can ask is (ADR 0328).
    fn replaced_with(&mut self, call: ExprId) {
        let recognition = &self.cx.recognition;
        let ExprKind::Call { fun, .. } = self.thir[call].kind else {
            return;
        };
        let Some((id, generic_args)) = fn_def(self.thir[fun].ty) else {
            return;
        };
        if recognition.classify(id, generic_args) != Some(Std::CellReplaceWith) {
            return;
        }
        let Some(cell) = self
            .borrows
            .borrowed_in(recognition, (self.body_owner, self.thir), call)
        else {
            return;
        };
        let drops = |ty| self.cx.drops_but_guards(self.cx.thir_ty(ty));
        if !self.borrows.bounds_ask(recognition, (id, generic_args), cell, &drops) {
            self.facts.momentary.insert(fun);
        }
    }

    /// A guard made here, `c.borrow_mut()` say, that's held only while
    /// nothing can ask whether its cell is borrowed: its borrow is checked,
    /// and not counted (ADR 0328).
    fn momentary(&mut self, e: ExprId) {
        let recognition = &self.cx.recognition;
        let ty = self.thir[e].ty;
        // Or a `try_borrow`'s `Result` of one, `match c.try_borrow() { Ok(v)
        // => .., Err(_) => .. }`, its guard held as a borrow's is (#687).
        let tried = matches!(ty.kind(), ty::Adt(_, args)
            if recognition.is_std_type(ty, StdItem::Result) && args.types().next().is_some_and(|ok| recognition.is_guard(ok)));
        if !recognition.is_guard(ty) && !tried {
            return;
        }
        // The borrow: `c.borrow()`, or `m.lock()` of `m.lock().unwrap()`.
        let mut made = e;
        let (fun, cell) = loop {
            let call = strip(self.thir, made);
            let ExprKind::Call { fun, ref args, .. } = self.thir[call].kind else {
                return;
            };
            match fn_def(self.thir[fun].ty).and_then(|(id, args)| recognition.classify(id, args)) {
                Some(Std::TryBorrow { lock: false, .. }) if tried => {
                    match self
                        .borrows
                        .borrowed_in(recognition, (self.body_owner, self.thir), call)
                    {
                        Some(cell) => break (fun, cell),
                        None => return,
                    }
                }
                Some(Std::Borrow { .. }) => {
                    match self
                        .borrows
                        .borrowed_in(recognition, (self.body_owner, self.thir), call)
                    {
                        Some(cell) => break (fun, cell),
                        None => return,
                    }
                }
                Some(Std::UnwrapOk) => made = args[0],
                _ => return,
            }
        };
        let tree = recognition.tcx.region_scope_tree(self.body_owner.expect_local());
        let Some(scope) = tree.temporary_scope(self.thir[e].temp_scope_id).temp_lifetime else {
            return;
        };
        // An `if let`'s scrutinee's, which lives while its test and its
        // `then` run, not its `else` (edition 2024).
        let if_then = (self.stack.iter().rev()).find_map(|&s| match self.thir[s].kind {
            ExprKind::If {
                if_then_scope,
                cond,
                then,
                ..
            } if if_then_scope == scope => Some([cond, then]),
            _ => None,
        });
        let extent = self
            .stack
            .iter()
            .rev()
            .copied()
            .find(|&s| matches!(self.thir[s].kind, ExprKind::Scope { region_scope, .. } if region_scope == scope))
            .or_else(|| {
                self.statements
                    .iter()
                    .rev()
                    .find(|(s, _)| *s == scope)
                    .and_then(|&(_, root)| root)
            });
        let extents: Vec<ExprId> = match (if_then, extent) {
            (Some(both), _) => both.to_vec(),
            // A guard of an assignment's place, `*c.borrow_mut() = f(v);`,
            // is made after its value, which Rust evaluates first: held
            // while its place is read and written, not while `f` runs (gap
            // rd32). Of a value with a destructor written over, which runs
            // while it's held, the whole statement, as before.
            (None, Some(extent))
                if let ExprKind::Assign { lhs, .. } = self.thir[strip(self.thir, extent)].kind
                    && (lhs == e || self.stack.contains(&lhs))
                    && !self.cx.drops_but_guards(self.cx.thir_ty(self.thir[lhs].ty)) =>
            {
                vec![lhs]
            }
            // A guard that's only the receiver of the statement's call,
            // `target.borrow_mut().push(show(value));`, is held while the
            // call runs, not while its other arguments do, as JS's
            // `target.push(show(value))` evaluates them first (ADR 0863,
            // amended, the user's decision): of them, only a borrow
            // rust-js sees asks of it.
            (None, Some(extent))
                if let ExprKind::Call {
                    fun: method, ref args, ..
                } = self.thir[strip(self.thir, extent)].kind
                    && let [receiver, rest @ ..] = &args[..]
                    && (*receiver == e || self.stack.contains(receiver))
                    && self.held_quietly_after((fun, cell), (strip(self.thir, extent), method, *receiver), rest) =>
            {
                self.facts.momentary.insert(fun);
                return;
            }
            // A guard read only for a binding's argument, what it guards
            // lent, `as_deref()`'s, or a copy or clone of it, `prepare(
            // response.nonce.borrow().as_deref())`, is held while that's read
            // and the arguments after it run, not while the binding does,
            // as JS's `prepare(response.nonce)` reads it first (ADR 0863,
            // amended, the user's decision, rd427): a binding that calls
            // back and writes the cell meets no borrow, where Rust's would.
            (None, Some(extent))
                if let ExprKind::Call {
                    fun: binding, ref args, ..
                } = self.thir[strip(self.thir, extent)].kind
                    && fn_def(self.thir[binding].ty)
                        .is_some_and(|(id, _)| crate::lower::bindings::is_binding(recognition.tcx, id))
                    && let Some(at) = args.iter().position(|&arg| arg == e || self.stack.contains(&arg))
                    && self.given_of_guard(args[at], e) =>
            {
                args[at..].to_vec()
            }
            (None, Some(extent)) => vec![extent],
            (None, None) => return,
        };
        if self.held_quietly((fun, cell), (&[], &extents), &self.ended_before()) {
            self.facts.momentary.insert(fun);
        }
    }

    /// Of the expressions being walked, the parts of each that run, and
    /// end, before `e`, the top of the walk, is made, in Rust's order of
    /// evaluation (#552): a call's callee and its arguments before the one
    /// `e` is in, a tuple's or an array's parts before it, a binary
    /// operation's left side of its right. `held(&log, 2)` of `println!("{}
    /// {:?}", held(&log, 2), log.borrow())`, which returns before the `Ref`
    /// is made, so no borrow it makes meets it. None runs again while it's
    /// held: a loop's body ends the temporaries made in it, so a guard's
    /// extent is inside each loop around it.
    fn ended_before(&self) -> IndexSet<ExprId> {
        let mut ended = IndexSet::default();
        for pair in self.stack.windows(2) {
            let &[parent, child] = pair else { continue };
            let before: &[ExprId] = match self.thir[parent].kind {
                ExprKind::Call { fun, ref args, .. } => match args.iter().position(|&arg| arg == child) {
                    Some(at) => {
                        ended.insert(fun);
                        &args[..at]
                    }
                    None => &[],
                },
                ExprKind::Tuple { ref fields } | ExprKind::Array { ref fields } => {
                    match fields.iter().position(|&field| field == child) {
                        Some(at) => &fields[..at],
                        None => &[],
                    }
                }
                ExprKind::Binary { lhs, rhs, .. } | ExprKind::LogicalOp { lhs, rhs, .. } if rhs == child => {
                    ended.insert(lhs);
                    &[]
                }
                _ => &[],
            };
            ended.extend(before.iter().copied());
        }
        ended
    }

    /// Of `e`, a borrow of what a guard guards, `&mut *c.borrow_mut()` or
    /// `&*c.borrow()`, through its `Deref`: the call that makes the guard.
    fn guarded_borrow(&self, e: ExprId) -> Option<ExprId> {
        let thir = self.thir;
        let ExprKind::Borrow { arg, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let ExprKind::Deref { arg } = thir[strip(thir, arg)].kind else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind else {
            return None;
        };
        let derefs = fn_def(thir[fun].ty).is_some_and(|(id, generic_args)| {
            matches!(
                handle_step(self.cx.recognition.tcx, id, generic_args),
                Some(HandleStep::GuardDeref | HandleStep::GuardDerefMut)
            )
        });
        match args[..] {
            [lent] if derefs => match thir[strip(thir, lent)].kind {
                ExprKind::Borrow { arg: guard, .. } => Some(strip(thir, guard)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether `arg`, a call's argument, is only what `guard` guards, read
    /// through it: lent, `&*guard`, a copy, `*guard`, or a clone or an
    /// `Option`'s `as_deref()` of it lent (rd427).
    fn given_of_guard(&self, arg: ExprId, guard: ExprId) -> bool {
        let thir = self.thir;
        let tcx = self.cx.recognition.tcx;
        let guard = strip(thir, guard);
        // `*guard`, through its `Deref`.
        let read = |e: ExprId| {
            let ExprKind::Deref { arg } = thir[strip(thir, e)].kind else {
                return false;
            };
            let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind else {
                return false;
            };
            fn_def(thir[fun].ty).is_some_and(|(id, generic_args)| {
                matches!(
                    handle_step(tcx, id, generic_args),
                    Some(HandleStep::GuardDeref | HandleStep::GuardDerefMut)
                )
            }) && matches!(args[..], [lent] if matches!(thir[strip(thir, lent)].kind,
                ExprKind::Borrow { arg, .. } if strip(thir, arg) == guard))
        };
        let lent = |e: ExprId| match thir[strip(thir, e)].kind {
            ExprKind::Borrow { arg, .. } => read(arg),
            _ => read(e),
        };
        match thir[strip(thir, arg)].kind {
            ExprKind::Call { fun, ref args, .. }
                if let [of] = args[..]
                    && fn_def(thir[fun].ty).is_some_and(|(id, _)| {
                        tcx.is_lang_item(id, rustc_hir::attrs::lang_items::LangItem::CloneFn) || is_as_deref(tcx, id)
                    }) =>
            {
                lent(of)
            }
            _ => lent(arg),
        }
    }

    /// A guard a `let` holds, `let v = c.borrow();`, until its block ends or
    /// `drop(v)`: not counted where nothing after it in its block can ask
    /// whether its cell is borrowed (ADR 0863), as a momentary one isn't
    /// (ADR 0328). Not one moved anywhere else, which may outlive what's
    /// asked of its block.
    fn let_held(&mut self, pattern: &Pat<'tcx>, init: ExprId, (rest, value): (&[StmtId], Option<ExprId>)) {
        let PatKind::Binding {
            var,
            mode: BindingMode(ByRef::No, _),
            subpattern: None,
            ty,
            ..
        } = pattern.kind
        else {
            return;
        };
        let recognition = &self.cx.recognition;
        // Or a borrow of what it guards, `let state = &mut
        // *chunk.state.borrow_mut();`: its guard a temporary the `let` holds
        // to its block's end, as Rust extends it, whatever the borrow does,
        // which can't outlive it (rd432).
        let through = self.guarded_borrow(init);
        let call = through.unwrap_or_else(|| strip(self.thir, init));
        let ExprKind::Call { fun, .. } = self.thir[call].kind else {
            return;
        };
        let ty = if through.is_some() { self.thir[call].ty } else { ty };
        if !recognition.is_guard(ty)
            || !matches!(
                fn_def(self.thir[fun].ty).and_then(|(id, args)| recognition.classify(id, args)),
                Some(Std::Borrow { lock: false, .. })
            )
        {
            return;
        }
        let Some(cell) = self
            .borrows
            .borrowed_in(recognition, (self.body_owner, self.thir), call)
        else {
            return;
        };
        let mut moves = Moves {
            recognition,
            thir: self.thir,
            var,
            moved: false,
        };
        // Held until a statement of its block drops it, `drop(v);`, or
        // its block ends; a borrow of what it guards, until its block ends.
        let dropped = rest.iter().position(|&stmt| match self.thir[stmt].kind {
            ThirStmt::Expr { expr, .. } => through.is_none() && moves.drops_it(expr),
            ThirStmt::Let { .. } => false,
        });
        let (rest, value) = match dropped {
            Some(at) => (&rest[..at], None),
            None => (rest, value),
        };
        if through.is_none() {
            for &stmt in rest {
                moves.visit_stmt(&self.thir[stmt]);
            }
            if let Some(value) = value {
                moves.visit_expr(&self.thir[value]);
            }
        }
        if !moves.moved && self.held_quietly((fun, cell), (rest, value.as_slice()), &IndexSet::default()) {
            self.facts.let_held.insert(fun);
        }
    }

    /// Whether `fun`'s guard of `cell`, the receiver of `call`, a call of
    /// `method`, meets no borrow that may ask of it while it's held: what
    /// its receiver and the call do, and, of the call's other arguments,
    /// which run before it's held (ADR 0863, amended), what rust-js sees.
    fn held_quietly_after(
        &mut self,
        (fun, cell): (ExprId, Borrow<'tcx>),
        (call, method, receiver): (ExprId, ExprId, ExprId),
        rest: &[ExprId],
    ) -> bool {
        let mut quiet = Quiet {
            cx: self.cx,
            borrows: self.borrows,
            thir: self.thir,
            owner: self.body_owner,
            own: fun,
            cell,
            quiet: true,
            items: IndexSet::default(),
            unmet: Vec::new(),
            seen_only: false,
            ids: &self.ids,
            ended: &IndexSet::default(),
        };
        quiet.items = quiet.items_of_own();
        quiet.visit_expr(&self.thir[receiver]);
        quiet.visit_call(call, method);
        quiet.seen_only = true;
        for &arg in rest {
            quiet.visit_expr(&self.thir[arg]);
        }
        quiet.quiet
    }

    /// Whether `fun`'s guard of `cell`, held while `stmts` and `exprs` run,
    /// but those of their parts `ended` before it's made, meets no borrow
    /// that may ask of it (`Quiet`); if one may, whether only code rust-js
    /// sees runs while it's held (`Seen`), a fact of it.
    fn held_quietly(
        &mut self,
        (fun, cell): (ExprId, Borrow<'tcx>),
        (stmts, exprs): (&[StmtId], &[ExprId]),
        ended: &IndexSet<ExprId>,
    ) -> bool {
        let mut quiet = Quiet {
            cx: self.cx,
            borrows: self.borrows,
            thir: self.thir,
            owner: self.body_owner,
            own: fun,
            cell,
            quiet: true,
            items: IndexSet::default(),
            unmet: Vec::new(),
            seen_only: false,
            ids: &self.ids,
            ended,
        };
        quiet.items = quiet.items_of_own();
        for &stmt in stmts {
            quiet.visit_stmt(&self.thir[stmt]);
        }
        for &extent in exprs {
            quiet.visit_expr(&self.thir[extent]);
        }
        if quiet.quiet {
            return true;
        }
        let mut seen = Seen {
            recognition: &self.cx.recognition,
            borrows: self.borrows,
            thir: self.thir,
            owner: self.body_owner,
            seen: true,
            ids: &self.ids,
            ended,
        };
        for &stmt in stmts {
            seen.visit_stmt(&self.thir[stmt]);
        }
        for &extent in exprs {
            seen.visit_expr(&self.thir[extent]);
        }
        if seen.seen {
            self.facts.held_while_seen.insert(fun);
        }
        false
    }
}

/// Whether a `let`'s guard `var` is moved, but into `drop`: given by value
/// anywhere, which may keep it (ADR 0863). A reference to it can't outlive
/// it.
struct Moves<'c, 'a, 'tcx> {
    recognition: &'c crate::lower::recognition::Recognition<'c, 'tcx>,
    thir: &'a Thir<'tcx>,
    var: LocalVarId,
    moved: bool,
}

impl<'c, 'a, 'tcx> Moves<'c, 'a, 'tcx> {
    fn is_var(&self, e: ExprId) -> bool {
        matches!(self.thir[strip(self.thir, e)].kind, ExprKind::VarRef { id } if id == self.var)
    }

    /// Whether `e` is `drop(var)`.
    fn drops_it(&self, e: ExprId) -> bool {
        self.is_drop(&self.thir[strip(self.thir, e)].kind)
    }

    fn is_drop(&self, kind: &ExprKind<'tcx>) -> bool {
        matches!(*kind, ExprKind::Call { fun, ref args, .. }
            if matches!(
                fn_def(self.thir[fun].ty).and_then(|(id, a)| self.recognition.classify(id, a)),
                Some(Std::Drop)
            ) && matches!(args[..], [only] if self.is_var(only)))
    }
}

impl<'c, 'a, 'tcx> Visitor<'a, 'tcx> for Moves<'c, 'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        if self.moved {
            return;
        }
        match expr.kind {
            ExprKind::Borrow { arg, .. } | ExprKind::RawBorrow { arg, .. } if self.is_var(arg) => return,
            ExprKind::Call { .. } if self.is_drop(&expr.kind) => return,
            ExprKind::VarRef { id } if id == self.var => {
                self.moved = true;
                return;
            }
            // What a closure takes, which rustc's walk doesn't reach.
            ExprKind::Closure(ref closure) => {
                for &upvar in closure.upvars.iter() {
                    self.visit_expr(&self.thir[upvar]);
                }
            }
            _ => {}
        }
        visit::walk_expr(self, expr);
    }
}

/// Whether evaluating an expression runs only code rust-js sees, the
/// crate's or std's, of no library's nor JS's (ADR 0863).
struct Seen<'c, 'a, 'tcx> {
    recognition: &'c crate::lower::recognition::Recognition<'c, 'tcx>,
    borrows: &'c CellBorrows<'tcx>,
    thir: &'a Thir<'tcx>,
    /// The function or closure whose body this is.
    owner: DefId,
    seen: bool,
    /// Each expression's id, by where it is.
    ids: &'c IndexMap<usize, ExprId>,
    /// What ended before the guard was made (`Finder::ended_before`).
    ended: &'c IndexSet<ExprId>,
}

impl<'c, 'a, 'tcx> Visitor<'a, 'tcx> for Seen<'c, 'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        if !self.seen || ended(self.ids, self.ended, expr) {
            return;
        }
        self.seen = match expr.kind {
            ExprKind::Call { fun, ref args, .. } => match fn_def(self.thir[fun].ty) {
                // A promise's resolve, given what runs nothing (rd654).
                Some(called) if resolve_runs_nothing(self.recognition, self.thir, called, args) => true,
                Some((id, args)) => !self.borrows.call_runs_unseen(self.recognition, (self.owner, id, args)),
                // A fn pointer's, of what it may be (ADR 0907).
                None => !self
                    .borrows
                    .pointer_call_runs_unseen(self.recognition, self.thir[fun].ty),
            },
            ExprKind::InlineAsm(_) | ExprKind::Yield { .. } | ExprKind::Become { .. } => false,
            _ => true,
        };
        visit::walk_expr(self, expr);
    }
}

/// A place: a variable, or a part of one, or what a reference points to.
pub(in crate::lower) fn is_place(kind: &ExprKind<'_>) -> bool {
    matches!(
        kind,
        ExprKind::VarRef { .. }
            | ExprKind::UpvarRef { .. }
            | ExprKind::Field { .. }
            | ExprKind::Index { .. }
            | ExprKind::Deref { .. }
            | ExprKind::StaticRef { .. }
            | ExprKind::Scope { .. }
            | ExprKind::Use { .. }
            | ExprKind::ValueTypeAscription { .. }
            | ExprKind::PlaceTypeAscription { .. }
            | ExprKind::NeverToAny { .. }
    )
}

impl<'c, 'q, 'a, 'tcx> Visitor<'a, 'tcx> for Finder<'c, 'q, 'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_stmt(&mut self, stmt: &'a rustc_middle::thir::Stmt<'tcx>) {
        if let ThirStmt::Let {
            initializer: Some(init),
            pattern,
            ..
        } = &stmt.kind
        {
            self.lets.insert(*init, pattern);
        }
        // A `let`'s temporaries live while its initializer runs, a
        // let-else's too, which drops them before its `else` runs, as
        // Rust does in either edition (ADR 0863, amended).
        self.statements.push(match stmt.kind {
            ThirStmt::Expr { scope, expr } => (scope, Some(expr)),
            ThirStmt::Let {
                init_scope,
                initializer,
                ..
            } => (init_scope, initializer),
        });
        visit::walk_stmt(self, stmt);
        self.statements.pop();
    }

    fn visit_block(&mut self, block: &'a Block) {
        for (i, &stmt) in block.stmts.iter().enumerate() {
            if let ThirStmt::Let {
                ref pattern,
                initializer: Some(init),
                else_block: None,
                ..
            } = self.thir[stmt].kind
            {
                self.let_held(pattern, init, (&block.stmts[i + 1..], block.expr));
            }
        }
        visit::walk_block(self, block);
    }

    fn visit_arm(&mut self, arm: &'a rustc_middle::thir::Arm<'tcx>) {
        self.statements
            .push((arm.scope, arm.guard.is_none().then_some(arm.body)));
        visit::walk_arm(self, arm);
        self.statements.pop();
    }

    fn visit_pat(&mut self, pat: &'a Pat<'tcx>) {
        if let PatKind::Binding {
            mode: BindingMode(ByRef::No, _),
            ty,
            ..
        } = pat.kind
            && let Drops::Unsupported(t, what) = self.drops_of(ty)
        {
            self.problem(pat.span, &describe(t, what));
        }
        visit::walk_pat(self, pat);
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        let id = self.id(expr);
        self.stack.push(id);
        match expr.kind {
            _ if !is_place(&expr.kind) && self.holds_drops(expr) => self.value_made(id),
            ExprKind::PointerCoercion {
                cast: PointerCoercion::Unsize,
                source,
                ..
            } if {
                // To a `dyn`: an array unsized to a slice is the same array.
                // What's unsized, and what to: `&D` or `Box<D>` to a `dyn`, and
                // `Rc<D>` to `Rc<dyn Send>` too, whose destructor then runs
                // through the `dyn`. Found by rustc's `issue-25515.rs`.
                let target = expr.ty;
                let from = self.thir[source].ty;
                let (to, from) = match (target.builtin_deref(true), from.builtin_deref(true)) {
                    (Some(to), Some(from)) => (to, from),
                    _ => (target, from),
                };
                // What owns one drops it through its dictionary's `$drop`.
                self.cx
                    .dyn_drop_lost(self.cx.thir_ty(target), self.cx.thir_ty(from), self.cx.thir_ty(to))
            } =>
            {
                self.problem(expr.span, "a `dyn` of a value with a destructor");
            }
            _ => {}
        }
        self.replaced_with(id);
        self.closure_checked(expr);
        // What a closure takes, which rustc's walk doesn't reach: a capture by
        // value moves it in.
        if let ExprKind::Closure(ref closure) = expr.kind {
            for &upvar in closure.upvars.iter() {
                self.visit_expr(&self.thir[upvar]);
            }
        }
        visit::walk_expr(self, expr);
        self.stack.pop();
    }
}

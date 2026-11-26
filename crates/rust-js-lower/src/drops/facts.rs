//! What a body does with its values that have destructors that MIR's drop
//! elaboration can't settle, found before it's lowered (ADR 0098): what's
//! refused of it, a walk of the THIR that asks what types drop of a
//! `DropQuery`, and sees nothing else of the function; and the borrows a
//! guard holds only for a moment (ADR 0328), each `RefCell` borrow's as
//! `CellForms` decides it (#2490), a `replace_with`'s as its bounds ask.

use rust_js_program::collections::{IndexMap, IndexSet};

use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{Expr as ThirExpr, ExprId, ExprKind, Pat, StmtKind as ThirStmt, Thir};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::super::recognition::{Std, StdItem};
use super::super::{fn_def, strip};

use super::super::analysis::{CellBorrows, CellForms};
use super::call_once::OnceDrop;
use super::types::{DropQuery, Drops, describe};

/// What a body does with its values that have destructors: what's
/// refused of it, and its momentary borrows, which destructors ask.
#[derive(Default)]
pub(crate) struct Facts {
    /// What this body does that isn't supported yet, each where and what:
    /// a body lowered from its MIR is refused it too (`check_drop_facts`).
    pub(super) problems: Vec<(Span, String)>,
    /// Each borrow, by its call's `fun`, whose guard is held only while
    /// nothing can ask whether its cell is borrowed: checked, and not
    /// counted, nor dropped (ADR 0328). A `RefCell`'s borrow or
    /// `try_borrow` as `CellForms` proves it (#2490); a `replace_with`'s
    /// where nothing its function runs can ask (#687).
    pub(crate) momentary: IndexSet<ExprId>,
    /// Each `RefCell` borrow, by its call's `fun`, whose guard is held only
    /// while code of the crate's or std's runs, which rust-js sees: no
    /// library's code, nor JS's (ADR 0863), as `CellForms::seen` says.
    pub(crate) held_while_seen: IndexSet<ExprId>,
}

/// Find what's refused of a body, and its momentary borrows, before it's lowered.
pub(super) fn find_facts<'tcx>(
    cx: &DropQuery<'_, 'tcx>,
    (borrows, forms): (&CellBorrows<'tcx>, &CellForms),
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
    // A `RefCell` borrow is counted unless `CellForm`'s proof says it
    // isn't (#2490's flip): one proved isn't counted, and is checked where
    // it's made, a momentary one, but where no borrow of its cell can meet
    // another, which its cell's other borrows say (`RefCells::quiet_class`,
    // `quiet_field`).
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let Some((id, generic_args)) = fn_def(thir[fun].ty) else {
            continue;
        };
        let borrow = matches!(
            cx.recognition.classify(id, generic_args),
            Some(Std::Borrow { lock: false, .. } | Std::TryBorrow { lock: false, .. })
        );
        let of_a_ref_cell =
            (args.first()).is_some_and(|&cell| cx.recognition.is_std_type(thir[cell].ty.peel_refs(), StdItem::RefCell));
        if !borrow || !of_a_ref_cell {
            continue;
        }
        let facts = &mut finder.facts;
        if !forms.counted(body_owner, thir[fun].span) {
            facts.momentary.insert(fun);
        }
        if forms.seen(body_owner, thir[fun].span) {
            facts.held_while_seen.insert(fun);
        }
    }
    // `--count-every-cell`'s: no borrow is momentary, each counted (#2491).
    if crate::session::caches(cx.recognition.tcx).count_every_cell {
        finder.facts.momentary.clear();
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
    /// Each `let` statement's value, and its pattern.
    lets: IndexMap<ExprId, &'a Pat<'tcx>>,
    facts: Facts,
}

impl<'c, 'q, 'a, 'tcx> Finder<'c, 'q, 'a, 'tcx> {
    fn id(&self, e: &ThirExpr<'tcx>) -> ExprId {
        self.ids[&(std::ptr::from_ref(e) as usize)]
    }

    /// What the expression at `at` in the walk is used for: where in the
    /// walk the expression it's in is, and which of its parts the one at
    /// `at` is.
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
                | ExprKind::ValueExpr { .. }
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

    fn problem(&mut self, span: Span, what: &str) {
        self.facts.problems.push((span, what.to_string()));
    }

    /// A guard a closure takes by reference, written whole there, `|| state
    /// = other`: refused (#2475). Its variable's release is decided by the
    /// guard its `let` gives it, uncounted where `CellForms` proves it
    /// (#2490), which a write in a closure doesn't change, so another
    /// cell's counted guard written there was never released. Written where
    /// the variable is, `state = other;`, it's seen, and a guard's `&mut`
    /// given to code that writes it whole is refused where it's given
    /// (`generic_facts::refuse_guard_written_whole`).
    fn guard_replaced_in_a_closure(&mut self, expr: &ThirExpr<'tcx>) {
        let tcx = self.cx.recognition.tcx;
        if let ExprKind::Assign { lhs, .. } = expr.kind
            && self.cx.recognition.is_guard(self.thir[lhs].ty)
            && let ExprKind::UpvarRef {
                closure_def_id,
                var_hir_id,
            } = self.thir[strip(self.thir, lhs)].kind
            && let Some(closure) = closure_def_id.as_local()
            && (tcx.closure_captures(closure).iter())
                .any(|captured| captured.get_root_variable() == var_hir_id.0 && captured.is_by_ref())
        {
            self.problem(
                expr.span,
                "a `RefCell` guard written whole by a closure that borrows its variable, `|| state = other`, whose release is decided by the guard it's first given (#2475): write it where the variable is, `state = other;`",
            );
        }
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
                    // One given back where JS calls it once, an effect's
                    // cleanup, drops what it took as that call ends (#1881).
                    let once = matches!(expr.kind, ExprKind::Closure(ref c)
                        if self.cx.closure_facts.returned_once(c.closure_id.to_def_id()));
                    if !placed && !carried && !once {
                        self.problem(expr.span, "a closure that holds a value with a destructor, made here");
                    } else if let ExprKind::Closure(ref c) = expr.kind
                        && let closure = c.closure_id.to_def_id()
                        && self.cx.closure_facts.called_by_value(closure)
                        && !self.cx.closure_facts.body_is_its_call_once(closure, true)
                    {
                        // Generic code calls it by `FnOnce`, whose call drops
                        // none of it, where Rust's drops it once it's run
                        // (ADR 0470, #2031); but one whose body is its
                        // `call_once`'s (ADR 1020, #995).
                        self.problem(
                            expr.span,
                            "a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it",
                        );
                    } else if let ExprKind::Closure(ref c) = expr.kind
                        && self.cx.closure_facts.held_fn_dyn(c.closure_id.to_def_id())
                    {
                        // Made a `dyn Fn` that's called by `FnOnce` where it's
                        // held in what's given, or given back, which no call
                        // gives as one whose call drops it (#2050).
                        self.problem(
                            expr.span,
                            "a closure that holds a value with a destructor, made a `dyn Fn` called by `FnOnce` where nothing drops it after",
                        );
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
                            && !matches!(*ty.kind(), ty::Closure(id, _)
                                if self.cx.closure_facts.body_is_its_call_once(id, true))
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
        !crate::bindings::is_binding(tcx, def_id)
            && (def_id.is_local()
                || crate::recognition::is_std_crate(tcx, def_id)
                || self.cx.recognition.foreign.is_instance(def_id))
    }

    /// Whether `ty` is a closure that holds a value with a destructor, which
    /// a call it's given by value may consume, though it isn't called once
    /// by its own body: what that drops, the call doesn't (ADR 0098).
    fn borrowing_closure(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Closure(_, args) if OnceDrop::of_closure(args) == OnceDrop::Whole) && self.has_drops(ty)
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
        let drops = |ty| self.cx.drop_may_ask(self.cx.thir_ty(ty));
        if !self.borrows.bounds_ask(recognition, (id, generic_args), cell, &drops) {
            self.facts.momentary.insert(fun);
        }
    }
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
        visit::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        let id = self.id(expr);
        self.stack.push(id);
        self.replaced_with(id);
        self.closure_checked(expr);
        self.guard_replaced_in_a_closure(expr);
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

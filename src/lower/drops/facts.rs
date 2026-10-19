//! What a body does with its values that have destructors that MIR's drop
//! elaboration can't settle, found before it's lowered (ADR 0098): what's
//! refused of it, and the borrows a guard holds only for a moment (ADR
//! 0328). A walk of the THIR that asks what types drop of a `DropQuery`,
//! and sees nothing else of the function.

use std::collections::{HashMap, HashSet};
use std::iter::once;

use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{
    AdtExprBase, Expr as ThirExpr, ExprId, ExprKind, LocalVarId, Pat, PatKind, StmtKind as ThirStmt, Thir,
};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::super::recognition::Std;
use super::super::{fn_def, strip};
use rustc_middle::middle::region;

use super::types::{DropQuery, Drops, describe};

/// What a body does with its values that have destructors: what's
/// refused of it, and its momentary borrows, which destructors ask.
#[derive(Default)]
pub(in crate::lower) struct Facts {
    /// What this body does that isn't supported yet, each where and what,
    /// and whether it's of a closure that holds a value with a destructor,
    /// which a body lowered from its MIR is refused too.
    pub(super) problems: Vec<(Span, String, bool)>,
    /// Each borrow, by its call's `fun`, whose guard is held only while
    /// nothing can ask whether its cell is borrowed: checked, and not
    /// counted, nor dropped (ADR 0328).
    pub(in crate::lower) momentary: HashSet<ExprId>,
}

/// What a pattern takes of a value it's matched against.
#[derive(PartialEq)]
enum Taken {
    /// Nothing that has a destructor: it borrows, copies or ignores it.
    Nothing,
    /// All of it, into one variable.
    Whole,
    /// A part, which a partial move leaves the rest of.
    Part,
}

/// Find what's refused of a body, and its momentary borrows, before it's lowered.
pub(super) fn find_facts<'tcx>(cx: &DropQuery<'_, 'tcx>, thir: &Thir<'tcx>, body_owner: DefId) -> Facts {
    let mut finder = Finder {
        cx,
        thir,
        body_owner,
        ids: thir
            .exprs
            .iter_enumerated()
            .map(|(id, e)| (std::ptr::from_ref(e) as usize, id))
            .collect(),
        stack: Vec::new(),
        statements: Vec::new(),
        lets: HashMap::new(),
        owners: HashSet::new(),
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
    thir: &'a Thir<'tcx>,
    /// The function or closure whose body this is.
    body_owner: DefId,
    ids: HashMap<usize, ExprId>,
    /// The expressions being walked, outermost first.
    stack: Vec<ExprId>,
    /// The statements being walked, each its temporaries' scope and what's
    /// in it, a `let` with an `else` nothing.
    statements: Vec<(region::Scope, Option<ExprId>)>,
    /// Each `let` statement's value, and its pattern.
    lets: HashMap<ExprId, &'a Pat<'tcx>>,
    /// The variables that own one, bound by value.
    owners: HashSet<LocalVarId>,
    facts: Facts,
}

impl<'c, 'q, 'a, 'tcx> Finder<'c, 'q, 'a, 'tcx> {
    fn id(&self, e: &ThirExpr<'tcx>) -> ExprId {
        self.ids[&(std::ptr::from_ref(e) as usize)]
    }

    /// What `e`, the top of the walk, is used for: the expression it's in,
    /// past what only passes it on, and which of its parts `e` is.
    fn context(&self) -> (Option<ExprId>, ExprId) {
        let mut child = *self.stack.last().expect("an expression being walked");
        for &parent in self.stack.iter().rev().skip(1) {
            // A `let`'s value is its statement's, not the block's around it:
            // its pattern says what it takes.
            if self.lets.contains_key(&child) {
                return (None, child);
            }
            match self.thir[parent].kind {
                ExprKind::Scope { .. }
                | ExprKind::Use { .. }
                | ExprKind::ValueTypeAscription { .. }
                | ExprKind::PlaceTypeAscription { .. } => child = parent,
                _ => return (Some(parent), child),
            }
        }
        (None, child)
    }

    /// Whether a value made here has a destructor to run: `None` of an
    /// `Option` that could hold one doesn't, nor does a variant of an enum
    /// without a `Drop` of its own whose fields have none.
    fn holds_drops(&self, expr: &ThirExpr<'tcx>) -> bool {
        if !self.cx.has_drops(expr.ty) {
            return false;
        }
        match &expr.kind {
            ExprKind::Adt(adt) => {
                self.cx
                    .recognition
                    .tcx
                    .adt_destructor(adt.adt_def.did())
                    .is_some_and(|d| self.cx.runs_drop(d.did))
                    || adt.fields.iter().any(|f| self.cx.has_drops(self.thir[f.expr].ty))
                    || !matches!(adt.base, AdtExprBase::None)
            }
            _ => true,
        }
    }

    fn problem(&mut self, span: Span, what: &str) {
        self.facts.problems.push((span, what.to_string(), false));
    }

    /// A closure that holds a value with a destructor, refused where it is.
    fn closure_problem(&mut self, span: Span, what: &str) {
        self.facts.problems.push((span, what.to_string(), true));
    }

    /// A closure that holds a value with a destructor drops it through the
    /// variables it took, which JS sees only where they're seen: it's made
    /// for a `let` or a call, and written over nowhere. One its own body
    /// doesn't call once isn't given to a call by value, which may consume it.
    fn closure_checked(&mut self, expr: &ThirExpr<'tcx>) {
        match expr.kind {
            ExprKind::Closure(_) => match self.cx.drops(expr.ty) {
                Drops::Nothing => {}
                Drops::Unsupported(t, what) => self.problem(expr.span, &describe(t, what)),
                Drops::Runs => {
                    let (parent, child) = self.context();
                    let placed = match parent.map(|p| &self.thir[p].kind) {
                        None => self.lets.contains_key(&child),
                        Some(ExprKind::Call { args, .. }) => args.contains(&child),
                        _ => false,
                    };
                    if !placed {
                        self.closure_problem(expr.span, "a closure that holds a value with a destructor, made here");
                    }
                }
            },
            ExprKind::Call { ref args, ty, .. }
                if !self.drops_given(ty) && args.iter().any(|&a| self.borrowing_closure(self.thir[a].ty)) =>
            {
                self.closure_problem(
                    expr.span,
                    "a closure that holds a value with a destructor, given away without being called once",
                );
            }
            ExprKind::Assign { lhs, .. }
                if self.thir[lhs].ty.walk().any(|part| {
                    part.as_type()
                        .is_some_and(|t| matches!(t.kind(), ty::Closure(..)) && self.cx.drops(t) != Drops::Nothing)
                }) =>
            {
                self.closure_problem(expr.span, "assigning a closure that holds a value with a destructor");
            }
            _ => {}
        }
    }

    /// Whether a call of `callee`, a function's type, may drop a closure
    /// it's given that's called by `&` or `&mut`: std's, which drops it as
    /// it returns, or the crate's own function's, as its caller's drop says,
    /// each where its lowering says (ADR 0470). Not a binding's, which JS
    /// holds, nor another crate's.
    fn drops_given(&self, callee: Ty<'tcx>) -> bool {
        let tcx = self.cx.recognition.tcx;
        let Some((def_id, _)) = fn_def(callee) else {
            return false;
        };
        !crate::lower::bindings::is_binding(tcx, def_id)
            && (def_id.is_local() || crate::lower::recognition::is_std_crate(tcx, def_id))
    }

    /// Whether `ty` is a closure that holds a value with a destructor, which
    /// a call it's given by value may consume, though it isn't called once
    /// by its own body: what that drops, the call doesn't (ADR 0098).
    fn borrowing_closure(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Closure(_, args) if args.as_closure().kind() != ty::ClosureKind::FnOnce)
            && self.cx.has_drops(ty)
    }

    fn taken(&self, pat: &Pat<'tcx>) -> Taken {
        match &pat.kind {
            PatKind::Wild => Taken::Nothing,
            PatKind::Binding {
                mode: BindingMode(ByRef::No, _),
                subpattern: None,
                ty,
                ..
            } if self.cx.has_drops(*ty) => Taken::Whole,
            _ => {
                let mut part = false;
                pat.walk_always(|p| {
                    if let PatKind::Binding {
                        mode: BindingMode(ByRef::No, _),
                        ty,
                        ..
                    } = p.kind
                    {
                        part |= self.cx.has_drops(ty);
                    }
                });
                if part { Taken::Part } else { Taken::Nothing }
            }
        }
    }

    /// A use of an owner at the top of the walk: refused where it moves a
    /// part of it a way MIR's lowering doesn't follow.
    fn owner_used(&mut self, e: ExprId) {
        let (parent, child) = self.context();
        let follows =
            |finder: &Self, pat: &Pat<'tcx>| finder.taken(pat) != Taken::Part || finder.cx.pattern_paths(pat).is_some();
        let followed = match parent.map(|p| &self.thir[p].kind) {
            // `let (c, d) = b;` moves the parts it binds.
            None => self.lets.get(&child).is_none_or(|pat| follows(self, pat)),
            // `if let (Some(c), _) = b` moves the parts it binds, if it matches.
            Some(ExprKind::Let { expr, pat }) if *expr == child => follows(self, pat),
            // A field moved out, `consume(pair.a)`: a struct's or a tuple's.
            Some(ExprKind::Field { lhs, .. }) if *lhs == child => self.projection_follows(),
            // Each arm moves the parts its pattern binds, and none all of it.
            Some(ExprKind::Match { scrutinee, arms, .. }) if *scrutinee == child => {
                let taken: Vec<Taken> = arms.iter().map(|&a| self.taken(&self.thir[a].pattern)).collect();
                !taken.contains(&Taken::Part)
                    || arms.iter().zip(&taken).all(|(&arm, taken)| {
                        *taken != Taken::Whole && self.cx.pattern_paths(&self.thir[arm].pattern).is_some()
                    })
            }
            _ => true,
        };
        if !followed {
            self.problem(self.thir[e].span, "moving part of a value with a destructor");
        }
    }

    /// Whether a projection chain from the variable at the top of the walk,
    /// where it moves a field, moves it out of structs and tuples alone, not
    /// through what isn't one, as a `Box`.
    fn projection_follows(&self) -> bool {
        if !self.projection_moved() {
            return true;
        }
        let mut child = *self.stack.last().expect("the variable");
        for &parent in self.stack.iter().rev().skip(1) {
            match &self.thir[parent].kind {
                ExprKind::Scope { .. } | ExprKind::PlaceTypeAscription { .. } => {}
                ExprKind::Field { lhs, .. } if *lhs == child => {
                    let ty = self.thir[*lhs].ty;
                    if !matches!(ty.kind(), ty::Tuple(_)) && !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_struct()) {
                        return false;
                    }
                }
                _ => break,
            }
            child = parent;
        }
        true
    }

    /// Whether the field at the top of the walk, of a projection chain, is
    /// used as a value, which moves it, rather than borrowed or written.
    fn projection_moved(&self) -> bool {
        let mut child = *self.stack.last().expect("the variable");
        for &parent in self.stack.iter().rev().skip(1) {
            match &self.thir[parent].kind {
                ExprKind::Scope { .. } | ExprKind::PlaceTypeAscription { .. } => {}
                ExprKind::Field { lhs, .. } if *lhs == child => {}
                ExprKind::Borrow { arg, .. } | ExprKind::RawBorrow { arg, .. } if *arg == child => return false,
                ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } | ExprKind::Index { lhs, .. }
                    if *lhs == child =>
                {
                    return false;
                }
                _ => return self.cx.has_drops(self.thir[child].ty),
            }
            child = parent;
        }
        self.cx.has_drops(self.thir[child].ty)
    }

    /// Whether `pats`, the patterns a temporary is matched against, move
    /// each part they bind a way `pattern_paths` follows.
    fn follows<'p>(&self, mut pats: impl Iterator<Item = &'p Pat<'tcx>>) -> bool
    where
        'tcx: 'p,
    {
        pats.all(|pat| self.cx.pattern_paths(pat).is_some())
    }

    /// A value with a destructor, made here, at the top of the walk: one
    /// taken apart is refused where its parts move a way MIR's lowering
    /// doesn't follow; one borrowed, a guard, may be momentary.
    fn value_made(&mut self, e: ExprId) {
        let (parent, child) = self.context();
        let span = self.thir[e].span;
        let taken = |finder: &Self, pats: &mut dyn Iterator<Item = &Pat<'tcx>>| {
            pats.map(|p| finder.taken(p)).any(|t| t != Taken::Nothing)
        };
        let used_in_place = match parent.map(|p| &self.thir[p].kind) {
            // `let (a, _) = (x, y);` moves what it binds out of a temporary,
            // which drops the rest as the statement ends.
            None => {
                let Some(&pat) = self.lets.get(&child) else {
                    return;
                };
                if matches!(pat.kind, PatKind::Wild | PatKind::Binding { subpattern: None, .. }) {
                    return;
                }
                if !self.follows(once(pat)) {
                    self.problem(span, "taking apart a temporary with a destructor");
                    return;
                }
                true
            }
            Some(ExprKind::Let { expr, pat }) if *expr == child => {
                if taken(self, &mut once(&**pat)) && !self.follows(once(&**pat)) {
                    self.problem(span, "moving part of a temporary with a destructor");
                    return;
                }
                true
            }
            Some(ExprKind::Match { scrutinee, arms, .. }) if *scrutinee == child => {
                let patterns = arms.iter().map(|&a| &*self.thir[a].pattern);
                if taken(self, &mut patterns.clone()) && !self.follows(patterns) {
                    self.problem(span, "moving part of a temporary with a destructor");
                    return;
                }
                true
            }
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

    /// A guard made here, `c.borrow_mut()` say, that's held only while
    /// nothing can ask whether its cell is borrowed: its borrow is checked,
    /// and not counted (ADR 0328).
    fn momentary(&mut self, e: ExprId) {
        let recognition = &self.cx.recognition;
        if !recognition.is_guard(self.thir[e].ty) {
            return;
        }
        // The borrow: `c.borrow()`, or `m.lock()` of `m.lock().unwrap()`.
        let mut made = e;
        let fun = loop {
            let ExprKind::Call { fun, ref args, .. } = self.thir[strip(self.thir, made)].kind else {
                return;
            };
            match fn_def(self.thir[fun].ty).and_then(|(id, args)| recognition.classify(id, args)) {
                Some(Std::Borrow { .. }) => break fun,
                Some(Std::UnwrapOk) => made = args[0],
                _ => return,
            }
        };
        let tree = recognition.tcx.region_scope_tree(self.body_owner.expect_local());
        let Some(scope) = tree.temporary_scope(self.thir[e].temp_scope_id).temp_lifetime else {
            return;
        };
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
        let Some(extent) = extent else {
            return;
        };
        let mut quiet = Quiet {
            cx: self.cx,
            thir: self.thir,
            own: fun,
            quiet: true,
        };
        quiet.visit_expr(&self.thir[extent]);
        if quiet.quiet {
            self.facts.momentary.insert(fun);
        }
    }
}

/// Whether evaluating an expression can't ask whether a cell is borrowed,
/// but by `own`, a borrow's call (ADR 0328): it calls nothing that may run
/// code of the crate's, or a cell's that borrows it, and drops nothing but
/// guards, of what it makes or gives a call by value or `&mut`. A shared
/// borrow beside `own`, a shared one too, asks only whether it's mutably
/// borrowed, which `own` doesn't change.
struct Quiet<'c, 'q, 'a, 'tcx> {
    cx: &'c DropQuery<'q, 'tcx>,
    thir: &'a Thir<'tcx>,
    own: ExprId,
    quiet: bool,
}

impl<'c, 'q, 'a, 'tcx> Quiet<'c, 'q, 'a, 'tcx> {
    fn drops(&self, ty: Ty<'tcx>) -> bool {
        self.cx.drops_but_guards(ty)
    }
}

impl<'c, 'q, 'a, 'tcx> Visitor<'a, 'tcx> for Quiet<'c, 'q, 'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        if !self.quiet {
            return;
        }
        let asks = match expr.kind {
            ExprKind::Call { fun, ref args, .. } if fun != self.own => match fn_def(self.thir[fun].ty) {
                Some((id, generic_args)) => {
                    self.cx.recognition.may_ask_borrows(id, generic_args)
                        || args.iter().any(|&a| match *self.thir[a].ty.kind() {
                            ty::Ref(_, inner, ty::Mutability::Mut) => self.drops(inner),
                            ty::Ref(..) => false,
                            _ => self.drops(self.thir[a].ty),
                        })
                }
                None => true,
            },
            ExprKind::InlineAsm(_) | ExprKind::Yield { .. } | ExprKind::Become { .. } => true,
            _ => false,
        };
        if asks || (!is_place(&expr.kind) && self.drops(expr.ty)) {
            self.quiet = false;
            return;
        }
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
        self.statements.push(match stmt.kind {
            ThirStmt::Expr { scope, expr } => (scope, Some(expr)),
            ThirStmt::Let {
                init_scope,
                initializer,
                else_block,
                ..
            } => (init_scope, initializer.filter(|_| else_block.is_none())),
        });
        visit::walk_stmt(self, stmt);
        self.statements.pop();
    }

    fn visit_pat(&mut self, pat: &'a Pat<'tcx>) {
        if let PatKind::Binding {
            var,
            mode: BindingMode(ByRef::No, _),
            ty,
            ..
        } = pat.kind
        {
            match self.cx.drops(ty) {
                Drops::Nothing => {}
                Drops::Runs => {
                    self.owners.insert(var);
                }
                Drops::Unsupported(t, what) => self.problem(pat.span, &describe(t, what)),
            }
        }
        visit::walk_pat(self, pat);
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        let id = self.id(expr);
        self.stack.push(id);
        match expr.kind {
            ExprKind::VarRef { id: var } | ExprKind::UpvarRef { var_hir_id: var, .. } if self.owners.contains(&var) => {
                self.owner_used(id);
            }
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
                self.cx.dyn_drop_lost(target, from, to)
            } =>
            {
                self.problem(expr.span, "a `dyn` of a value with a destructor");
            }
            _ => {}
        }
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

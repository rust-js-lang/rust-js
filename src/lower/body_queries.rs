//! Read-only questions about a captured THIR body. No emission state, names,
//! dependencies or JavaScript: these answers cannot change lowering as a side effect.

use super::bindings::named_callback;
use super::fn_def;
use super::recognition::{SliceLength, StdItem, is_std_def, is_std_method, slice_length};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef, HirId, Mutability};
use rustc_middle::middle::region;
use rustc_middle::mir::{BinOp, BorrowKind, UnOp};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, LogicalOp, Pat, PatKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

/// The parts of a `for pat in head { body }` (ADR 0025).
pub(super) struct ForLoop<'a, 'tcx> {
    pub(super) head: ExprId,
    pub(super) pat: &'a Pat<'tcx>,
    pub(super) body: ExprId,
    /// The `loop` inside, which `break` and `continue` refer to.
    pub(super) scope: region::Scope,
    pub(super) hir_id: HirId,
}

pub(super) struct BodyQuery<'a, 'tcx> {
    pub(super) tcx: TyCtxt<'tcx>,
    pub(super) thir: &'a Thir<'tcx>,
}

impl<'a, 'tcx> BodyQuery<'a, 'tcx> {
    fn strip(&self, e: ExprId) -> ExprId {
        strip(self.thir, e)
    }

    /// The body of the `loop` a scope holds: its value, or, for one that
    /// never ends used as a value of another type, what `NeverToAny` holds.
    pub(super) fn scoped_loop(&self, value: ExprId) -> Option<ExprId> {
        match self.thir[value].kind {
            ExprKind::Loop { body } => Some(body),
            ExprKind::NeverToAny { source } => match self.thir[source].kind {
                ExprKind::Loop { body } => Some(body),
                _ => None,
            },
            _ => None,
        }
    }

    /// Recognize the `for` desugaring (ADR 0025):
    ///
    /// ```text
    /// match IntoIterator::into_iter(head) {
    ///     mut iter => loop {
    ///         match Iterator::next(&mut iter) { None => break, Some(pat) => body }
    ///     }
    /// }
    /// ```
    pub(super) fn as_for(&self, e: ExprId) -> Option<ForLoop<'a, 'tcx>> {
        let thir: &'a Thir<'tcx> = self.thir;
        let is_call_to = |e: ExprId, item: LangItem| match thir[strip(thir, e)].kind {
            ExprKind::Call { fun, ref args, .. } => {
                matches!(thir[strip(thir, fun)].ty.kind(), &ty::FnDef(d, _) if self.tcx.is_lang_item(d, item))
                    .then(|| args[0])
            }
            _ => None,
        };
        let ExprKind::Match {
            scrutinee, ref arms, ..
        } = thir[strip(thir, e)].kind
        else {
            return None;
        };
        let head = is_call_to(scrutinee, LangItem::IntoIterIntoIter)?;
        let [arm] = &arms[..] else { return None };
        let ExprKind::Scope {
            value,
            region_scope,
            hir_id,
        } = thir[thir[*arm].body].kind
        else {
            return None;
        };
        let ExprKind::Loop { body } = thir[value].kind else {
            return None;
        };
        let ExprKind::Block { block } = thir[strip(thir, body)].kind else {
            return None;
        };
        let ([stmt], None) = (&*thir[block].stmts, thir[block].expr) else {
            return None;
        };
        let thir::StmtKind::Expr { expr, .. } = thir[*stmt].kind else {
            return None;
        };
        let ExprKind::Match {
            scrutinee: next,
            ref arms,
            ..
        } = thir[strip(thir, expr)].kind
        else {
            return None;
        };
        is_call_to(next, LangItem::IteratorNext)?;
        let some = arms.iter().find_map(|&a| match &thir[a].pattern.kind {
            PatKind::Variant { subpatterns, .. } if subpatterns.len() == 1 => {
                Some((&subpatterns[0].pattern, thir[a].body))
            }
            _ => None,
        })?;
        Some(ForLoop {
            head,
            pat: some.0,
            body: some.1,
            scope: region_scope,
            hir_id,
        })
    }

    /// Recognize `.await`'s desugaring, and return what's awaited (ADR 0029):
    ///
    /// ```text
    /// match IntoFuture::into_future(e) {
    ///     mut __awaitee => loop { match Future::poll(..) { Ready(r) => break r, Pending => {} } yield }
    /// }
    /// ```
    pub(super) fn as_await(&self, e: ExprId) -> Option<ExprId> {
        let thir = self.thir;
        let ExprKind::Match {
            scrutinee, ref arms, ..
        } = thir[strip(thir, e)].kind
        else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, scrutinee)].kind else {
            return None;
        };
        let (into_future, _) = fn_def(thir[strip(thir, fun)].ty)?;
        let [arm] = &arms[..] else { return None };
        let mut body = strip(thir, thir[*arm].body);
        // Given to a generic `&T`, what's awaited is reborrowed, `&*loop {..}`:
        // rustc adjusts the arm, not the match.
        if let ExprKind::Borrow { arg, .. } = thir[body].kind
            && let ExprKind::Deref { arg } = thir[strip(thir, arg)].kind
        {
            body = strip(thir, arg);
        }
        let is_loop = matches!(thir[body].kind, ExprKind::Loop { .. });
        (self.tcx.is_lang_item(into_future, LangItem::IntoFutureIntoFuture) && is_loop).then(|| args[0])
    }

    /// Recognize `?`'s desugaring (ADR 0035), and return what's tried:
    ///
    /// ```text
    /// match Try::branch(e) { Continue(v) => v, Break(r) => return FromResidual::from_residual(r) }
    /// ```
    pub(super) fn as_question(&self, e: ExprId) -> Option<ExprId> {
        let thir = self.thir;
        let ExprKind::Match { scrutinee, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, scrutinee)].kind else {
            return None;
        };
        let (branch, _) = fn_def(thir[strip(thir, fun)].ty)?;
        self.tcx.is_lang_item(branch, LangItem::TryTraitBranch).then(|| args[0])
    }

    /// The variable a place starts from.
    pub(super) fn root_var(&self, e: ExprId) -> Option<LocalVarId> {
        match self.thir[self.strip(e)].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Deref { arg: lhs } => self.root_var(lhs),
            _ => None,
        }
    }

    /// A place as a variable and a path of fields, like `p.x` as `(p, [0])`.
    pub(super) fn place_path(&self, e: ExprId) -> Option<(LocalVarId, Vec<usize>)> {
        match self.thir[self.strip(e)].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some((id, Vec::new())),
            ExprKind::Field { lhs, name, .. } => {
                let (id, mut path) = self.place_path(lhs)?;
                path.push(name.as_usize());
                Some((id, path))
            }
            _ => None,
        }
    }
}

/// Context-independent facts collected once, before borrowck steals the body.
/// Type/representation facts depend on the current instantiation and stay elsewhere.
#[derive(Default)]
pub(super) struct BodyFacts {
    pub(super) uses: HashMap<LocalVarId, usize>,
    pub(super) mutably_borrowed: HashSet<LocalVarId>,
    pub(super) stepped: HashSet<LocalVarId>,
    /// What a borrow lends, `g` of `&mut g`, as a closure's call does: read
    /// there, it's not copied.
    pub(super) lent: HashSet<ExprId>,
    /// The variables set again that a pattern takes apart in place
    /// (`steady_subjects`), by their reads there.
    pub(super) steady: HashSet<ExprId>,
    /// The items of a `Vec` or a slice read where a condition shows their
    /// index in bounds (`known_in_bounds`): each `a[i]`, and the function
    /// of each `Index::index(&v, i)`.
    pub(super) in_bounds: HashSet<ExprId>,
    /// The closures a `let` names that only a hook's function is, each
    /// written there, named (`named_callbacks`).
    pub(super) named_callbacks: HashSet<LocalVarId>,
    /// The casts of an `f64` known to be a whole number in the integer's
    /// range, each the value as it is (`whole_casts`).
    pub(super) whole_casts: HashSet<ExprId>,
    /// The structs JSX gives a component as its props (`jsx_given_props`).
    pub(super) jsx_props: HashSet<ExprId>,
    /// Where each variable is changed (`changes`), and where each loop is.
    changed: HashMap<LocalVarId, Vec<Span>>,
    loops: Vec<Span>,
}

impl BodyFacts {
    /// Whether `var` may be changed after `at`: by a change written after
    /// it, or one in a loop around it, which runs again after it.
    /// Whether the body changes `var`, its own or what it captures.
    pub(super) fn changes(&self, var: LocalVarId) -> bool {
        self.changed.contains_key(&var)
    }

    /// Whether anything in `region` changes `var`: a loop's body, which
    /// Rust's borrows let nothing outside it change while it reads it (ADR 0313).
    pub(super) fn changed_in(&self, var: LocalVarId, region: Span) -> bool {
        let region = region.source_callsite();
        (self.changed.get(&var)).is_some_and(|changes| changes.iter().any(|change| region.contains(*change)))
    }

    pub(super) fn changed_after(&self, var: LocalVarId, at: Span) -> bool {
        let at = at.source_callsite();
        let changes = self.changed.get(&var).map_or(&[][..], Vec::as_slice);
        changes.iter().any(|change| change.lo() > at.lo())
            || (self.loops.iter())
                .filter(|l| l.contains(at))
                .any(|l| changes.iter().any(|change| l.contains(*change)))
    }

    pub(super) fn collect<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Self {
        let query = BodyQuery { tcx, thir };
        let mut facts = Self {
            stepped: stepped_locals(tcx, thir),
            steady: steady_subjects(tcx, thir),
            in_bounds: known_in_bounds(tcx, thir),
            whole_casts: whole_casts(tcx, thir),
            jsx_props: jsx_given_props(tcx, thir).0,
            changed: changes(thir),
            loops: thir
                .exprs
                .iter()
                .filter(|e| matches!(e.kind, ExprKind::Loop { .. }))
                .map(|e| e.span.source_callsite())
                .collect(),
            ..Self::default()
        };
        for expr in thir.exprs.iter() {
            match expr.kind {
                ExprKind::VarRef { id } => {
                    *facts.uses.entry(id).or_default() += 1;
                }
                ExprKind::Borrow { borrow_kind, arg } => {
                    facts.lent.insert(strip(thir, arg));
                    if matches!(borrow_kind, BorrowKind::Mut { .. })
                        && let Some(id) = query.root_var(arg)
                    {
                        facts.mutably_borrowed.insert(id);
                    }
                }
                _ => {}
            }
        }
        facts.named_callbacks = named_callbacks(tcx, thir, &facts.uses);
        facts
    }
}

/// The closures a `let` names whose one use is as the function a
/// `#[rust_js::named_callback]` hook is given: each written there, named,
/// `useEffect(function createBundler() { .. }, [])` (ADR 0297).
fn named_callbacks<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    uses: &HashMap<LocalVarId, usize>,
) -> HashSet<LocalVarId> {
    let closures: HashSet<LocalVarId> = thir
        .stmts
        .iter()
        .filter_map(|stmt| match stmt.kind {
            thir::StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                else_block: None,
                ..
            } => match (&pattern.kind, &thir[strip(thir, init)].kind) {
                (
                    &PatKind::Binding {
                        var,
                        mode: BindingMode(ByRef::No, Mutability::Not),
                        subpattern: None,
                        ..
                    },
                    ExprKind::Closure(_),
                ) => Some(var),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let mut found = HashSet::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::Call { fun, ref args, .. } = expr.kind
            && fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, _)| named_callback(tcx, def_id))
            && let Some(&first) = args.first()
            && let ExprKind::VarRef { id } = thir[strip(thir, first)].kind
            && closures.contains(&id)
            && uses.get(&id) == Some(&1)
        {
            found.insert(id);
        }
    }
    found
}

/// Where each variable is changed, as the body's order has it: set, lent as
/// `&mut`, or bound by a `ref mut`, in whole or in part. A variable of the
/// enclosing function a closure captures is the closure's too.
fn changes(thir: &Thir<'_>) -> HashMap<LocalVarId, Vec<Span>> {
    let at = |span: Span| span.source_callsite();
    let root = |mut e: ExprId| loop {
        match thir[strip(thir, e)].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => break Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => break None,
        }
    };
    let mut changes: HashMap<LocalVarId, Vec<Span>> = HashMap::new();
    let mut taken_apart: Vec<(ExprId, &Pat<'_>)> = Vec::new();
    for expr in thir.exprs.iter() {
        let changed = match expr.kind {
            ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => Some(lhs),
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg,
            }
            | ExprKind::RawBorrow {
                mutability: Mutability::Mut,
                arg,
            } => Some(arg),
            ExprKind::Let { expr, ref pat } => {
                taken_apart.push((expr, pat));
                None
            }
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => {
                taken_apart.extend(arms.iter().map(|&arm| (scrutinee, &*thir[arm].pattern)));
                None
            }
            _ => None,
        };
        if let Some(var) = changed.and_then(root) {
            changes.entry(var).or_default().push(at(expr.span));
        }
    }
    for stmt in thir.stmts.iter() {
        if let thir::StmtKind::Let {
            ref pattern,
            initializer: Some(init),
            ..
        } = stmt.kind
        {
            taken_apart.push((init, pattern));
        }
    }
    for (subject, pat) in taken_apart {
        let Some(var) = root(subject) else { continue };
        pat.walk_always(|p| {
            if let PatKind::Binding {
                mode: BindingMode(ByRef::Yes(_, Mutability::Mut), _),
                ..
            } = p.kind
            {
                changes.entry(var).or_default().push(at(p.span));
            }
        });
    }
    changes
}

/// The reads of a variable set again that an `if let` or a `match` takes
/// apart, `raw` of `if let Some(e) = raw`, where nothing changes it while what the pattern
/// binds is read, so that names it in place (ADR 0290). A change counts once
/// it may come after the pattern's test and before a read of what it bound:
/// not one earlier in the body, nor one in a loop the pattern is in too, but
/// any after a closure's read or a `Cell` lent, which read the variable
/// when they're called.
pub(super) fn steady_subjects<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let at = |span: Span| span.source_callsite();
    let query = BodyQuery { tcx, thir };
    // The variable a place is in, `e` of `e.message`.
    let root = |mut e: ExprId| loop {
        match thir[strip(thir, e)].kind {
            ExprKind::Field { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => break strip(thir, e),
        }
    };
    let changes = changes(thir);
    let mut later: HashSet<ExprId> = HashSet::new();
    let mut loops = Vec::new();
    let mut subjects: Vec<(ExprId, LocalVarId, Span, Vec<&Pat<'tcx>>)> = Vec::new();
    // A variable, or a field of one, whose test comes where it's read.
    let subject = |scrutinee: ExprId| {
        let scrutinee = strip(thir, scrutinee);
        let mut root = scrutinee;
        while let ExprKind::Field { lhs, .. } = thir[root].kind {
            root = strip(thir, lhs);
        }
        match thir[root].kind {
            ExprKind::VarRef { id } => Some((scrutinee, id, at(thir[scrutinee].span))),
            _ => None,
        }
    };
    for (id, expr) in thir.exprs.iter_enumerated() {
        match expr.kind {
            ExprKind::Borrow { arg, .. } => {
                // A `Cell` lent of a field is a handle on the field, which
                // reads its place when it's read (ADR 0288).
                if matches!(thir[arg].ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell)) {
                    later.insert(root(arg));
                }
            }
            ExprKind::Closure(ref closure) => {
                for &upvar in &closure.upvars {
                    let upvar = match thir[strip(thir, upvar)].kind {
                        ExprKind::Borrow { arg, .. } => arg,
                        _ => upvar,
                    };
                    later.insert(root(upvar));
                }
            }
            ExprKind::Loop { .. } => loops.push(at(expr.span)),
            ExprKind::Let {
                expr: scrutinee,
                ref pat,
            } => {
                if let Some((scrutinee, var, site)) = subject(scrutinee) {
                    subjects.push((scrutinee, var, site, vec![&**pat]));
                }
            }
            ExprKind::Match {
                scrutinee, ref arms, ..
            } if query.as_for(id).is_none() => {
                if let Some((scrutinee, var, site)) = subject(scrutinee) {
                    subjects.push((
                        scrutinee,
                        var,
                        site,
                        arms.iter().map(|&arm| &*thir[arm].pattern).collect(),
                    ));
                }
            }
            _ => {}
        }
    }
    // Where each variable is read, and whether it's read later too.
    let mut reads: HashMap<LocalVarId, Vec<(Span, bool)>> = HashMap::new();
    for (id, expr) in thir.exprs.iter_enumerated() {
        if let ExprKind::VarRef { id: var } = expr.kind {
            reads.entry(var).or_default().push((at(expr.span), later.contains(&id)));
        }
    }
    let mut steady = HashSet::new();
    for (scrutinee, var, site, pats) in subjects {
        let changes: Vec<Span> = changes
            .get(&var)
            .into_iter()
            .flatten()
            .copied()
            .filter(|change| change.lo() >= site.hi())
            .collect();
        let mut uses: Vec<(Span, bool)> = Vec::new();
        for pat in &pats {
            pat.walk_always(|p| {
                if let PatKind::Binding { var, .. } = p.kind {
                    uses.extend(reads.get(&var).into_iter().flatten().copied());
                }
            });
        }
        let changed_while_read = changes.iter().any(|change| {
            uses.iter().any(|&(read, later)| {
                later
                    || change.lo() < read.hi()
                    || loops
                        .iter()
                        .any(|l: &Span| l.contains(*change) && l.contains(read) && !l.contains(site))
            })
        });
        if !changed_while_read {
            steady.insert(scrutinee);
        }
    }
    steady
}

/// A `Vec` or a slice, by the variable it's in and the fields to it, and
/// what a condition shows of its length (ADR 0292).
#[derive(Clone, PartialEq)]
enum Length {
    /// It has at least this many items.
    AtLeast(Vec<usize>, LocalVarId, u128),
    /// This variable is an index of it.
    Above(Vec<usize>, LocalVarId, LocalVarId),
}

/// The items of a `Vec` or a slice whose index a condition shows in
/// bounds, so they're read as JS reads them, `xs[0]` (ADR 0292). Where a
/// condition holds: in its `if`'s branch, after an `if` whose other
/// branch leaves, and `i` in `for i in 0..xs.len()`. Only of a slice and an
/// index the body never changes, so what it showed still holds.
pub(super) fn known_in_bounds<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let at = |span: Span| span.source_callsite();
    let query = BodyQuery { tcx, thir };
    let changed = changes(thir);
    let fixed = |var: LocalVarId| !changed.contains_key(&var);
    // Whether nothing in `region` changes `var`: a loop's body, which Rust's
    // borrows let nothing outside it change while it reads it (ADR 0313).
    let unchanged_in = |var: LocalVarId, region: Span| {
        !(changed.get(&var)).is_some_and(|changes| changes.iter().any(|change| region.contains(*change)))
    };
    // The slice a place is, through references: `items` of `*items`, of a
    // variable `fixed` says may be read where it's known.
    let place_of = |mut e: ExprId, fixed: &dyn Fn(LocalVarId) -> bool| {
        let mut path = Vec::new();
        loop {
            e = strip(thir, e);
            match thir[e].kind {
                ExprKind::Borrow { arg, .. } => e = arg,
                ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(..)) || thir[arg].ty.is_box() => {
                    e = arg
                }
                ExprKind::Field { lhs, name, .. } => {
                    path.push(name.as_usize());
                    e = lhs;
                }
                ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if fixed(id) => {
                    path.reverse();
                    break Some((path, id));
                }
                _ => break None,
            }
        }
    };
    let slice = |e: ExprId| place_of(e, &fixed);
    let any_slice = |e: ExprId| place_of(e, &|_| true);
    let asks = |e: ExprId, of: SliceLength| match thir[strip(thir, e)].kind {
        ExprKind::Call { fun, ref args, .. } => fn_def(thir[strip(thir, fun)].ty)
            .filter(|&(def_id, generic_args)| slice_length(tcx, def_id, generic_args) == Some(of))
            .and_then(|_| slice(args[0])),
        _ => None,
    };
    let constant = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Literal { lit, neg: false } => match lit.node {
            rustc_ast::LitKind::Int(n, _) => Some(n.get()),
            _ => None,
        },
        _ => None,
    };
    let index_var = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if fixed(id) => Some(id),
        _ => None,
    };
    // What `cond` being `holds` shows.
    fn shown<'tcx>(
        e: ExprId,
        holds: bool,
        thir: &Thir<'tcx>,
        found: &mut Vec<Length>,
        asks: &dyn Fn(ExprId, SliceLength) -> Option<(Vec<usize>, LocalVarId)>,
        constant: &dyn Fn(ExprId) -> Option<u128>,
        index_var: &dyn Fn(ExprId) -> Option<LocalVarId>,
    ) {
        let on = |e, holds, found: &mut Vec<Length>| shown(e, holds, thir, found, asks, constant, index_var);
        match thir[strip(thir, e)].kind {
            ExprKind::Unary { op: UnOp::Not, arg } => on(arg, !holds, found),
            ExprKind::LogicalOp { op, lhs, rhs } if matches!(op, LogicalOp::And) == holds => {
                on(lhs, holds, found);
                on(rhs, holds, found);
            }
            _ if !holds && let Some((path, var)) = asks(e, SliceLength::IsEmpty) => {
                found.push(Length::AtLeast(path, var, 1));
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let op = match (op, holds) {
                    (op, true) => op,
                    (BinOp::Lt, false) => BinOp::Ge,
                    (BinOp::Le, false) => BinOp::Gt,
                    (BinOp::Gt, false) => BinOp::Le,
                    (BinOp::Ge, false) => BinOp::Lt,
                    (BinOp::Eq, false) => BinOp::Ne,
                    (BinOp::Ne, false) => BinOp::Eq,
                    _ => return,
                };
                // The length on the left: `k < xs.len()` is `xs.len() > k`.
                let (op, len, other) = match (asks(lhs, SliceLength::Len), asks(rhs, SliceLength::Len)) {
                    (Some(len), _) => (op, len, rhs),
                    (None, Some(len)) => match op {
                        BinOp::Lt => (BinOp::Gt, len, lhs),
                        BinOp::Le => (BinOp::Ge, len, lhs),
                        BinOp::Gt => (BinOp::Lt, len, lhs),
                        BinOp::Ge => (BinOp::Le, len, lhs),
                        op => (op, len, lhs),
                    },
                    _ => return,
                };
                let (path, var) = len;
                if let Some(k) = constant(other) {
                    let least = match op {
                        BinOp::Gt => k + 1,
                        BinOp::Ge | BinOp::Eq => k,
                        BinOp::Ne if k == 0 => 1,
                        _ => return,
                    };
                    found.push(Length::AtLeast(path, var, least));
                } else if op == BinOp::Gt
                    && let Some(index) = index_var(other)
                {
                    found.push(Length::Above(path, var, index));
                }
            }
            _ => {}
        }
    }
    let shows = |cond: ExprId, holds: bool| {
        let mut found = Vec::new();
        shown(cond, holds, thir, &mut found, &asks, &constant, &index_var);
        found
    };
    // A branch that leaves, `{ return 0; }`, which rustc types `!`.
    let leaves = |e: ExprId| thir[strip(thir, e)].ty.is_never();
    // Where each condition holds, and what it shows there.
    let mut known: Vec<(Span, Length)> = Vec::new();
    for (id, expr) in thir.exprs.iter_enumerated() {
        match expr.kind {
            ExprKind::If {
                cond, then, else_opt, ..
            } => {
                known.extend(shows(cond, true).into_iter().map(|l| (at(thir[then].span), l)));
                if let Some(other) = else_opt {
                    known.extend(shows(cond, false).into_iter().map(|l| (at(thir[other].span), l)));
                }
            }
            ExprKind::Block { block } => {
                let block = &thir[block];
                for &stmt in &block.stmts {
                    let thir::StmtKind::Expr { expr: stmt, .. } = thir[stmt].kind else {
                        continue;
                    };
                    let ExprKind::If {
                        cond, then, else_opt, ..
                    } = thir[strip(thir, stmt)].kind
                    else {
                        continue;
                    };
                    // What follows `if c { return; }` in the block runs only
                    // where `c` didn't hold.
                    if else_opt.is_some() || !leaves(then) {
                        continue;
                    }
                    let rest = at(block.span).with_lo(at(thir[stmt].span).hi());
                    known.extend(shows(cond, false).into_iter().map(|l| (rest, l)));
                }
            }
            ExprKind::Match { .. } => {
                let Some(for_loop) = query.as_for(id) else { continue };
                let body = at(thir[for_loop.body].span);
                if let ExprKind::Adt(ref range) = thir[strip(thir, for_loop.head)].kind
                    && tcx.is_lang_item(range.adt_def.did(), LangItem::Range)
                    && let [_, end] = &range.fields[..]
                    && let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, end.expr)].kind
                    && fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                        slice_length(tcx, def_id, generic_args) == Some(SliceLength::Len)
                    })
                    && let Some((path, var)) = any_slice(args[0])
                    && unchanged_in(var, body)
                    && let PatKind::Binding {
                        var: index,
                        mode: BindingMode(ByRef::No, Mutability::Not),
                        subpattern: None,
                        ..
                    } = for_loop.pat.kind
                {
                    known.push((at(thir[for_loop.body].span), Length::Above(path, var, index)));
                }
            }
            _ => {}
        }
    }
    let mut in_bounds = HashSet::new();
    if known.is_empty() {
        return in_bounds;
    }
    for (id, expr) in thir.exprs.iter_enumerated() {
        let (site, items, index) = match expr.kind {
            ExprKind::Index { lhs, index } => (id, lhs, index),
            ExprKind::Call { fun, ref args, .. }
                if fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                    slice_length(tcx, def_id, generic_args) == Some(SliceLength::Index)
                }) =>
            {
                (fun, args[0], args[1])
            }
            _ => continue,
        };
        let Some((path, var)) = any_slice(items) else { continue };
        let here = at(expr.span);
        let shown_here = known.iter().filter(|(region, _)| region.contains(here)).map(|(_, l)| l);
        let ok = match (constant(index), index_var(index)) {
            (Some(k), _) => shown_here
                .into_iter()
                .any(|l| matches!(l, Length::AtLeast(p, v, least) if *p == path && *v == var && k < *least)),
            (None, Some(i)) => shown_here
                .into_iter()
                .any(|l| matches!(l, Length::Above(p, v, index) if *p == path && *v == var && *index == i)),
            _ => false,
        };
        if ok {
            in_bounds.insert(site);
        }
    }
    in_bounds
}

/// The locals a `#[rust_js::nullable]` field is bound to, by value and not
/// set again: `T | null` already, as JS has the field (ADR 0275).
pub(super) fn nullable_bindings<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<LocalVarId> {
    let mut found = HashSet::new();
    let mut bound = |pat: &Pat<'tcx>| {
        let (variant, subpatterns) = match (&pat.kind, pat.ty.kind()) {
            (PatKind::Leaf { subpatterns }, ty::Adt(adt, _)) => (adt.non_enum_variant(), subpatterns),
            (
                PatKind::Variant {
                    adt_def,
                    variant_index,
                    subpatterns,
                    ..
                },
                _,
            ) => (adt_def.variant(*variant_index), subpatterns),
            _ => return,
        };
        for field in subpatterns {
            if super::bindings::is_nullable(tcx, &variant.fields[field.field])
                && let PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, Mutability::Not),
                    subpattern: None,
                    ..
                } = field.pattern.kind
            {
                found.insert(var);
            }
        }
    };
    let params = thir.params.iter().filter_map(|param| param.pat.as_deref());
    let lets = thir.stmts.iter().filter_map(|stmt| match &stmt.kind {
        thir::StmtKind::Let { pattern, .. } => Some(&**pattern),
        thir::StmtKind::Expr { .. } => None,
    });
    let arms = thir.arms.iter().map(|arm| &*arm.pattern);
    for pat in params.chain(lets).chain(arms) {
        pat.walk_always(&mut bound);
    }
    found
}

/// Skip THIR's wrapper nodes that don't change meaning.
pub(super) fn strip(thir: &Thir<'_>, mut e: ExprId) -> ExprId {
    loop {
        match thir[e].kind {
            ExprKind::Scope { value: inner, .. }
            | ExprKind::Use { source: inner }
            | ExprKind::NeverToAny { source: inner }
            | ExprKind::ValueTypeAscription { source: inner, .. }
            | ExprKind::PlaceTypeAscription { source: inner, .. } => e = inner,
            _ => return e,
        }
    }
}

/// What `&mut it`, reborrowed or not, lends: `it`.
pub(super) fn lent(thir: &Thir<'_>, e: ExprId) -> ExprId {
    let e = strip(thir, e);
    match thir[e].kind {
        ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => lent(thir, arg),
        _ => e,
    }
}

/// The locals a body calls `next()` or `by_ref()` on, directly or through
/// `&mut`, or iterates by `&mut` in a `for` loop: the
/// ones that must know where they are (ADR 0071). So is one lent as a
/// `&mut dyn Iterator`, or to a function of the crate's that takes a `&mut`
/// to a generic iterator, which what it's lent to steps through.
pub(super) fn stepped_locals<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<LocalVarId> {
    let mut stepped = HashSet::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = expr.kind
            && expr.ty.is_ref()
            && super::recognition::is_dyn_iter(tcx, expr.ty)
            && let ExprKind::VarRef { id } = thir[lent(thir, source)].kind
        {
            stepped.insert(id);
        }
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let Some((def_id, _)) = fn_def(thir[fun].ty) else {
            continue;
        };
        // `by_ref()` lends it to what steps through it, a loop or a chain.
        let steps = is_std_method(tcx, def_id, StdItem::Iterator, "next")
            || is_std_method(tcx, def_id, StdItem::Iterator, "by_ref");
        let Some(&receiver) = args.first() else {
            continue;
        };
        if steps && let Some(id) = stepped_local(thir, receiver) {
            stepped.insert(id);
        }
        // `for x in &mut it` of an iterator, whose `into_iter()` is itself:
        // a `break` leaves the rest in it, for what reads it after.
        if is_std_def(tcx, tcx.parent(def_id), StdItem::IntoIterator)
            && expr.ty == thir[receiver].ty
            && matches!(
                thir[strip(thir, receiver)].kind,
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    ..
                }
            )
            && let Some(id) = stepped_local(thir, receiver)
        {
            stepped.insert(id);
        }
        if def_id.is_local() {
            let inputs = tcx
                .fn_sig(def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs();
            for (&arg, &input) in args.iter().zip(inputs) {
                if let ty::Ref(_, inner, rustc_ast::Mutability::Mut) = *input.kind()
                    && lends_iterator(tcx, def_id, inner)
                    && let Some(id) = stepped_local(thir, arg)
                {
                    stepped.insert(id);
                }
            }
        }
    }
    stepped
}

/// The local an iterator argument is: `it` of `it`, `&mut it` or `&mut *it`.
fn stepped_local(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    let mut e = strip(thir, e);
    while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[e].kind {
        e = strip(thir, arg);
    }
    match thir[e].kind {
        ExprKind::VarRef { id } => Some(id),
        _ => None,
    }
}

/// Is `ty`, of `callee`'s parameter `&mut ty`, a generic iterator: a type
/// parameter, an `impl Iterator` too, that `callee` bounds by `Iterator`?
fn lends_iterator<'tcx>(tcx: TyCtxt<'tcx>, callee: rustc_span::def_id::DefId, ty: ty::Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Param(_))
        && tcx
            .clauses_of(callee)
            .instantiate_identity(tcx)
            .into_iter()
            .any(|(clause, _)| {
                clause.skip_normalization().as_trait_clause().is_some_and(|tr| {
                    tr.self_ty().skip_binder() == ty && is_std_def(tcx, tr.def_id(), StdItem::Iterator)
                })
            })
}

/// Does `thir` read a static of `module`, or a `thread_local!`'s, whose
/// value, made where its module loads, may not be there yet where another's
/// initializer is (ADR 0096)? Another module's is an import, which JS makes
/// first (ADR 0234).
pub(super) fn reads_statics(tcx: TyCtxt<'_>, thir: &Thir<'_>, module: LocalModId) -> bool {
    let own = |def_id: DefId| {
        def_id
            .as_local()
            .is_some_and(|local| tcx.parent_module_from_def_id(local) == module)
    };
    thir.exprs.iter().any(|e| match e.kind {
        ExprKind::StaticRef { def_id, .. } | ExprKind::ThreadLocalRef(def_id) => own(def_id),
        ExprKind::NamedConst { def_id, .. } => {
            own(def_id)
                && def_id
                    .as_local()
                    .is_some_and(|local| super::analysis::is_thread_local(tcx, local))
        }
        _ => false,
    })
}

/// What a comparison of a variable with a constant shows: `x < 3`.
#[derive(Clone, Copy)]
struct Compared {
    var: LocalVarId,
    op: BinOp,
    with: f64,
}

/// The casts to an integer of an `f64` variable known there to be a whole
/// number in the integer's range, each the variable as it is (ADR 0302).
/// What it may hold is what each `let` and `=` gives it: a whole number, an
/// integer `as f64`, or what a `#[rust_js::position]` binding gives, a
/// position or -1; narrowed where a test of it holds, if nothing sets it
/// after the test.
pub(super) fn whole_casts<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let at = |span: Span| span.source_callsite();
    let pointer_bits = tcx.data_layout.pointer_size().bits();
    // The whole numbers an integer type holds, of 32 bits or fewer: a JS
    // number, not a BigInt.
    let int_range = |t: ty::Ty<'tcx>| {
        let (bits, signed) = match *t.kind() {
            ty::Int(i) => (i.bit_width().unwrap_or(pointer_bits), true),
            ty::Uint(u) => (u.bit_width().unwrap_or(pointer_bits), false),
            _ => return None,
        };
        if bits > 32 {
            return None;
        }
        Some(match signed {
            true => (-(2f64.powi(bits as i32 - 1)), 2f64.powi(bits as i32 - 1) - 1.0),
            false => (0.0, 2f64.powi(bits as i32) - 1.0),
        })
    };
    let var_of = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } if thir[e].ty.is_floating_point() => Some(id),
        _ => None,
    };
    let casts: Vec<(ExprId, LocalVarId, (f64, f64))> = thir
        .exprs
        .iter_enumerated()
        .filter_map(|(id, expr)| match expr.kind {
            ExprKind::Cast { source } => Some((id, var_of(source)?, int_range(expr.ty)?)),
            _ => None,
        })
        .collect();
    let mut found = HashSet::new();
    if casts.is_empty() {
        return found;
    }
    let constant = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Literal { lit, neg } => {
            let value = match lit.node {
                rustc_ast::LitKind::Float(text, _) => text.as_str().replace('_', "").parse::<f64>().ok()?,
                rustc_ast::LitKind::Int(n, _) => n.get() as f64,
                _ => return None,
            };
            Some(if neg { -value } else { value })
        }
        _ => None,
    };
    // What a value given to a variable may be, as a range of whole numbers.
    let given = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Literal { .. } => constant(e).filter(|v| v.fract() == 0.0).map(|v| (v, v)),
        ExprKind::Cast { source } => int_range(thir[source].ty),
        ExprKind::Call { fun, .. } => fn_def(thir[strip(thir, fun)].ty)
            .filter(|&(def_id, _)| super::bindings::position(tcx, def_id))
            .map(|_| (-1.0, 2f64.powi(32) - 2.0)),
        _ => None,
    };
    // Each variable's values, of a plain `let` and each `=` of it.
    let mut values: HashMap<LocalVarId, Option<(f64, f64)>> = HashMap::new();
    let mut bound: HashSet<LocalVarId> = HashSet::new();
    fn join(values: &mut HashMap<LocalVarId, Option<(f64, f64)>>, var: LocalVarId, range: Option<(f64, f64)>) {
        let entry = values.entry(var).or_insert(range);
        *entry = entry.zip(range).map(|((lo, hi), (a, b))| (lo.min(a), hi.max(b)));
    }
    for stmt in thir.stmts.iter() {
        if let thir::StmtKind::Let {
            ref pattern,
            initializer,
            else_block: None,
            ..
        } = stmt.kind
            && let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, _),
                subpattern: None,
                ..
            } = pattern.kind
            && pattern.ty.is_floating_point()
        {
            bound.insert(var);
            if let Some(init) = initializer {
                join(&mut values, var, given(init));
            }
        }
    }
    let mut sets: HashMap<LocalVarId, usize> = HashMap::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::Assign { lhs, rhs } = expr.kind
            && let ExprKind::VarRef { id } = thir[strip(thir, lhs)].kind
        {
            *sets.entry(id).or_default() += 1;
            join(&mut values, id, given(rhs));
        }
    }
    let changed = changes(thir);
    // A variable only its `let` and its `=`s give values, and those known.
    let known = |var: LocalVarId| {
        let changes = changed.get(&var).map_or(0, Vec::len);
        let only_set = bound.contains(&var) && changes == sets.get(&var).copied().unwrap_or(0);
        values.get(&var).copied().flatten().filter(|_| only_set)
    };
    // What `cond` being `holds` shows of a variable.
    fn shown(
        e: ExprId,
        holds: bool,
        thir: &Thir<'_>,
        compared: &dyn Fn(ExprId) -> Option<Compared>,
        found: &mut Vec<Compared>,
    ) {
        match thir[strip(thir, e)].kind {
            ExprKind::Unary { op: UnOp::Not, arg } => shown(arg, !holds, thir, compared, found),
            ExprKind::LogicalOp { op, lhs, rhs } if matches!(op, LogicalOp::And) == holds => {
                shown(lhs, holds, thir, compared, found);
                shown(rhs, holds, thir, compared, found);
            }
            _ => {
                if let Some(mut c) = compared(e) {
                    if !holds {
                        c.op = match c.op {
                            BinOp::Eq => BinOp::Ne,
                            BinOp::Ne => BinOp::Eq,
                            BinOp::Lt => BinOp::Ge,
                            BinOp::Le => BinOp::Gt,
                            BinOp::Gt => BinOp::Le,
                            BinOp::Ge => BinOp::Lt,
                            _ => return,
                        };
                    }
                    found.push(c);
                }
            }
        }
    }
    // `x < 3`, or `3 > x` as it.
    let compared = |e: ExprId| {
        let ExprKind::Binary { op, lhs, rhs } = thir[strip(thir, e)].kind else {
            return None;
        };
        let flipped = match op {
            BinOp::Lt => BinOp::Gt,
            BinOp::Le => BinOp::Ge,
            BinOp::Gt => BinOp::Lt,
            BinOp::Ge => BinOp::Le,
            BinOp::Eq | BinOp::Ne => op,
            _ => return None,
        };
        match (var_of(lhs), var_of(rhs)) {
            (Some(var), None) => Some(Compared {
                var,
                op,
                with: constant(rhs)?,
            }),
            (None, Some(var)) => Some(Compared {
                var,
                op: flipped,
                with: constant(lhs)?,
            }),
            _ => None,
        }
    };
    let shows = |cond: ExprId, holds: bool| {
        let mut found = Vec::new();
        shown(cond, holds, thir, &compared, &mut found);
        found
    };
    // Where each test holds, from where it's made, and what it shows.
    let mut tested: Vec<(Span, Span, Compared)> = Vec::new();
    for expr in thir.exprs.iter() {
        match expr.kind {
            ExprKind::If {
                cond, then, else_opt, ..
            } => {
                let from = at(thir[cond].span);
                tested.extend(shows(cond, true).into_iter().map(|c| (from, at(thir[then].span), c)));
                if let Some(other) = else_opt {
                    tested.extend(shows(cond, false).into_iter().map(|c| (from, at(thir[other].span), c)));
                }
            }
            // What follows `if c { panic!() }` in its block runs only where
            // `c` didn't hold.
            ExprKind::Block { block } => {
                let block = &thir[block];
                for &stmt in &block.stmts {
                    let thir::StmtKind::Expr { expr: stmt, .. } = thir[stmt].kind else {
                        continue;
                    };
                    let ExprKind::If {
                        cond,
                        then,
                        else_opt: None,
                        ..
                    } = thir[strip(thir, stmt)].kind
                    else {
                        continue;
                    };
                    if !thir[strip(thir, then)].ty.is_never() {
                        continue;
                    }
                    let (from, rest) = (at(thir[stmt].span), at(block.span).with_lo(at(thir[stmt].span).hi()));
                    tested.extend(shows(cond, false).into_iter().map(|c| (from, rest, c)));
                }
            }
            _ => {}
        }
    }
    for (cast, var, (tlo, thi)) in casts {
        let Some((mut lo, mut hi)) = known(var) else { continue };
        let here = at(thir[cast].span);
        // A test holds here if nothing sets the variable after it's made.
        let after = |from: Span| {
            changed
                .get(&var)
                .into_iter()
                .flatten()
                .any(|change| change.hi() > from.lo())
        };
        let holding: Vec<Compared> = tested
            .iter()
            .filter(|(from, region, c)| c.var == var && region.contains(here) && !after(*from))
            .map(|&(_, _, c)| c)
            .collect();
        for c in &holding {
            match c.op {
                BinOp::Ge => lo = lo.max(c.with.ceil()),
                BinOp::Gt => lo = lo.max(c.with.floor() + 1.0),
                BinOp::Le => hi = hi.min(c.with.floor()),
                BinOp::Lt => hi = hi.min(c.with.ceil() - 1.0),
                BinOp::Eq => {
                    lo = lo.max(c.with);
                    hi = hi.min(c.with);
                }
                _ => {}
            }
        }
        // `x != -1` of what's -1 or more is 0 or more.
        let mut narrowed = true;
        while narrowed {
            narrowed = false;
            for c in holding.iter().filter(|c| c.op == BinOp::Ne) {
                if c.with == lo {
                    lo += 1.0;
                    narrowed = true;
                } else if c.with == hi {
                    hi -= 1.0;
                    narrowed = true;
                }
            }
        }
        if tlo <= lo && hi <= thi {
            found.insert(cast);
        }
    }
    found
}

/// Each struct JSX gives a component as its props, `<*>`'s second, with
/// its base, `{..Default::default()}`, and its flattened fields' values,
/// made there as structs or defaults: JSX takes them apart (ADR 0213); and
/// what's a base of them, `..props.html`, where a flattened field read
/// whole is the object its parent is, `{...props}`.
pub(crate) fn jsx_given_props<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> (HashSet<ExprId>, HashSet<ExprId>) {
    let mut made = Vec::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::Call { fun, ref args, .. } = expr.kind
            && let ty::FnDef(def_id, _) = *thir[fun].ty.kind()
            && matches!(super::bindings::js_form(tcx, def_id), super::bindings::JsForm::Jsx(tag) if tag == "*")
            && let [_, given] = args[..]
        {
            made.push(strip(thir, given));
        }
    }
    let (mut props, mut bases) = (HashSet::new(), HashSet::new());
    while let Some(e) = made.pop() {
        props.insert(e);
        if let ExprKind::Adt(ref adt) = thir[e].kind {
            if let thir::AdtExprBase::Base(ref fru) = adt.base {
                bases.insert(strip(thir, fru.base));
                made.push(strip(thir, fru.base));
            }
            for field in &adt.fields {
                if super::bindings::is_flatten_field(tcx, thir[e].ty, field.name.as_usize()) {
                    made.push(strip(thir, field.expr));
                }
            }
        }
    }
    (props, bases)
}

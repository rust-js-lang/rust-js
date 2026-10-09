//! Read-only questions about a captured THIR body. No emission state, names,
//! dependencies or JavaScript: these answers cannot change lowering as a side effect.

use super::fn_def;
use super::recognition::{StdItem, is_std_def, is_std_method};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef, HirId, Mutability};
use rustc_middle::middle::region;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, Pat, PatKind, Thir};
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
}

impl BodyFacts {
    pub(super) fn collect<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Self {
        let query = BodyQuery { tcx, thir };
        let mut facts = Self {
            stepped: stepped_locals(tcx, thir),
            steady: steady_subjects(tcx, thir),
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
        facts
    }
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
    // What changes each variable: set, or lent as `&mut`, in place or in part.
    // The variable a place is in, `e` of `e.message`.
    let root = |mut e: ExprId| loop {
        match thir[strip(thir, e)].kind {
            ExprKind::Field { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => break strip(thir, e),
        }
    };
    let changed_root = |mut e: ExprId| loop {
        match thir[strip(thir, e)].kind {
            ExprKind::VarRef { id } => break Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => break None,
        }
    };
    let mut changes: HashMap<LocalVarId, Vec<Span>> = HashMap::new();
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
            ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => {
                if let Some(var) = changed_root(lhs) {
                    changes.entry(var).or_default().push(at(expr.span));
                }
            }
            ExprKind::Borrow { borrow_kind, arg } => {
                if matches!(borrow_kind, BorrowKind::Mut { .. })
                    && let Some(var) = changed_root(arg)
                {
                    changes.entry(var).or_default().push(at(expr.span));
                }
                // A `Cell` lent of a field is a handle on the field, which
                // reads its place when it's read (ADR 0288).
                if matches!(thir[arg].ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell)) {
                    later.insert(root(arg));
                }
            }
            ExprKind::RawBorrow {
                mutability: Mutability::Mut,
                arg,
            } => {
                if let Some(var) = changed_root(arg) {
                    changes.entry(var).or_default().push(at(expr.span));
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
    // A `ref mut` binding changes the variable it takes apart, a `let`'s too.
    let lets = thir.stmts.iter().filter_map(|stmt| match stmt.kind {
        thir::StmtKind::Let {
            ref pattern,
            initializer: Some(init),
            ..
        } => subject(init).map(|(_, var, _)| (var, &**pattern)),
        _ => None,
    });
    let taken_apart = subjects
        .iter()
        .flat_map(|(_, var, _, pats)| pats.iter().map(|&pat| (*var, pat)));
    for (var, pat) in taken_apart.chain(lets).collect::<Vec<_>>() {
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
fn lent(thir: &Thir<'_>, e: ExprId) -> ExprId {
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

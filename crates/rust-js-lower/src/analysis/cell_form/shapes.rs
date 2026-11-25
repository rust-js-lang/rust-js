//! What `CellForm`'s proof reads of a body's THIR: where each borrow's
//! call is, the shape that holds its guard in the source, what a call's
//! receiver is, and ADR 0863's case N, a borrow that can't meet a guard
//! (#2490), split from `cell_form.rs` by concept.

use super::*;

/// Of a body's THIR, where each borrow's call is and what holds its guard:
/// its shape in the source, worker-02 agent's patterns' (#2490 comment 7132169613).
pub(super) struct Shapes<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    /// Each expression's parent.
    parents: IndexMap<ExprId, ExprId>,
    /// Each `let` statement's initializer, and whether it has an `else`, and
    /// whether its pattern binds nothing it reads, `_held`.
    lets: IndexMap<ExprId, (bool, bool)>,
    /// Each call, by where its callee is written: `None` where two are.
    calls: IndexMap<rustc_span::Span, Option<ExprId>>,
}

impl<'t, 'tcx> Shapes<'t, 'tcx> {
    pub(super) fn of(thir: &'t Thir<'tcx>) -> Self {
        let mut walk = Parents {
            thir,
            ids: (thir.exprs.iter_enumerated())
                .map(|(id, e)| (std::ptr::from_ref(e) as usize, id))
                .collect(),
            stack: Vec::new(),
            parents: IndexMap::default(),
        };
        if let Some(root) = thir.exprs.last_index() {
            walk.visit_expr(&thir[root]);
        }
        let mut lets = IndexMap::default();
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                initializer: Some(init),
                else_block,
                pattern,
                ..
            } = &stmt.kind
            {
                let unread = match &pattern.kind {
                    thir::PatKind::Wild => true,
                    thir::PatKind::Binding { name, .. } => name.as_str().starts_with('_'),
                    _ => false,
                };
                lets.insert(*init, (else_block.is_some(), unread));
            }
        }
        let mut calls: IndexMap<rustc_span::Span, Option<ExprId>> = IndexMap::default();
        for (id, e) in thir.exprs.iter_enumerated() {
            if let ExprKind::Call { fun, .. } = e.kind {
                calls
                    .entry(thir[fun].span)
                    .and_modify(|call| *call = None)
                    .or_insert(Some(id));
            }
        }
        Shapes {
            thir,
            parents: walk.parents,
            lets,
            calls,
        }
    }

    /// `e` through what reads a place's value as it is: a borrow, a deref,
    /// a std handle's `Deref::deref`, an `Rc`'s, a `Ref`'s, a `Vec`'s, and a
    /// slice's `iter()`, whose items are references into it. Not a type of
    /// the program's, whose `deref` may give another place each time.
    fn through_reads(&self, recognition: &Recognition<'_, 'tcx>, mut e: ExprId) -> ExprId {
        let (thir, tcx) = (self.thir, recognition.tcx);
        loop {
            e = crate::strip(thir, e);
            e = match thir[e].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => arg,
                ExprKind::Call { fun, ref args, .. }
                    if let [arg] = args[..]
                        && let Some((id, _)) = crate::fn_def(thir[fun].ty)
                        && ((tcx
                            .trait_of_assoc(id)
                            .is_some_and(|t| tcx.is_lang_item(t, rustc_hir::attrs::lang_items::LangItem::Deref))
                            && [StdItem::Rc, StdItem::RefCellRef, StdItem::Vec]
                                .into_iter()
                                .any(|item| recognition.is_std_type(thir[arg].ty.peel_refs(), item)))
                            || recognition.is_std_type(thir[e].ty, StdItem::SliceIter)) =>
                {
                    arg
                }
                _ => return e,
            };
        }
    }

    /// Whether the borrow whose callee is written at `theirs` can't meet the
    /// guard the one written at `own` makes (ADR 0863, amended, its case N):
    /// it borrows `v`, in the `then` of `if !v.borrow().contains(t)`, where
    /// `t` is an item of a `for` over what `own`'s guard holds, by
    /// reference, of a type equal to itself, a string's, an integer's or a
    /// `bool`'s, and `v` a cell's handle the body never sets again nor
    /// lends mutably. Were `v` `own`'s cell, `contains` would find `t`, so
    /// the `then` never runs; where it isn't, the two never meet.
    pub(super) fn unmet(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        own: rustc_span::Span,
        theirs: rustc_span::Span,
    ) -> bool {
        let thir = self.thir;
        let (Some(&Some(own)), Some(&Some(theirs))) = (self.calls.get(&own), self.calls.get(&theirs)) else {
            return false;
        };
        let ExprKind::Call { ref args, .. } = thir[theirs].kind else {
            return false;
        };
        let Some(&cell) = args.first() else { return false };
        let ExprKind::VarRef { id: var } = thir[self.through_reads(recognition, cell)].kind else {
            return false;
        };
        // The variables a `for` over what `own`'s guard holds binds each
        // item to, `for t in types.borrow().iter()`'s `t`.
        let query = crate::body_queries::BodyQuery {
            tcx: recognition.tcx,
            thir,
        };
        let items: IndexSet<thir::LocalVarId> = (thir.exprs.indices())
            .filter_map(|e| query.as_for(e))
            .filter(|for_loop| self.through_reads(recognition, for_loop.head) == own)
            .filter_map(|for_loop| match for_loop.pat.kind {
                thir::PatKind::Binding {
                    var,
                    mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, _),
                    subpattern: None,
                    ..
                } => Some(var),
                _ => None,
            })
            .collect();
        let mut child = theirs;
        while let Some(&parent) = self.parents.get(&child) {
            if let ExprKind::If { cond, then, .. } = thir[parent].kind
                && then == child
                && self.unmet_in(recognition, cond, &items) == Some(var)
            {
                return true;
            }
            child = parent;
        }
        false
    }

    /// The variable `v` of `!v.borrow().contains(t)`, `t` one of `items`, of
    /// a type equal to itself, `v` never set again nor lent mutably, in
    /// whole or in part, through its box too, `*v = other` (`changes`,
    /// #1000): a handle to one cell wherever it's read.
    fn unmet_in(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        cond: ExprId,
        items: &IndexSet<thir::LocalVarId>,
    ) -> Option<thir::LocalVarId> {
        let thir = self.thir;
        let ExprKind::Unary {
            op: mir::UnOp::Not,
            arg,
        } = thir[crate::strip(thir, cond)].kind
        else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[crate::strip(thir, arg)].kind else {
            return None;
        };
        let (id, generic_args) = crate::fn_def(thir[fun].ty)?;
        let (Some(Std::Comb(crate::combinators::Comb::Contains)), [slice, item]) =
            (recognition.classify(id, generic_args), &args[..])
        else {
            return None;
        };
        let ExprKind::VarRef { id: item } = thir[self.through_reads(recognition, *item)].kind else {
            return None;
        };
        let element = thir[*slice].ty.peel_refs().sequence_element_type(recognition.tcx);
        let reflexive = recognition.is_string_like(element) || element.is_integral() || element.is_bool();
        if !items.contains(&item) || !reflexive {
            return None;
        }
        let ExprKind::Call {
            fun: borrow,
            args: ref lent,
            ..
        } = thir[self.through_reads(recognition, *slice)].kind
        else {
            return None;
        };
        let (id, generic_args) = crate::fn_def(thir[borrow].ty)?;
        let (
            Some(Std::Borrow {
                mutable: false,
                lock: false,
            }),
            [cell],
        ) = (recognition.classify(id, generic_args), &lent[..])
        else {
            return None;
        };
        let ExprKind::VarRef { id: var } = thir[self.through_reads(recognition, *cell)].kind else {
            return None;
        };
        let changed = crate::body_queries::changes(thir).contains_key(&var);
        (!changed).then_some(var)
    }

    /// The call whose callee is written at `fun`, where one alone is: its
    /// THIR and its arguments.
    pub(super) fn call_at(&self, fun: rustc_span::Span) -> Option<(&'t Thir<'tcx>, &'t [ExprId])> {
        let call = (*self.calls.get(&fun)?)?;
        match &self.thir[call].kind {
            ExprKind::Call { args, .. } => Some((self.thir, &args[..])),
            _ => None,
        }
    }

    /// The shape of the borrow whose callee is written at `fun`: what holds
    /// its guard, a `let`, a `match`'s, a `for`'s, an `if let`'s or a `while
    /// let`'s scrutinee, a `return`; else, a temporary of its statement, how
    /// it's used there, a method's receiver, read, written, an argument or
    /// a `Ref::map`'s.
    pub(super) fn shape_at(
        &self,
        fun: rustc_span::Span,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let call = (thir.exprs.iter_enumerated())
            .find(|(_, e)| matches!(e.kind, ExprKind::Call { fun: f, .. } if thir[f].span == fun))?
            .0;
        Some(self.holder(call, classify).unwrap_or_else(|| self.used(call, classify)))
    }

    /// What holds the guard `call` makes past its statement, where one does:
    /// a `let` of the guard itself, or of a map of it, `Ref::map(c.borrow(),
    /// f)`; a scrutinee's temporary, which lives while its arms or its loop
    /// run, of a `match`, a `for`, an `if let` or a `while let`; a `let`-`else`'s;
    /// or a `return`. A `let` of what's made of it, `let n =
    /// c.borrow().len();`, drops it as the statement ends: none.
    fn holder(
        &self,
        call: ExprId,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let mut at = call;
        // Whether `at` is the guard, or a map of it, but for scopes.
        let mut itself = true;
        let mut mapped = false;
        loop {
            if let Some(&(with_else, unread)) = self.lets.get(&at) {
                return match (with_else, unread, itself) {
                    (true, ..) => Some("let-else"),
                    (_, _, false) => None,
                    _ if mapped => Some("map"),
                    (false, true, true) => Some("let-unread"),
                    (false, false, true) => Some("let"),
                };
            }
            let parent = *self.parents.get(&at)?;
            match &thir[parent].kind {
                ExprKind::Match { scrutinee, .. } if *scrutinee == at => {
                    return Some(if self.is_for(at) { "for" } else { "match" });
                }
                ExprKind::Let { expr, .. } if *expr == at => {
                    return Some(if self.in_loop(parent) { "while-let" } else { "if-let" });
                }
                ExprKind::Return { .. } if itself => return Some("returned"),
                // A statement's end, past which no temporary lives.
                ExprKind::Block { .. } | ExprKind::Loop { .. } | ExprKind::If { .. } | ExprKind::Closure(..) => {
                    return None;
                }
                ExprKind::Scope { .. } => {}
                ExprKind::Call { fun, .. }
                    if crate::fn_def(thir[*fun].ty)
                        .and_then(|(id, args)| classify(id, args))
                        .is_some_and(|std| matches!(std, Std::GuardMap { .. })) =>
                {
                    mapped = true;
                }
                _ => itself = false,
            }
            at = parent;
        }
    }

    /// How a temporary guard `call` makes is used in its statement.
    fn used(&self, call: ExprId, classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>) -> &'static str {
        let thir = self.thir;
        let mut at = call;
        loop {
            let Some(&parent) = self.parents.get(&at) else {
                return "tail";
            };
            match &thir[parent].kind {
                ExprKind::Scope { .. } | ExprKind::NeverToAny { .. } | ExprKind::Borrow { .. } => {}
                ExprKind::Deref { .. } => {
                    let mut place = parent;
                    while let Some(&up) = self.parents.get(&place)
                        && matches!(thir[up].kind, ExprKind::Scope { .. })
                    {
                        place = up;
                    }
                    let parent = place;
                    return match self.parents.get(&parent).map(|&p| &thir[p].kind) {
                        Some(ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. }) if *lhs == parent => {
                            "write"
                        }
                        Some(ExprKind::Borrow { .. } | ExprKind::RawBorrow { .. }) => self.lent_shape(parent),
                        _ => "read",
                    };
                }
                ExprKind::Call { fun, args, .. } => {
                    let std = crate::fn_def(thir[*fun].ty).and_then(|(id, args)| classify(id, args));
                    match std {
                        // The guard's own `Deref`, `*g` or a method's autoref.
                        Some(Std::GuardValue { .. }) => {}
                        Some(Std::GuardMap { .. }) => return "map",
                        _ => {
                            return if args.first() == Some(&at) {
                                "method"
                            } else {
                                "argument"
                            };
                        }
                    }
                }
                _ => return "other",
            }
            at = parent;
        }
    }

    /// Of `*guard` lent, `&*c.borrow()`: what takes the reference.
    fn lent_shape(&self, deref: ExprId) -> &'static str {
        let mut at = deref;
        while let Some(&parent) = self.parents.get(&at) {
            match &self.thir[parent].kind {
                ExprKind::Borrow { .. } | ExprKind::Scope { .. } => at = parent,
                ExprKind::Call { args, .. } => {
                    return if args.first() == Some(&at) {
                        "method"
                    } else {
                        "argument"
                    };
                }
                ExprKind::Match { .. } => return "match",
                _ => return "read",
            }
        }
        "read"
    }

    /// Whether `scrutinee`, a `match`'s, is a `for` loop's `IntoIterator::into_iter(..)`.
    fn is_for(&self, scrutinee: ExprId) -> bool {
        let mut at = scrutinee;
        loop {
            match &self.thir[at].kind {
                ExprKind::Scope { value, .. } => at = *value,
                ExprKind::Call { fn_span, .. } => return fn_span.is_desugaring(rustc_span::DesugaringKind::ForLoop),
                _ => return false,
            }
        }
    }

    /// Whether `e` is inside a `loop`, `while let`'s.
    fn in_loop(&self, mut e: ExprId) -> bool {
        while let Some(&parent) = self.parents.get(&e) {
            match &self.thir[parent].kind {
                ExprKind::Loop { .. } => return true,
                ExprKind::If { .. } | ExprKind::Scope { .. } | ExprKind::Block { .. } => e = parent,
                _ => return false,
            }
        }
        false
    }
}

/// Each THIR expression's parent, by a walk from the body's root.
struct Parents<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    ids: IndexMap<usize, ExprId>,
    stack: Vec<ExprId>,
    parents: IndexMap<ExprId, ExprId>,
}

impl<'t, 'tcx> Visitor<'t, 'tcx> for Parents<'t, 'tcx> {
    fn thir(&self) -> &'t Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'t thir::Expr<'tcx>) {
        let id = self.ids[&(std::ptr::from_ref(expr) as usize)];
        if let Some(&parent) = self.stack.last() {
            self.parents.insert(id, parent);
        }
        self.stack.push(id);
        visit::walk_expr(self, expr);
        self.stack.pop();
    }
}

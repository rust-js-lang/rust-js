//! Whether evaluating an expression can't ask whether a cell is borrowed
//! but by a borrow's own call of it, `Quiet` (ADRs 0328, 0580, 0863): what
//! `Finder::held_quietly` asks of a guard a `let` holds. Split unchanged
//! from `facts.rs` near its cap (#1334).

use super::*;

/// Whether evaluating an expression can't ask whether `cell` is borrowed,
/// but by `own`, a borrow's call of it (ADR 0328): it calls nothing that may
/// borrow it (ADR 0580), and drops nothing but guards, of what it makes or
/// gives a call by value or `&mut`. A shared borrow beside `own`, a shared
/// one too, asks only whether it's mutably borrowed, which `own` doesn't
/// change.
pub(super) struct Quiet<'c, 'q, 'a, 'tcx> {
    pub(super) cx: &'c DropQuery<'q, 'tcx>,
    pub(super) borrows: &'c CellBorrows<'tcx>,
    pub(super) thir: &'a Thir<'tcx>,
    /// The function or closure whose body this is.
    pub(super) owner: DefId,
    pub(super) own: ExprId,
    pub(super) cell: Borrow<'tcx>,
    pub(super) quiet: bool,
    /// The variables a `for` over what `own`'s guard holds binds each item
    /// to, `for t in types.borrow().iter()`'s `t`: a reference to an item
    /// of the cell `own` borrows, which stays there while it's held.
    pub(super) items: IndexSet<LocalVarId>,
    /// The variables, each a cell's handle, whose borrows can't meet
    /// `own`'s where they're asked: in the `then` of `if
    /// !v.borrow().contains(t)` of such an item `t` (`unmet_in`).
    pub(super) unmet: Vec<LocalVarId>,
    /// Whether only what rust-js sees asks, of a call's arguments that
    /// run before `own`'s guard, its receiver, is held (ADR 0863, amended).
    pub(super) seen_only: bool,
    /// Each expression's id, by where it is.
    pub(super) ids: &'c IndexMap<usize, ExprId>,
    /// What ended before `own`'s guard was made, which can't meet it
    /// (`Finder::ended_before`).
    pub(super) ended: &'c IndexSet<ExprId>,
}

impl<'c, 'q, 'a, 'tcx> Quiet<'c, 'q, 'a, 'tcx> {
    fn drops(&self, ty: Ty<'tcx>) -> bool {
        self.cx.drops_but_guards(self.cx.thir_ty(ty))
    }

    /// `e` through what reads a place's value as it is: a borrow, a deref,
    /// a std handle's `Deref::deref`, an `Rc`'s, a `Ref`'s, a `Vec`'s, and
    /// a slice's `iter()`, whose items are references into it. Not a type
    /// of the program's, whose `deref` may give another place each time.
    fn through_reads(&self, mut e: ExprId) -> ExprId {
        let recognition = &self.cx.recognition;
        let tcx = recognition.tcx;
        loop {
            e = strip(self.thir, e);
            e = match self.thir[e].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => arg,
                ExprKind::Call { fun, ref args, .. }
                    if let [arg] = args[..]
                        && let Some((id, _)) = fn_def(self.thir[fun].ty)
                        && ((tcx
                            .trait_of_assoc(id)
                            .is_some_and(|t| tcx.is_lang_item(t, rustc_hir::attrs::lang_items::LangItem::Deref))
                            && [StdItem::Rc, StdItem::RefCellRef, StdItem::Vec]
                                .into_iter()
                                .any(|item| recognition.is_std_type(self.thir[arg].ty.peel_refs(), item)))
                            || recognition.is_std_type(self.thir[e].ty, StdItem::SliceIter)) =>
                {
                    arg
                }
                _ => return e,
            };
        }
    }

    /// `items`: each `for` of the body whose head reads `own`'s guard's
    /// items by reference, its pattern's variable.
    pub(super) fn items_of_own(&self) -> IndexSet<LocalVarId> {
        let query = crate::lower::body_queries::BodyQuery {
            tcx: self.cx.recognition.tcx,
            thir: self.thir,
        };
        (self.thir.exprs.indices())
            .filter_map(|e| query.as_for(e))
            .filter(|for_loop| {
                matches!(self.thir[self.through_reads(for_loop.head)].kind, ExprKind::Call { fun, .. } if fun == self.own)
            })
            .filter_map(|for_loop| match for_loop.pat.kind {
                PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                } => Some(var),
                _ => None,
            })
            .collect()
    }

    /// The variable `v` of `!v.borrow().contains(t)`, `t` one of `items`,
    /// whose cell's borrows in the `if`'s `then` can't meet `own`'s
    /// (ADR 0863, amended): were `v` `own`'s cell, `t` would be one of its
    /// items, which `contains` finds, being equal to itself, a string's,
    /// an integer's, a `bool`'s or a `char`'s, so the `then` never runs.
    /// Of a `v` the body never sets again nor lends mutably, a handle to
    /// one cell wherever it's read.
    fn unmet_in(&self, cond: ExprId) -> Option<LocalVarId> {
        let recognition = &self.cx.recognition;
        let thir = self.thir;
        let ExprKind::Unary {
            op: rustc_middle::mir::UnOp::Not,
            arg,
        } = thir[strip(thir, cond)].kind
        else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind else {
            return None;
        };
        let (id, generic_args) = fn_def(thir[fun].ty)?;
        let (Some(Std::Comb(crate::lower::combinators::Comb::Contains)), [slice, item]) =
            (recognition.classify(id, generic_args), &args[..])
        else {
            return None;
        };
        let ExprKind::VarRef { id: item } = thir[self.through_reads(*item)].kind else {
            return None;
        };
        let element = thir[*slice].ty.peel_refs().sequence_element_type(recognition.tcx);
        let reflexive = recognition.is_string_like(element) || element.is_integral() || element.is_bool();
        if !self.items.contains(&item) || !reflexive {
            return None;
        }
        let ExprKind::Call {
            fun: borrow,
            args: ref lent,
            ..
        } = thir[self.through_reads(*slice)].kind
        else {
            return None;
        };
        let (id, generic_args) = fn_def(thir[borrow].ty)?;
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
        let ExprKind::VarRef { id: var } = thir[self.through_reads(*cell)].kind else {
            return None;
        };
        // Set or lent `&mut`, in whole or in part, through its box too, `*v
        // = other`, a box being the value it holds (`changes`, #1000).
        let changed = crate::lower::body_queries::changes(thir).contains_key(&var);
        (!changed).then_some(var)
    }
}

impl<'c, 'q, 'a, 'tcx> Quiet<'c, 'q, 'a, 'tcx> {
    /// Whether a call of `fun`, given `args`, may ask of `own`'s guard: what
    /// it runs may (of what rust-js sees, where `seen_only`), or it drops a
    /// value it's given with a destructor.
    fn call_asks(&self, fun: ExprId, args: &[ExprId]) -> bool {
        let drops = &|ty| self.drops(ty);
        let recognition = &self.cx.recognition;
        let ptr = self.thir[fun].ty;
        let receiver = args.first().map(|&receiver| (self.thir, receiver));
        let called = match fn_def(ptr) {
            // A promise's resolve, given what runs nothing (rd654).
            Some(called) if resolve_runs_nothing(recognition, self.thir, called, args) => false,
            Some((id, generic_args)) if self.seen_only => {
                (self.borrows).call_asks_seen(recognition, (self.owner, id, generic_args), receiver, self.cell)
            }
            Some((id, generic_args)) => {
                (self.borrows).call_asks(recognition, (self.owner, id, generic_args), receiver, self.cell, drops)
            }
            // A fn pointer's, of what it may be (ADR 0907).
            None if self.seen_only => self.borrows.pointer_call_asks_seen(recognition, ptr, self.cell),
            None => self.borrows.pointer_call_asks(recognition, ptr, self.cell, drops),
        };
        called
            || args.iter().any(|&a| match *self.thir[a].ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Mut) => self.drops(inner),
                ty::Ref(..) => false,
                _ => self.drops(self.thir[a].ty),
            })
    }

    /// `call`, a call of `method` whose receiver holds `own`'s guard, as
    /// it runs, not its arguments: whether it may ask of the guard.
    pub(super) fn visit_call(&mut self, call: ExprId, method: ExprId) {
        let ExprKind::Call { ref args, .. } = self.thir[call].kind else {
            self.quiet = false;
            return;
        };
        if self.quiet && self.call_asks(method, args) {
            self.quiet = false;
        }
    }
}

impl<'c, 'q, 'a, 'tcx> Visitor<'a, 'tcx> for Quiet<'c, 'q, 'a, 'tcx> {
    fn thir(&self) -> &'a Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'a ThirExpr<'tcx>) {
        if !self.quiet || ended(self.ids, self.ended, expr) {
            return;
        }
        // A borrow of a cell that can't be `own`'s here (`unmet_in`).
        if let ExprKind::If {
            cond, then, else_opt, ..
        } = expr.kind
            && let Some(var) = self.unmet_in(cond)
        {
            self.visit_expr(&self.thir[cond]);
            self.unmet.push(var);
            self.visit_expr(&self.thir[then]);
            self.unmet.pop();
            if let Some(otherwise) = else_opt {
                self.visit_expr(&self.thir[otherwise]);
            }
            return;
        }
        let unmet = |quiet: &Self, id: DefId, generic_args, args: &[ExprId]| {
            matches!(
                quiet.cx.recognition.classify(id, generic_args),
                Some(Std::Borrow { .. } | Std::TryBorrow { .. })
            ) && args.first().is_some_and(|&cell| {
                matches!(quiet.thir[quiet.through_reads(cell)].kind, ExprKind::VarRef { id } if quiet.unmet.contains(&id))
            })
        };
        let borrow = |quiet: &Self| {
            let call = *quiet.ids.get(&(std::ptr::from_ref(expr) as usize))?;
            (quiet.borrows).borrowed_in(&quiet.cx.recognition, (quiet.owner, quiet.thir), call)
        };
        let asks = match expr.kind {
            ExprKind::Call { fun, ref args, .. }
                if fun != self.own
                    && let Some((id, generic_args)) = fn_def(self.thir[fun].ty)
                    && unmet(self, id, generic_args, args) =>
            {
                false
            }
            // A borrow of a cell of a class of its own, by its flow, that
            // can't be `own`'s (rd482), and what the call runs by its bounds
            // while it's held, `replace_with`'s function (#687).
            ExprKind::Call { fun, .. }
                if fun != self.own
                    && let Some(theirs) = borrow(self)
                    && theirs.class.is_some()
                    && let Some(called) = fn_def(self.thir[fun].ty) =>
            {
                let drops = |ty| self.drops(ty);
                (self.borrows).cell_call_asks(&self.cx.recognition, called, theirs, self.cell, &drops)
            }
            ExprKind::Call { fun, ref args, .. } if fun != self.own => self.call_asks(fun, args),
            ExprKind::InlineAsm(_) | ExprKind::Yield { .. } | ExprKind::Become { .. } => true,
            _ => false,
        };
        // A write over a value with a destructor runs it.
        let writes_over = matches!(expr.kind, ExprKind::Assign { lhs, .. } if self.drops(self.thir[lhs].ty));
        if asks || writes_over || (!is_place(&expr.kind) && self.drops(expr.ty)) {
            self.quiet = false;
            return;
        }
        visit::walk_expr(self, expr);
    }

    /// A `let` of a value with a destructor, which runs where its block
    /// ends, even of a place moved into it, `let y = x;`.
    fn visit_stmt(&mut self, stmt: &'a rustc_middle::thir::Stmt<'tcx>) {
        if let ThirStmt::Let { ref pattern, .. } = stmt.kind
            && self.drops(pattern.ty)
        {
            self.quiet = false;
            return;
        }
        visit::walk_stmt(self, stmt);
    }
}

/// Whether `expr`, by its id among `ids`, is one of `ended`, which ran and
/// ended before a guard was made (`Finder::ended_before`).
pub(super) fn ended(ids: &IndexMap<usize, ExprId>, ended: &IndexSet<ExprId>, expr: &ThirExpr<'_>) -> bool {
    (ids.get(&(std::ptr::from_ref(expr) as usize))).is_some_and(|id| ended.contains(id))
}

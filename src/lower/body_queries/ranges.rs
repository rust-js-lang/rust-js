//! What a body's integers can be where it reads them (ADR 0576): each one's
//! range, and its bound by a slice's length, found by walking the body as
//! it runs, each condition narrowing them where it holds. Of these: which
//! index of a `Vec` or a slice is in bounds, read as JS reads it, `heap[i]`
//! (ADR 0292); and which `+`, `-` or `*` of a small integer can't leave
//! its type's range, written without its wrap, `(i + 1) * 2 - 1`.

use super::*;
use crate::lower::recognition::{VecView, vec_view};
use rustc_middle::middle::region;

/// A `Vec` or a slice, by the variable it's in and the fields to it.
type Slice = (Vec<usize>, LocalVarId);

/// An upper bound of an integer by a slice's length `L`, `⌊(c·L + b) /
/// 2^s⌋`: `L - 1` of an index `i < xs.len()`, `L / 2 - 1` of one below its
/// half.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Below {
    c: i128,
    b: i128,
    s: u32,
}

impl Below {
    /// The length itself.
    const LENGTH: Below = Below { c: 1, b: 0, s: 0 };

    /// This bound and `other` over one denominator.
    fn aligned(self, other: Below) -> (Below, Below) {
        let s = self.s.max(other.s);
        let up = |x: Below| Below {
            c: x.c << (s - x.s),
            b: x.b << (s - x.s),
            s,
        };
        (up(self), up(other))
    }

    fn plus(self, other: Below) -> Below {
        let (x, y) = self.aligned(other);
        Below {
            c: x.c + y.c,
            b: x.b + y.b,
            s: x.s,
        }
    }

    fn offset(self, k: i128) -> Below {
        Below {
            b: self.b + (k << self.s),
            ..self
        }
    }

    fn times(self, k: i128) -> Below {
        Below {
            c: self.c * k,
            b: self.b * k,
            ..self
        }
    }

    /// Whether it's below the length, `≤ L - 1`, whatever `L` is.
    fn within(self) -> bool {
        self.c <= 1 << self.s && self.b <= -(1 << self.s)
    }

    /// A bound at least either is.
    fn join(self, other: Below) -> Below {
        let (x, y) = self.aligned(other);
        Below {
            c: x.c.max(y.c),
            b: x.b.max(y.b),
            s: x.s,
        }
    }
}

/// What an integer can be: from `lo` to `hi`, and at most `below` by a
/// slice's length.
#[derive(Clone, PartialEq, Debug)]
struct Range {
    lo: i128,
    hi: i128,
    below: Option<(Slice, Below)>,
}

impl Range {
    fn of(lo: i128, hi: i128) -> Self {
        Range { lo, hi, below: None }
    }

    fn constant(&self) -> Option<i128> {
        (self.lo == self.hi).then_some(self.lo)
    }

    fn join(&self, other: &Range) -> Range {
        Range {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            below: match (&self.below, &other.below) {
                (Some((s, x)), Some((t, y))) if s == t => Some((s.clone(), x.join(*y))),
                _ => None,
            },
        }
    }
}

/// What's known where the body is: each integer variable's range, and the
/// least length of each slice.
#[derive(Clone, PartialEq, Default, Debug)]
struct Known {
    vars: HashMap<LocalVarId, Range>,
    lengths: HashMap<Slice, i128>,
}

impl Known {
    fn join(&self, other: &Known) -> Known {
        let vars = (self.vars.iter())
            .filter_map(|(v, r)| other.vars.get(v).map(|o| (*v, r.join(o))))
            .collect();
        let lengths = (self.lengths.iter())
            .filter_map(|(s, &n)| other.lengths.get(s).map(|&m| (s.clone(), n.min(m))))
            .collect();
        Known { vars, lengths }
    }

    /// What's known after `var` may have changed, as an integer or as what
    /// holds a slice: nothing of it.
    fn changed(&mut self, var: LocalVarId) {
        self.vars.remove(&var);
        self.lengths.retain(|(_, v), _| *v != var);
        for range in self.vars.values_mut() {
            if range.below.as_ref().is_some_and(|((_, v), _)| *v == var) {
                range.below = None;
            }
        }
    }
}

/// Either of two places the body goes on from, or the one it does.
fn either(a: Option<Known>, b: Option<Known>) -> Option<Known> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.join(&b)),
        (a, b) => a.or(b),
    }
}

/// Where a `break` or a `continue` of a scope goes, and what's known where
/// each is.
struct Target {
    scope: region::Scope,
    breaks: Option<Known>,
    continues: Option<Known>,
}

/// What a body's integer ranges show: each index known in bounds, its
/// `a[i]` or its `Index::index`'s function, and where each `+`, `-` and `*`
/// known to stay in its type's range is written.
#[derive(Default)]
pub(in crate::lower) struct Ranges {
    pub(in crate::lower) in_bounds: HashSet<ExprId>,
    pub(in crate::lower) exact: HashSet<Span>,
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    /// Each site's judgement: in bounds, or exact, each time it's reached.
    indexes: HashMap<ExprId, bool>,
    ops: HashMap<ExprId, bool>,
    /// Variables changed where the walk doesn't see it, by a raw pointer:
    /// never known.
    volatile: HashSet<LocalVarId>,
    changes: HashMap<LocalVarId, Vec<Span>>,
    targets: Vec<Target>,
    /// A jump the walk can't follow: nothing it found holds.
    lost: bool,
}

/// The ranges of a body's integers, and what they show (ADR 0576).
pub(in crate::lower) fn ranges<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, root: ExprId) -> Ranges {
    let mut walk = Walk {
        tcx,
        thir,
        indexes: HashMap::new(),
        ops: HashMap::new(),
        volatile: volatile(thir),
        changes: changes(thir),
        targets: Vec::new(),
        lost: false,
    };
    walk.expr(root, &mut Some(Known::default()));
    if walk.lost {
        return Ranges::default();
    }
    // One span may be of more than one operator, a macro's: exact where each is.
    let mut exact: HashMap<Span, bool> = HashMap::new();
    for (id, ok) in walk.ops {
        let span = thir[id].span;
        *exact.entry(span).or_insert(true) &= ok && !span.from_expansion();
    }
    Ranges {
        in_bounds: (walk.indexes.into_iter())
            .filter(|&(_, ok)| ok)
            .map(|(id, _)| id)
            .collect(),
        exact: (exact.into_iter())
            .filter(|&(_, ok)| ok)
            .map(|(span, _)| span)
            .collect(),
    }
}

/// The variables a raw pointer is made of, which it may change where the
/// walk doesn't see.
fn volatile(thir: &Thir<'_>) -> HashSet<LocalVarId> {
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::RawBorrow { arg, .. } => root_var(thir, arg),
            _ => None,
        })
        .collect()
}

/// The variable `e` reads, of the body or a closure's capture.
fn var_of(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
        _ => None,
    }
}

/// The variable a place is of, `v` of `v.items[0]` or `*v`.
fn root_var(thir: &Thir<'_>, mut e: ExprId) -> Option<LocalVarId> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => return Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => return None,
        }
    }
}

/// The range of an integer type rust-js writes as a JS number, of 32 bits
/// at most, whose operators wrap: `usize`'s is `u32`'s.
fn type_range(ty: ty::Ty<'_>) -> Option<(i128, i128)> {
    let bits = match ty.kind() {
        ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8) => 8,
        ty::Uint(ty::UintTy::U16) | ty::Int(ty::IntTy::I16) => 16,
        ty::Uint(ty::UintTy::U32 | ty::UintTy::Usize) | ty::Int(ty::IntTy::I32 | ty::IntTy::Isize) => 32,
        _ => return None,
    };
    Some(match ty.is_signed() {
        true => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
        false => (0, (1i128 << bits) - 1),
    })
}

/// Whether `e` reads without changing anything, so its value where a
/// condition is tested is its value after: `i < xs.len()`.
fn pure<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> bool {
    match thir[e].kind {
        ExprKind::Scope { value: inner, .. }
        | ExprKind::Use { source: inner }
        | ExprKind::Deref { arg: inner }
        | ExprKind::Field { lhs: inner, .. }
        | ExprKind::Unary { arg: inner, .. }
        | ExprKind::Cast { source: inner }
        | ExprKind::Borrow {
            borrow_kind: BorrowKind::Shared,
            arg: inner,
        } => pure(tcx, thir, inner),
        ExprKind::Binary { lhs, rhs, .. } => pure(tcx, thir, lhs) && pure(tcx, thir, rhs),
        ExprKind::VarRef { .. }
        | ExprKind::UpvarRef { .. }
        | ExprKind::Literal { .. }
        | ExprKind::NonHirLiteral { .. }
        | ExprKind::NamedConst { .. }
        | ExprKind::ConstParam { .. } => true,
        ExprKind::Call { fun, ref args, .. } => {
            fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                matches!(
                    slice_length(tcx, def_id, generic_args),
                    Some(SliceLength::Len | SliceLength::IsEmpty)
                )
            }) && args.iter().all(|&arg| pure(tcx, thir, arg))
        }
        _ => false,
    }
}

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// The slice a place is, through references: `heap` of `*heap`.
    fn slice(&self, mut e: ExprId) -> Option<Slice> {
        let thir = self.thir;
        let mut path = Vec::new();
        loop {
            e = strip(thir, e);
            match thir[e].kind {
                ExprKind::Borrow { arg, .. } => e = arg,
                ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(..)) || thir[arg].ty.is_box() => {
                    e = arg
                }
                // A `Vec`'s slice, `deref` or `deref_mut`, is as long.
                ExprKind::Call { fun, ref args, .. } if self.vec_view(fun) == Some(VecView::Slice) => e = args[0],
                ExprKind::Field { lhs, name, .. } => {
                    path.push(name.as_usize());
                    e = lhs;
                }
                ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if !self.volatile.contains(&id) => {
                    path.reverse();
                    return Some((path, id));
                }
                _ => return None,
            }
        }
    }

    /// What a call asks of a slice's length, and of which.
    fn asks(&self, e: ExprId) -> Option<(SliceLength, ExprId)> {
        let thir = self.thir;
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
        slice_length(self.tcx, def_id, generic_args).map(|asked| (asked, args[0]))
    }

    /// What `fun` makes of a `Vec`, as long as it is.
    fn vec_view(&self, fun: ExprId) -> Option<VecView> {
        let (def_id, generic_args) = fn_def(self.thir[strip(self.thir, fun)].ty)?;
        vec_view(self.tcx, def_id, generic_args)
    }

    /// What's known after `var` is changed.
    fn forget(&self, var: Option<LocalVarId>, known: &mut Option<Known>) {
        if let (Some(var), Some(k)) = (var, known.as_mut()) {
            k.changed(var);
        }
    }

    /// The range of `e` where `known` is, an integer's, walking it: what it
    /// changes forgotten, where it jumps followed, its sites judged. `None`
    /// of what isn't an integer rust-js knows, or where the body doesn't
    /// go on past it.
    fn expr(&mut self, e: ExprId, known: &mut Option<Known>) -> Option<Range> {
        let thir = self.thir;
        known.as_ref()?;
        let ty_range = type_range(thir[e].ty);
        let top = || ty_range.map(|(lo, hi)| Range::of(lo, hi));
        match thir[e].kind {
            ExprKind::Scope {
                region_scope, value, ..
            } => {
                self.targets.push(Target {
                    scope: region_scope,
                    breaks: None,
                    continues: None,
                });
                let value = self.expr(value, known);
                let target = self.targets.pop().expect("pushed");
                match target.breaks {
                    Some(broken) => {
                        *known = either(known.take(), Some(broken));
                        top()
                    }
                    None => value,
                }
            }
            ExprKind::Use { source }
            | ExprKind::NeverToAny { source }
            | ExprKind::ValueTypeAscription { source, .. }
            | ExprKind::PlaceTypeAscription { source, .. } => self.expr(source, known),
            ExprKind::Literal { lit, neg } => match lit.node {
                rustc_ast::LitKind::Int(n, _) if ty_range.is_some() => {
                    let n = n.get() as i128;
                    let n = if neg { -n } else { n };
                    Some(Range::of(n, n))
                }
                _ => top(),
            },
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => {
                (known.as_ref()).and_then(|k| k.vars.get(&id).cloned()).or_else(top)
            }
            ExprKind::Block { block } => self.block(block, known),
            ExprKind::If {
                cond, then, else_opt, ..
            } => {
                self.expr(cond, known);
                let mut yes = known.clone();
                let mut no = known.take();
                self.refine(cond, true, &mut yes);
                self.refine(cond, false, &mut no);
                let a = self.expr(then, &mut yes);
                let b = else_opt.and_then(|other| self.expr(other, &mut no));
                let value = match (&yes, &no) {
                    (Some(_), Some(_)) => a.zip(b).map(|(a, b)| a.join(&b)),
                    (Some(_), None) => a,
                    (None, Some(_)) => b,
                    (None, None) => None,
                };
                *known = either(yes, no);
                value.or_else(top)
            }
            ExprKind::Loop { body } => {
                self.walk_loop(body, None, known);
                None
            }
            ExprKind::Break { label, value } => {
                if let Some(value) = value {
                    self.expr(value, known);
                }
                self.jump(label, true, known);
                None
            }
            ExprKind::Continue { label } => {
                self.jump(label, false, known);
                None
            }
            ExprKind::Return { value } => {
                if let Some(value) = value {
                    self.expr(value, known);
                }
                *known = None;
                None
            }
            ExprKind::Become { value } => {
                self.expr(value, known);
                *known = None;
                None
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.expr(lhs, known);
                let r = self.expr(rhs, known);
                match (l, r, ty_range) {
                    (Some(l), Some(r), Some(bounds)) => Some(self.op(e, op, &l, &r, bounds)),
                    _ => top(),
                }
            }
            ExprKind::Assign { lhs, rhs } => {
                let value = self.expr(rhs, known);
                self.assign(lhs, value, known);
                None
            }
            ExprKind::AssignOp { op, lhs, rhs } => {
                let current = self.expr(lhs, known);
                let r = self.expr(rhs, known);
                let value = match (current, r, type_range(thir[lhs].ty)) {
                    (Some(l), Some(r), Some(bounds)) => Some(self.op(e, op.into(), &l, &r, bounds)),
                    _ => None,
                };
                self.assign(lhs, value, known);
                None
            }
            ExprKind::Call { fun, ref args, .. } => self.call(e, fun, args, known).or_else(top),
            ExprKind::Index { lhs, index } => {
                self.expr(lhs, known);
                let at = self.expr(index, known);
                self.judge_index(e, lhs, at, known);
                top()
            }
            ExprKind::Borrow { arg, borrow_kind } => {
                self.place(arg, known);
                // A `&mut` kept: what changes through it isn't seen.
                if matches!(borrow_kind, BorrowKind::Mut { .. }) {
                    self.forget(root_var(thir, arg), known);
                }
                None
            }
            ExprKind::RawBorrow { arg, .. } => {
                self.place(arg, known);
                None
            }
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => {
                if let Some(for_loop) = (BodyQuery { tcx: self.tcx, thir }).as_for(e) {
                    self.walk_for(for_loop, known);
                    return None;
                }
                self.expr(scrutinee, known);
                let before = known.take();
                for &arm in arms.iter() {
                    let mut here = before.clone();
                    self.forget_bound(&thir[arm].pattern, &mut here);
                    if let Some(guard) = thir[arm].guard {
                        self.expr(guard, &mut here);
                    }
                    self.expr(thir[arm].body, &mut here);
                    *known = either(known.take(), here);
                }
                top()
            }
            ExprKind::Let { expr, ref pat } => {
                self.expr(expr, known);
                self.forget_bound(pat, known);
                None
            }
            ExprKind::LogicalOp { op, lhs, rhs } => {
                // Its right only where its left lets it run.
                self.expr(lhs, known);
                let mut runs = known.clone();
                self.refine(lhs, matches!(op, LogicalOp::And), &mut runs);
                self.expr(rhs, &mut runs);
                if runs.is_some() {
                    *known = either(known.take(), runs);
                }
                None
            }
            ExprKind::Deref { arg }
            | ExprKind::Unary { arg, .. }
            | ExprKind::Cast { source: arg }
            | ExprKind::PointerCoercion { source: arg, .. }
            | ExprKind::Field { lhs: arg, .. }
            | ExprKind::Repeat { value: arg, .. }
            | ExprKind::Yield { value: arg }
            | ExprKind::ByUse { expr: arg, .. }
            | ExprKind::Reborrow { source: arg, .. }
            | ExprKind::PlaceUnwrapUnsafeBinder { source: arg }
            | ExprKind::ValueUnwrapUnsafeBinder { source: arg }
            | ExprKind::WrapUnsafeBinder { source: arg } => {
                self.expr(arg, known);
                top()
            }
            ExprKind::Tuple { ref fields } | ExprKind::Array { ref fields } => {
                for &field in fields.iter() {
                    self.expr(field, known);
                }
                None
            }
            ExprKind::Adt(ref adt) => {
                for field in adt.fields.iter() {
                    self.expr(field.expr, known);
                }
                if let thir::AdtExprBase::Base(ref base) = adt.base {
                    self.expr(base.base, known);
                }
                None
            }
            // What it captures by `&mut` it may change whenever it's called,
            // and, as Rust's borrows have it, nothing else reads meanwhile:
            // forgotten as it's made.
            ExprKind::Closure(ref closure) => {
                for &upvar in closure.upvars.iter() {
                    self.expr(upvar, known);
                }
                None
            }
            ExprKind::NonHirLiteral { .. }
            | ExprKind::ZstLiteral { .. }
            | ExprKind::NamedConst { .. }
            | ExprKind::ConstParam { .. }
            | ExprKind::ConstBlock { .. }
            | ExprKind::StaticRef { .. }
            | ExprKind::ThreadLocalRef(_) => top(),
            // A `#[loop_match]`'s jump, or assembly's: not followed.
            ExprKind::LoopMatch { .. } | ExprKind::ConstContinue { .. } | ExprKind::InlineAsm(_) => {
                self.lost = true;
                *known = None;
                None
            }
        }
    }

    /// A place walked where it's set or borrowed, not read: what its
    /// indexes compute.
    fn place(&mut self, e: ExprId, known: &mut Option<Known>) {
        if var_of(self.thir, e).is_none() {
            self.expr(e, known);
        }
    }

    /// A call: its arguments in order, then what it changes. A `&mut` it's
    /// given may change what it's of; a `Vec`'s length too, unless the call
    /// keeps it.
    fn call(&mut self, e: ExprId, fun: ExprId, args: &[ExprId], known: &mut Option<Known>) -> Option<Range> {
        let thir = self.thir;
        let lent = |arg: ExprId| match thir[strip(thir, arg)].kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg: place,
            } => Some(place),
            _ => None,
        };
        self.expr(fun, known);
        let mut ranges = Vec::new();
        for &arg in args {
            ranges.push(match lent(arg) {
                Some(place) => {
                    self.place(place, known);
                    None
                }
                None => self.expr(arg, known),
            });
        }
        let before = known.clone();
        let keeps = self.vec_view(fun).is_some();
        for &arg in args {
            if let Some(place) = lent(arg)
                && let (Some(var), Some(k)) = (root_var(thir, place), known.as_mut())
            {
                // A slice's `&mut` can't change its length.
                let slice = matches!(thir[strip(thir, arg)].ty.kind(), ty::Ref(_, inner, _) if inner.is_slice());
                match slice || keeps {
                    true => _ = k.vars.remove(&var),
                    false => k.changed(var),
                }
            }
        }
        match self.asks(e) {
            // Its indexes where it's called, the length as it was.
            Some((SliceLength::Index | SliceLength::Swap | SliceLength::SwapRemove, items)) => {
                for index in ranges.into_iter().skip(1) {
                    self.judge_index(fun, items, index, &before);
                }
                None
            }
            Some((SliceLength::Len, items)) => self.slice(items).map(|slice| {
                let least = (known.as_ref())
                    .and_then(|k| k.lengths.get(&slice).copied())
                    .unwrap_or(0);
                Range {
                    lo: least,
                    hi: u32::MAX as i128,
                    below: Some((slice, Below::LENGTH)),
                }
            }),
            _ => None,
        }
    }

    /// A `break` or a `continue` to `label`: what's known here goes there.
    fn jump(&mut self, label: region::Scope, breaks: bool, known: &mut Option<Known>) {
        let here = known.take();
        match self.targets.iter_mut().rev().find(|t| t.scope == label) {
            Some(target) => {
                let to = if breaks {
                    &mut target.breaks
                } else {
                    &mut target.continues
                };
                *to = either(to.take(), here);
            }
            None => self.lost = true,
        }
    }

    /// `lhs = value`: a variable's range, or what it's in forgotten.
    fn assign(&mut self, lhs: ExprId, value: Option<Range>, known: &mut Option<Known>) {
        let thir = self.thir;
        self.place(lhs, known);
        if let Some(var) = var_of(thir, lhs) {
            self.forget(Some(var), known);
            if let (Some(value), Some(k)) = (value, known.as_mut())
                && !self.volatile.contains(&var)
            {
                k.vars.insert(var, value);
            }
            return;
        }
        // An item of a slice set keeps its length.
        if !matches!(thir[strip(thir, lhs)].kind, ExprKind::Index { .. }) {
            self.forget(root_var(thir, lhs), known);
        }
    }

    fn block(&mut self, block: thir::BlockId, known: &mut Option<Known>) -> Option<Range> {
        let block = &self.thir[block];
        for &stmt in &block.stmts {
            self.stmt(stmt, known);
        }
        block.expr.and_then(|value| self.expr(value, known))
    }

    fn stmt(&mut self, stmt: thir::StmtId, known: &mut Option<Known>) {
        match self.thir[stmt].kind {
            thir::StmtKind::Expr { expr, .. } => {
                self.expr(expr, known);
            }
            thir::StmtKind::Let {
                ref pattern,
                initializer,
                else_block,
                ..
            } => {
                let value = initializer.and_then(|init| self.expr(init, known));
                if let Some(otherwise) = else_block {
                    // It never goes on: nothing it learns is known after.
                    let mut gone = known.clone();
                    self.block(otherwise, &mut gone);
                }
                self.forget_bound(pattern, known);
                if let PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                } = pattern.kind
                    && let (Some(value), Some(k)) = (value, known.as_mut())
                    && !self.volatile.contains(&var)
                {
                    k.vars.insert(var, value);
                }
            }
        }
    }

    /// Each variable `pattern` binds, unknown.
    fn forget_bound(&self, pattern: &Pat<'tcx>, known: &mut Option<Known>) {
        let Some(k) = known.as_mut() else { return };
        pattern.walk_always(|p| {
            if let PatKind::Binding { var, .. } = p.kind {
                k.changed(var);
            }
        });
    }

    /// A loop of `body`, its scope the innermost: walked from what's known
    /// at its start each time round, the join of what's known as it's
    /// entered and as each turn goes round again, until that holds. Past
    /// it, what's known where it's left: its scope's `break`s, joined in
    /// where the scope ends. `each`, a `for`'s variable, is set as each
    /// turn starts, and its iterator, run out, leaves where a turn starts.
    fn walk_loop(&mut self, body: ExprId, each: Option<(&Pat<'tcx>, Option<Range>)>, known: &mut Option<Known>) {
        let scope = self.targets.len().checked_sub(1);
        let entry = known.take();
        let mut head = entry.clone();
        for round in 0.. {
            let mut here = head.clone();
            if let Some((pat, value)) = &each {
                self.forget_bound(pat, &mut here);
                if let PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                } = pat.kind
                    && let (Some(value), Some(k)) = (value, here.as_mut())
                {
                    k.vars.insert(var, value.clone());
                }
            }
            self.expr(body, &mut here);
            let continued = scope.and_then(|at| self.targets[at].continues.take());
            let mut next = either(entry.clone(), either(here, continued));
            // Past a few rounds, what still changes is forgotten: what's
            // left holds.
            if round >= 3
                && let (Some(next), Some(head)) = (next.as_mut(), &head)
            {
                next.vars.retain(|v, r| head.vars.get(v) == Some(r));
                next.lengths.retain(|s, l| head.lengths.get(s) == Some(l));
            }
            if next == head {
                break;
            }
            // Each round past widening forgets something; one that never
            // settles is a bug, and nothing found is trusted.
            if round == 64 {
                self.lost = true;
                break;
            }
            head = next;
        }
        *known = match each {
            Some(_) => head,
            None => None,
        };
    }

    /// `for pat in head { body }`. Of a range, `0..xs.len()`, its variable is
    /// in it, and below the slice's length where the body never changes the
    /// slice: the range's end is its length as the loop began.
    fn walk_for(&mut self, for_loop: ForLoop<'a, 'tcx>, known: &mut Option<Known>) {
        let thir = self.thir;
        let each = match thir[strip(thir, for_loop.head)].kind {
            ExprKind::Adt(ref range) if self.tcx.is_lang_item(range.adt_def.did(), LangItem::Range) => {
                let [start, end] = &range.fields[..] else {
                    unreachable!("a range's two fields")
                };
                let start = self.expr(start.expr, known);
                let end = self.expr(end.expr, known);
                let body = thir[for_loop.body].span.source_callsite();
                let unchanged = |(path, var): Slice| {
                    let changed = (self.changes.get(&var)).is_some_and(|at| at.iter().any(|at| body.contains(*at)));
                    (!changed).then_some((path, var))
                };
                start.zip(end).map(|(start, end)| Range {
                    lo: start.lo,
                    hi: end.hi - 1,
                    below: (end.below).and_then(|(slice, b)| Some((unchanged(slice)?, b.offset(-1)))),
                })
            }
            _ => {
                self.expr(for_loop.head, known);
                None
            }
        };
        self.targets.push(Target {
            scope: for_loop.scope,
            breaks: None,
            continues: None,
        });
        self.walk_loop(for_loop.body, Some((for_loop.pat, each)), known);
        let target = self.targets.pop().expect("pushed");
        *known = either(known.take(), target.breaks);
    }

    /// What `cond` being `holds` shows, narrowing what's known: of a
    /// variable compared, `i < half`, and of a slice's length,
    /// `!xs.is_empty()`. Where it can't hold, nothing goes on.
    fn refine(&mut self, cond: ExprId, holds: bool, known: &mut Option<Known>) {
        let thir = self.thir;
        let e = strip(thir, cond);
        match thir[e].kind {
            ExprKind::Unary { op: UnOp::Not, arg } => self.refine(arg, !holds, known),
            ExprKind::LogicalOp { op, lhs, rhs } if matches!(op, LogicalOp::And) == holds => {
                self.refine(lhs, holds, known);
                self.refine(rhs, holds, known);
            }
            _ if let Some((SliceLength::IsEmpty, items)) = self.asks(e) => {
                if !holds
                    && let Some(slice) = self.slice(items)
                    && let Some(k) = known.as_mut()
                {
                    let least = k.lengths.entry(slice).or_insert(0);
                    *least = (*least).max(1);
                }
            }
            ExprKind::Binary { op, lhs, rhs } if pure(self.tcx, thir, lhs) && pure(self.tcx, thir, rhs) => {
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
                let flipped = match op {
                    BinOp::Lt => BinOp::Gt,
                    BinOp::Le => BinOp::Ge,
                    BinOp::Gt => BinOp::Lt,
                    BinOp::Ge => BinOp::Le,
                    op => op,
                };
                // Each side's value, which the condition's own walk judged.
                let judged = (std::mem::take(&mut self.indexes), std::mem::take(&mut self.ops));
                let mut quiet = known.clone();
                let l = self.expr(lhs, &mut quiet);
                let r = self.expr(rhs, &mut quiet);
                (self.indexes, self.ops) = judged;
                let (Some(l), Some(r)) = (l, r) else { return };
                self.narrow(lhs, op, &r, known);
                self.narrow(rhs, flipped, &l, known);
                // `xs.len() > k`: it's at least so long.
                for (side, op, other) in [(lhs, op, &r), (rhs, flipped, &l)] {
                    if let Some((SliceLength::Len, items)) = self.asks(side)
                        && let Some(slice) = self.slice(items)
                        && let Some(k) = known.as_mut()
                    {
                        let least = match op {
                            BinOp::Gt => other.lo + 1,
                            BinOp::Ge | BinOp::Eq => other.lo,
                            BinOp::Ne if other.constant() == Some(0) => 1,
                            _ => continue,
                        };
                        let at = k.lengths.entry(slice).or_insert(0);
                        *at = (*at).max(least);
                    }
                }
            }
            _ => {}
        }
    }

    /// A variable `side` that's `op` `other`, narrowed.
    fn narrow(&self, side: ExprId, op: BinOp, other: &Range, known: &mut Option<Known>) {
        let thir = self.thir;
        let Some(var) = var_of(thir, side) else { return };
        let Some((lo, hi)) = type_range(thir[side].ty) else {
            return;
        };
        if self.volatile.contains(&var) {
            return;
        }
        let Some(k) = known.as_mut() else { return };
        let current = k.vars.get(&var).cloned().unwrap_or(Range::of(lo, hi));
        let below = |by: i128| {
            (other.below.as_ref())
                .map(|(s, x)| (s.clone(), x.offset(by)))
                .or(current.below.clone())
        };
        let narrowed = match op {
            BinOp::Lt => Range {
                hi: current.hi.min(other.hi - 1),
                below: below(-1),
                ..current
            },
            BinOp::Le => Range {
                hi: current.hi.min(other.hi),
                below: below(0),
                ..current
            },
            BinOp::Eq => Range {
                lo: current.lo.max(other.lo),
                hi: current.hi.min(other.hi),
                below: below(0),
            },
            BinOp::Gt => Range {
                lo: current.lo.max(other.lo + 1),
                ..current
            },
            BinOp::Ge => Range {
                lo: current.lo.max(other.lo),
                ..current
            },
            BinOp::Ne if other.constant() == Some(current.lo) => Range {
                lo: current.lo + 1,
                ..current
            },
            BinOp::Ne if other.constant() == Some(current.hi) => Range {
                hi: current.hi - 1,
                ..current
            },
            _ => return,
        };
        if narrowed.lo > narrowed.hi {
            *known = None;
            return;
        }
        k.vars.insert(var, narrowed);
    }

    /// `l op r` of integers in `(lo, hi)`, at `site`: its range, and whether
    /// it stays in them, which its site's judgement keeps.
    fn op(&mut self, site: ExprId, op: BinOp, l: &Range, r: &Range, (lo, hi): (i128, i128)) -> Range {
        let bound = |r: &Range, by: &dyn Fn(Below) -> Below| r.below.as_ref().map(|(s, x)| (s.clone(), by(*x)));
        let made = match op {
            BinOp::Add => Some(Range {
                lo: l.lo + r.lo,
                hi: l.hi + r.hi,
                below: match (&l.below, &r.below) {
                    (Some((s, x)), Some((t, y))) if s == t => Some((s.clone(), x.plus(*y))),
                    (Some(_), _) => bound(l, &|x| x.offset(r.hi)),
                    (None, _) => bound(r, &|y| y.offset(l.hi)),
                },
            }),
            BinOp::Sub => Some(Range {
                lo: l.lo - r.hi,
                hi: l.hi - r.lo,
                below: bound(l, &|x| x.offset(-r.lo)),
            }),
            BinOp::Mul => {
                let products = [l.lo * r.lo, l.lo * r.hi, l.hi * r.lo, l.hi * r.hi];
                Some(Range {
                    lo: *products.iter().min().expect("four"),
                    hi: *products.iter().max().expect("four"),
                    below: match (r.constant(), l.constant()) {
                        (Some(k), _) if k >= 0 => bound(l, &|x| x.times(k)),
                        (_, Some(k)) if k >= 0 => bound(r, &|y| y.times(k)),
                        _ => None,
                    },
                })
            }
            BinOp::Shr if l.lo >= 0 => Some(match r.constant() {
                Some(k @ 0..32) => Range {
                    lo: l.lo >> k,
                    hi: l.hi >> k,
                    below: bound(l, &|x| Below { s: x.s + k as u32, ..x }).filter(|(_, x)| x.s <= 64),
                },
                _ => Range::of(0, l.hi),
            }),
            BinOp::BitAnd if l.lo >= 0 && r.lo >= 0 => Some(Range::of(0, l.hi.min(r.hi))),
            BinOp::Div if l.lo >= 0 && r.lo > 0 => Some(Range::of(l.lo / r.hi, l.hi / r.lo)),
            BinOp::Rem if l.lo >= 0 && r.lo > 0 => Some(Range::of(0, (r.hi - 1).min(l.hi))),
            _ => None,
        };
        let within = made.filter(|m| m.lo >= lo && m.hi <= hi);
        if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
            *self.ops.entry(site).or_insert(true) &= within.is_some();
        }
        // What wraps is anything in its type.
        within.unwrap_or(Range::of(lo, hi))
    }

    /// `items[index]` at `site`: in bounds where the index is below the
    /// slice's length each time it's reached.
    fn judge_index(&mut self, site: ExprId, items: ExprId, index: Option<Range>, known: &Option<Known>) {
        let Some(k) = known else { return };
        let ok = self.slice(items).zip(index).is_some_and(|(slice, index)| {
            index.lo >= 0
                && (index.below.as_ref().is_some_and(|(s, b)| *s == slice && b.within())
                    || k.lengths.get(&slice).is_some_and(|&least| index.hi < least))
        });
        *self.indexes.entry(site).or_insert(true) &= ok;
    }
}

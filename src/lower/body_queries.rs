//! Read-only questions about a captured THIR body. No emission state, names,
//! dependencies or JavaScript: these answers cannot change lowering as a side effect.

use super::bindings::named_callback;
use super::fn_def;
use super::recognition::{SliceLength, slice_length};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef, Mutability};

use rustc_middle::mir::{BinOp, BorrowKind, UnOp};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, LogicalOp, Pat, PatKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

mod ranges;

pub(super) use ranges::ranges;

/// The parts of a `for pat in head { body }` (ADR 0025).
pub(super) struct ForLoop<'a, 'tcx> {
    pub(super) head: ExprId,
    pub(super) pat: &'a Pat<'tcx>,
    pub(super) body: ExprId,
    /// The loop's scope, which its `break`s and `continue`s name.
    pub(super) scope: rustc_middle::middle::region::Scope,
}

pub(super) struct BodyQuery<'a, 'tcx> {
    pub(super) tcx: TyCtxt<'tcx>,
    pub(super) thir: &'a Thir<'tcx>,
}

impl<'a, 'tcx> BodyQuery<'a, 'tcx> {
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
            region_scope: scope,
            value,
            ..
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
            scope,
        })
    }
}

/// Context-independent facts collected once, before borrowck steals the body.
/// Type/representation facts depend on the current instantiation and stay elsewhere.
#[derive(Default)]
pub(super) struct BodyFacts {
    /// The items of a `Vec` or a slice whose index the body's ranges show
    /// in bounds (`ranges`): each `a[i]`, and the function of each
    /// `Index::index(&v, i)`.
    pub(super) in_bounds: HashSet<ExprId>,
    /// Where each `+`, `-` and `*` of a small integer is written that the
    /// body's ranges show never leaves its type's range (`ranges`).
    pub(super) exact: HashSet<Span>,
    /// The closures a `let` names that a hook's function is, each written
    /// there, named (`named_callbacks`).
    pub(super) named_callbacks: HashSet<LocalVarId>,
    /// The casts of an `f64` known to be a whole number in the integer's
    /// range, each the value as it is (`whole_casts`).
    pub(super) whole_casts: HashSet<ExprId>,
}

impl BodyFacts {
    /// Its facts but its ranges, which are the crate's (`ranges`).
    pub(super) fn collect<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Self {
        let mut facts = Self {
            whole_casts: whole_casts(tcx, thir),
            ..Self::default()
        };
        facts.named_callbacks = named_callbacks(tcx, thir);
        facts
    }
}

/// The closures a `let` names that are the function a
/// `#[rust_js::named_callback]` hook is given: each written there, named,
/// `useEffect(function createBundler() { .. }, [])` (ADR 0297).
fn named_callbacks<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<LocalVarId> {
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
        {
            found.insert(id);
        }
    }
    found
}

/// Where each variable is changed, as the body's order has it: set or lent
/// as `&mut`, in whole or in part. A variable of the
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
            _ => None,
        };
        if let Some(var) = changed.and_then(root) {
            changes.entry(var).or_default().push(at(expr.span));
        }
    }
    changes
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

/// A value a pattern tests, which a `const` may name (`constant_patterns`):
/// a number's or a `char`'s, by its type and bits; or the registry's
/// symbol's, `js::SymbolFor`'s, by its key (ADR 0634).
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum PatternValue<'tcx> {
    Bits(ty::Ty<'tcx>, u128),
    Key(String),
}

/// Each value a body's patterns test of a number, a `char` or the
/// registry's symbol, and the `const` they name it by:
/// `Some(IMMEDIATE_PRIORITY)` where every pattern of it is that constant,
/// `None` where one is written another way, a literal, another constant or
/// a trait's (ADRs 0590, 0634).
pub(super) fn constant_patterns<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
) -> HashMap<PatternValue<'tcx>, Option<DefId>> {
    let mut found: HashMap<PatternValue<'tcx>, Option<DefId>> = HashMap::new();
    let mut of = |pattern: &Pat<'tcx>| {
        pattern.walk_always(|pat| {
            // A constant of no generics: a trait's, `<u8 as Level>::HIGH`, is
            // of a type the pattern doesn't keep, so it's its value.
            let named = (pat.extra.as_ref().and_then(|extra| extra.expanded_const))
                .filter(|&d| tcx.generics_of(d).count() == 0);
            let value = match &pat.kind {
                PatKind::Constant { value } if pat.ty.is_integral() || pat.ty.is_char() => value
                    .valtree
                    .try_to_leaf()
                    .map(|leaf| PatternValue::Bits(pat.ty, leaf.to_bits_unchecked())),
                // Its key's pattern, a string's, inside it.
                PatKind::Leaf { subpatterns }
                    if matches!(pat.ty.kind(), ty::Adt(adt, _)
                    if super::recognition::is_symbol_for_def(tcx, adt.did())) =>
                {
                    match &subpatterns[..] {
                        [key] => pattern_text(&key.pattern).map(PatternValue::Key),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(value) = value {
                (found.entry(value))
                    .and_modify(|seen| *seen = seen.filter(|&d| Some(d) == named))
                    .or_insert(named);
            }
        });
    };
    thir.arms.iter().for_each(|arm| of(&arm.pattern));
    for expr in thir.exprs.iter() {
        if let ExprKind::Let { ref pat, .. } = expr.kind {
            of(pat);
        }
    }
    for stmt in thir.stmts.iter() {
        if let thir::StmtKind::Let { ref pattern, .. } = stmt.kind {
            of(pattern);
        }
    }
    found
}

/// Where each `js::SymbolFor::from_value(value)` is called, its `fn_span`,
/// whose value the body only tests against the registry's symbols: what a
/// `match`, a `matches!` or an `if let` compares, of patterns each `_` or a
/// `Some` of them, `Some(REACT_MEMO_TYPE | REACT_LAZY_TYPE)`, binding
/// nothing. There, no other value is one of them, so the value itself
/// answers each as its `Some` would (ADR 0634).
pub(super) fn registered_reads<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    let read = |mut e: ExprId| {
        while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
            e = inner;
        }
        match thir[e].kind {
            ExprKind::Call { fun, fn_span, .. } => match *thir[fun].ty.kind() {
                ty::FnDef(def_id, _) if super::recognition::is_symbol_for_read(tcx, def_id) => Some(fn_span),
                _ => None,
            },
            _ => None,
        }
    };
    (thir.exprs.iter())
        .filter_map(|e| match e.kind {
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => {
                let span = read(scrutinee)?;
                (arms.iter())
                    .all(|&arm| tests_registered(tcx, &thir[arm].pattern))
                    .then_some(span)
            }
            ExprKind::Let { expr, ref pat } => tests_registered(tcx, pat).then_some(read(expr)?),
            _ => None,
        })
        .collect()
}

/// Whether `pattern`, of an `Option<js::SymbolFor>`, is `_` or a `Some` of
/// the registry's symbols, binding nothing (`registered_reads`).
fn tests_registered<'tcx>(tcx: TyCtxt<'tcx>, pattern: &Pat<'tcx>) -> bool {
    let symbol = |pat: &Pat<'tcx>| {
        let mut each = true;
        pat.walk_always(|p| {
            each &= match &p.kind {
                PatKind::Or { .. } | PatKind::Deref { .. } => true,
                PatKind::Leaf { subpatterns } => {
                    matches!(p.ty.kind(), ty::Adt(adt, _) if super::recognition::is_symbol_for_def(tcx, adt.did()))
                        && matches!(&subpatterns[..], [key] if pattern_text(&key.pattern).is_some())
                }
                PatKind::Constant { .. } => p.ty.is_str(),
                _ => false,
            }
        });
        each
    };
    match &pattern.kind {
        PatKind::Wild => true,
        PatKind::Or { pats } => pats.iter().all(|p| tests_registered(tcx, p)),
        PatKind::Variant {
            adt_def, subpatterns, ..
        } if tcx.is_lang_item(adt_def.did(), LangItem::Option) => {
            matches!(&subpatterns[..], [some] if symbol(&some.pattern))
        }
        _ => false,
    }
}

/// The text a pattern of a string, `"react.memo"`, is.
fn pattern_text(pattern: &Pat<'_>) -> Option<String> {
    match &pattern.kind {
        PatKind::Deref { subpattern, .. } => pattern_text(subpattern),
        PatKind::Constant { value } if pattern.ty.is_str() => {
            let ty::ValTreeKind::Branch(bytes) = &*value.valtree else {
                return None;
            };
            let bytes = (bytes.iter())
                .map(|byte| Some(byte.try_to_value()?.try_to_leaf()?.to_u8()))
                .collect::<Option<Vec<u8>>>()?;
            String::from_utf8(bytes).ok()
        }
        _ => None,
    }
}

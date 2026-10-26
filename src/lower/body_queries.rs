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
pub(super) struct BodyFacts<'tcx> {
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
    /// The casts whose value is what they cast as it is: of an `f64` known
    /// to be a whole number in the integer's range (`whole_casts`), of an
    /// integer the body's ranges show in its target's (`ranges`), and of a
    /// signed one that's the count of text's `repeat` (`repeat_counts`).
    pub(super) kept_casts: HashSet<ExprId>,
    /// The function of each `OnceCell::set` whose cell the body's ranges
    /// show still empty there, every time it's reached (`ranges`, ADR 0847).
    pub(super) set_empty: HashSet<ExprId>,
    /// Where each read of a registered symbol is that's only tested against
    /// the registry's, `registered_reads`'s (ADR 0634).
    pub(super) registered_reads: HashSet<Span>,
    /// Where each `==` or `!=` of such a read and a `Some(..)` is,
    /// `registered_equalities`'s (ADR 0771).
    pub(super) registered_equalities: HashSet<Span>,
    /// Where each guard is that holds where its arm matched,
    /// `held_guards`'s.
    pub(super) held_guards: HashSet<Span>,
    /// Where each call is whose value is told present, `present_calls`'s.
    pub(super) present_calls: HashSet<Span>,
    /// Where each `rev()`, `enumerate()` or `map()` is whose adapter a std
    /// iterator's method steps where it's made, and where each such method
    /// is, `in_place_adapters`'s.
    pub(super) in_place_adapters: HashSet<Span>,
    pub(super) in_place_adapter_steps: HashSet<Span>,
    /// Each value the body's patterns test and the `const` they name it by,
    /// `constant_patterns`'s (ADRs 0590, 0634).
    pub(super) constant_patterns: HashMap<PatternValue<'tcx>, Option<DefId>>,
    /// The props given to the crate's components, and what's a base of
    /// them, `jsx_given_props`'s.
    pub(super) jsx_given: HashSet<ExprId>,
    pub(super) jsx_bases: HashSet<ExprId>,
}

impl<'tcx> BodyFacts<'tcx> {
    /// Its facts but its ranges, which are the crate's (`ranges`): each
    /// read of its own THIR once, before it's lowered, so a call it lowers
    /// looks its answer up, and a closure's are its own body's.
    pub(super) fn collect(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Self {
        let (jsx_given, jsx_bases) = jsx_given_props(tcx, thir);
        let (in_place_adapters, in_place_adapter_steps) = in_place_adapters(tcx, thir, None, &HashSet::new());
        Self {
            kept_casts: whole_casts(tcx, thir)
                .into_iter()
                .chain(repeat_counts(tcx, thir))
                .collect(),
            named_callbacks: named_callbacks(tcx, thir),
            registered_reads: registered_reads(tcx, thir),
            registered_equalities: registered_equalities(tcx, thir),
            held_guards: held_guards(tcx, thir),
            present_calls: present_calls(tcx, thir),
            in_place_adapters,
            in_place_adapter_steps,
            constant_patterns: constant_patterns(tcx, thir),
            jsx_given,
            jsx_bases,
            ..Self::default()
        }
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
/// What `e` gives, a block's of its value, `strip`ped.
fn tail(thir: &Thir<'_>, e: ExprId) -> ExprId {
    let e = strip(thir, e);
    match thir[e].kind {
        ExprKind::Block { block } if let Some(value) = thir[block].expr => tail(thir, value),
        _ => e,
    }
}

/// Each closure the crate gives `flat_map`, which steps what it gives
/// there as a JS iterator: what it gives is made in place
/// (`in_place_adapters`), its facts made again so. Before lowering, of the
/// crate's bodies.
pub(super) fn results_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &mut [super::Body<'tcx>],
    cloned: &HashSet<ty::Ty<'tcx>>,
) {
    let mut given = HashSet::new();
    for body in bodies.iter() {
        for e in body.thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = e.kind
                && matches!(*body.thir[fun].ty.kind(), ty::FnDef(id, _) if super::recognition::is_flat_map(tcx, id))
                && let Some(&f) = args.get(1)
                && let ExprKind::Closure(ref closure) = body.thir[strip(&body.thir, f)].kind
            {
                given.insert(closure.closure_id.to_def_id());
            }
        }
    }
    // And each that makes an adapter the crate clones apart, `chain()`'s
    // of a `cycle()`, whose parts its maker holds (ADR 0915).
    let clones_apart = |body: &super::Body<'tcx>| {
        body.thir
            .exprs
            .iter()
            .any(|e| matches!(e.kind, ExprKind::Call { .. }) && cloned_apart(tcx, cloned, e.ty))
    };
    for body in bodies.iter_mut() {
        let flat_mapped = given.contains(&body.def_id);
        if !flat_mapped && !clones_apart(body) {
            continue;
        }
        let stepped_result = flat_mapped.then_some(body.expr);
        let (adapters, steps) = in_place_adapters(tcx, &body.thir, stepped_result, cloned);
        body.facts.in_place_adapters = adapters;
        body.facts.in_place_adapter_steps = steps;
    }
}

/// Whether `ty` is an adapter of two parts the crate clones apart, its
/// `chain()`'s or `map()`'s cloned: made by a function of its parts, which
/// holds them (ADR 0525).
fn cloned_apart<'tcx>(tcx: TyCtxt<'tcx>, cloned: &HashSet<ty::Ty<'tcx>>, ty: ty::Ty<'tcx>) -> bool {
    super::recognition::cloned_parts(tcx, ty).is_some() && cloned.contains(&tcx.erase_and_anonymize_regions(ty))
}

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

/// The function `thir`'s body is a call of, given its parameters as they
/// are, in order: `ptr::eq(self, other)`, each reborrowed or made a raw
/// pointer, and its generic arguments (ADR 0864). `None` of a body that
/// does anything else.
pub(super) fn forwards_to<'tcx>(thir: &Thir<'tcx>) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    let mut e = strip(thir, thir.exprs.last_index()?);
    // Its block's value, or of a function of no result, its one statement.
    if let ExprKind::Block { block } = thir[e].kind {
        let block = &thir[block];
        e = match (block.stmts.as_ref(), block.expr) {
            ([], Some(value)) => strip(thir, value),
            ([only], None) => match thir[*only].kind {
                rustc_middle::thir::StmtKind::Expr { expr, .. } => strip(thir, expr),
                _ => return None,
            },
            _ => return None,
        };
    }
    let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
        return None;
    };
    // A parameter, as it's given: reborrowed, `&*self`, or made a raw
    // pointer, `self as *const _`.
    let param = |mut a: ExprId| -> Option<LocalVarId> {
        loop {
            a = strip(thir, a);
            match thir[a].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::RawBorrow { arg, .. } => match thir[strip(thir, arg)].kind {
                    ExprKind::Deref { arg } => a = arg,
                    _ => return None,
                },
                ExprKind::PointerCoercion { source, .. } | ExprKind::Cast { source } => a = source,
                ExprKind::VarRef { id } => return Some(id),
                _ => return None,
            }
        }
    };
    let params: Vec<LocalVarId> = (thir.params.iter())
        .map(|p| match p.pat.as_deref().map(|pat| &pat.kind) {
            Some(rustc_middle::thir::PatKind::Binding { var, .. }) => Some(*var),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let given: Vec<LocalVarId> = args.iter().map(|&a| param(a)).collect::<Option<_>>()?;
    (given == params).then(|| super::fn_def(thir[fun].ty)).flatten()
}

/// Is `thir`, a constant's initializer, written of other constants that
/// are read by their names (`by_name`), `A | B` or `EventPriority(NO_LANE)`,
/// and of no call of a function `by_name` says is compiled, whose body would
/// run where its module loads: so its JS is as it's written, not rustc's
/// value (ADR 0031, amended).
pub(super) fn written_of_constants(thir: &Thir<'_>, by_name: &dyn Fn(DefId) -> bool) -> bool {
    let calls_compiled = thir.exprs.iter().any(|e| match e.kind {
        ExprKind::Call { fun, .. } => super::fn_def(thir[fun].ty).is_none_or(|(def_id, _)| by_name(def_id)),
        ExprKind::Closure(_) => true,
        _ => false,
    });
    let names_constant =
        (thir.exprs.iter()).any(|e| matches!(e.kind, ExprKind::NamedConst { def_id, .. } if by_name(def_id)));
    names_constant && !calls_compiled
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

/// The casts to `usize` of a signed integer of 32 bits or fewer that are
/// the count of a `repeat` of text that isn't empty, `"  ".repeat(indent as
/// usize)`, each the integer as it is (ADR 0774, amended): a negative one is
/// `2^31` or more as a `usize`, past `isize::MAX` bytes of text that isn't
/// empty, which std panics of and JS's `repeat` throws its `RangeError` of,
/// as it does of a negative count.
fn repeat_counts<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
    let pointer_bits = tcx.data_layout.pointer_size().bits();
    // Text written, `"  "`, through each borrow of it.
    let text_of = |mut e: ExprId| loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Deref { arg } | ExprKind::Borrow { arg, .. } => e = arg,
            ExprKind::Literal { lit, .. } => match lit.node {
                rustc_ast::LitKind::Str(text, _) => return Some(text),
                _ => return None,
            },
            _ => return None,
        }
    };
    (thir.exprs.iter())
        .filter_map(|expr| {
            let ExprKind::Call { fun, ref args, .. } = expr.kind else {
                return None;
            };
            let (def_id, _) = fn_def(thir[strip(thir, fun)].ty)?;
            let &[text, count] = &args[..] else { return None };
            if !super::recognition::is_str_repeat(tcx, def_id) || text_of(text)?.as_str().is_empty() {
                return None;
            }
            let count = strip(thir, count);
            let ExprKind::Cast { source } = thir[count].kind else {
                return None;
            };
            let signed = matches!(*thir[source].ty.kind(), ty::Int(i) if i.bit_width().unwrap_or(pointer_bits) <= 32);
            (signed && thir[count].ty.is_usize()).then_some(count)
        })
        .collect()
}

/// The casts to an integer of an `f64` variable known there to be a whole
/// number in the integer's range, each the variable as it is (ADR 0302).
/// What it may hold is what each `let` and `=` gives it: a whole number, an
/// integer `as f64`, or what a `#[rust_js::position]` binding gives, a
/// position or -1; narrowed where a test of it holds, if nothing sets it
/// after the test.
fn whole_casts<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<ExprId> {
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
fn jsx_given_props<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> (HashSet<ExprId>, HashSet<ExprId>) {
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
fn constant_patterns<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashMap<PatternValue<'tcx>, Option<DefId>> {
    let mut found: HashMap<PatternValue<'tcx>, Option<DefId>> = HashMap::new();
    let mut of = |pattern: &Pat<'tcx>| {
        pattern.walk_always(|pat| {
            // A constant of no generics: a trait's, `<u8 as Level>::HIGH`, is
            // of a type the pattern doesn't keep, so it's its value.
            let named = (pat.extra.as_ref().and_then(|extra| extra.expanded_const))
                .filter(|&d| tcx.generics_of(d).count() == 0);
            if let Some(value) = pattern_value(tcx, pat) {
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

/// The `const`s `thir` names: read, `URGENT_LANES`, and tested, a
/// pattern's, `IMMEDIATE_PRIORITY =>`, which a test names too (ADR 0590).
pub(super) fn consts_named(thir: &Thir<'_>) -> HashSet<DefId> {
    let mut named: HashSet<DefId> = (thir.exprs.iter())
        .filter_map(|e| match e.kind {
            ExprKind::NamedConst { def_id, .. } => Some(def_id),
            _ => None,
        })
        .collect();
    let mut of = |pattern: &Pat<'_>| {
        pattern.walk_always(|pat| named.extend(pat.extra.as_ref().and_then(|extra| extra.expanded_const)));
    };
    thir.arms.iter().for_each(|arm| of(&arm.pattern));
    thir.params
        .iter()
        .filter_map(|param| param.pat.as_deref())
        .for_each(&mut of);
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
    named
}

/// Where `e` is a call of a binding that's its argument, `this`, giving an
/// untagged enum of a `Some(..)`, `js::classify(Some(value))`: its
/// `fn_span`. What it gives is there, never `undefined` nor `null`, so no
/// variant's test leaves those out (ADR 0676).
pub(super) fn given_present<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId) -> Option<Span> {
    let through = |mut e: ExprId| {
        while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
            e = inner;
        }
        e
    };
    e = through(e);
    let ExprKind::Call {
        fun, ref args, fn_span, ..
    } = thir[e].kind
    else {
        return None;
    };
    let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
        return None;
    };
    let identity = super::bindings::is_binding(tcx, def_id)
        && matches!(super::bindings::js_form(tcx, def_id), super::bindings::JsForm::This);
    let some = |arg: ExprId| {
        matches!(&thir[through(arg)].kind, ExprKind::Adt(adt)
            if tcx.is_lang_item(adt.adt_def.did(), LangItem::Option)
                && tcx.is_lang_item(adt.adt_def.variant(adt.variant_index).def_id, LangItem::OptionSome))
    };
    let untagged = super::untagged::untagged_adt(tcx, thir[e].ty).is_some();
    (identity && untagged && matches!(&args[..], [arg] if some(*arg))).then_some(fn_span)
}

/// The value of an integer `const` of no generic arguments, rustc's own,
/// which a body's ranges read as its literal's (ADR 0720).
pub(super) fn constant_integer<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<i128> {
    if !args.is_empty() {
        return None;
    }
    let scalar = tcx.const_eval_poly(def_id).ok()?.try_to_scalar_int()?;
    let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
    match ty.is_signed() {
        true => Some(scalar.to_int(scalar.size())),
        false => i128::try_from(scalar.to_uint(scalar.size())).ok(),
    }
}

/// Each call `given_present` says is of what's there: its `fn_span`.
fn present_calls<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    (thir.exprs.indices())
        .filter_map(|e| given_present(tcx, thir, e))
        .collect()
}

/// Each `rev()`, `enumerate()` or `map()`, `HELD_COMPILED`'s, whose adapter
/// is given, as what's called or an argument, where it's stepped as a JS
/// iterator, or a variable holds that it's stepped through alone, or by
/// another of them made in place of it: to another of them made in place,
/// to a std function or method not
/// compiled, `v.iter().rev().filter(f)`'s `filter`, an iterator trait's
/// method on it, `rfold`, a `for` loop's `into_iter`,
/// `zip(a, v.iter().rev())`, or to a type parameter of the crate's
/// function given a JS iterator, not as it is (ADR 0061): its `fn_span`,
/// and that call's. No one else holds it, so it's what the recognizers
/// make of it, `v.toReversed()`; one held, by a `let`, a field, a
/// parameter of its own type or one given as it is, or given to a compiled
/// std body, `fuse()`'s, is std's struct, `{ iter }` (ADR 0871).
fn in_place_adapters<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    stepped_result: Option<ExprId>,
    cloned: &HashSet<ty::Ty<'tcx>>,
) -> (HashSet<Span>, HashSet<Span>) {
    use super::recognition::{StdItem, std_item};
    let iterator_traits = [std_item(tcx, StdItem::Iterator), std_item(tcx, StdItem::IntoIterator)];
    // `cycle()`, which holds what it's given and clones it, as std's
    // `Cycle` does (ADR 0915).
    let cycle = super::recognition::trait_method(tcx, iterator_traits[0], "cycle");
    // Each call of a `rev()`, an `enumerate()` or a `map()` given, as what's called or
    // an argument, borrowed for it or not, to another call: its `fn_span`,
    // and the call's, its callee, and the argument's place.
    let mut given = Vec::new();
    for e in thir.exprs.indices() {
        let ExprKind::Call {
            fun,
            ref args,
            fn_span: at,
            ..
        } = thir[e].kind
        else {
            continue;
        };
        let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
            continue;
        };
        for (index, &arg) in args.iter().enumerate() {
            // Or borrowed for it, `rev().nth(n)`'s `&mut`, which it steps
            // there too.
            let arg = match thir[strip(thir, arg)].kind {
                ExprKind::Borrow { arg, .. } => strip(thir, arg),
                _ => strip(thir, arg),
            };
            let ExprKind::Call { fun, fn_span, .. } = thir[arg].kind else {
                continue;
            };
            // Not one held to be cloned, by `cycle()` or as a part of an
            // adapter cloned apart, which its maker holds: std's struct,
            // which its compiled `Clone` clones, but one cloned apart
            // itself, a `map()`'s, made by its own maker (ADRs 0525, 0915).
            let held_to_clone =
                (callee == cycle || cloned_apart(tcx, cloned, thir[e].ty)) && !cloned_apart(tcx, cloned, thir[arg].ty);
            if let ty::FnDef(maker, _) = *thir[fun].ty.kind()
                && super::recognition::makes_held_compiled(tcx, maker)
                && !held_to_clone
            {
                given.push((fn_span, at, callee, index, maker));
            }
        }
    }
    // `size_hint()`, which a JS iterator, once made, can't answer as Rust
    // does: of what the recognizers size where it's made, `rev()`'s, or of
    // std's struct, which knows it, a variable's or a `flatten()`'s (ADR
    // 0899).
    let size_hint = super::recognition::trait_method(tcx, iterator_traits[0], "size_hint");
    let steps_as_js = |callee: DefId, index: usize, in_place: &HashSet<Span>, at: Span, maker: DefId| {
        // Another of them, itself made in place, `enumerate().rev()`'s: in
        // place too.
        if super::recognition::makes_held_compiled(tcx, callee) {
            return in_place.contains(&at);
        }
        if callee == size_hint && !super::recognition::sized_in_place(tcx, maker) {
            return false;
        }
        let std = super::recognition::is_std_crate(tcx, callee) && !super::recognition::is_compiled_std(tcx, callee);
        // A trait's method called on it but an iterator trait's, which the
        // recognizers step it by, `next_back()` and `len()` too: `clone()`
        // or `fmt`, its own compiled impl's, of its struct.
        let on_it_by_its_impl = tcx
            .trait_of_assoc(callee)
            .is_some_and(|tr| !iterator_traits.contains(&tr) && !super::recognition::is_iterator_extension(tcx, tr));
        let inputs = tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        let input = inputs.get(index).map(|input| input.peel_refs().kind());
        let its_self = matches!(input, Some(ty::Param(param)) if super::recognition::is_trait_self(*param));
        let given_js = callee.is_local()
            && matches!(input, Some(ty::Param(param)) if !super::iterators::given_as_it_is(tcx, callee, *param));
        std && !(its_self && on_it_by_its_impl) || given_js
    };
    let (mut found, mut steps) = (HashSet::new(), HashSet::new());
    // What a closure gives that's stepped where it's given, `flat_map`'s:
    // made in place, as one given to it would be.
    if let Some(result) = stepped_result.map(|e| tail(thir, e))
        && let ExprKind::Call { fun, fn_span, .. } = thir[result].kind
        && matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if super::recognition::makes_held_compiled(tcx, id))
    {
        found.insert(fn_span);
    }
    // A variable a `let` binds one to, `let mut it = s.chars().enumerate()`,
    // read only where `Iterator`'s methods step it as a JS iterator, `next()`,
    // a `for` loop's `into_iter`, a `by_ref()` whose `&mut` they step, or
    // `drop`, never given anywhere else: nothing tells what it is, so it's
    // what the recognizers make of it too, `s.entries()`.
    let made: HashMap<LocalVarId, Span> = (thir.stmts.iter())
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
                        mode: BindingMode(ByRef::No, _),
                        subpattern: None,
                        ..
                    },
                    &ExprKind::Call { fun, fn_span, .. },
                ) if matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if super::recognition::makes_held_compiled(tcx, id)) => {
                    Some((var, fn_span))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();

    let mut reads: HashMap<LocalVarId, usize> = HashMap::new();
    for e in thir.exprs.iter() {
        if let ExprKind::VarRef { id } = e.kind
            && made.contains_key(&id)
        {
            *reads.entry(id).or_default() += 1;
        }
    }
    // Each call that steps what it's given as a JS iterator, by `Iterator`'s
    // methods but `by_ref()`, and `mem::drop`, which closes either: what
    // it's given.
    let mut js_steps: Vec<(ExprId, Span)> = Vec::new();
    let mut lent: Vec<(ExprId, ExprId)> = Vec::new();
    let mut makers: Vec<(ExprId, Span)> = Vec::new();
    for e in thir.exprs.indices() {
        let ExprKind::Call {
            fun,
            ref args,
            fn_span: at,
            ..
        } = thir[e].kind
        else {
            continue;
        };
        let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
            continue;
        };
        let by_iterator = tcx
            .trait_of_assoc(callee)
            .is_some_and(|tr| iterator_traits.contains(&tr))
            && !super::recognition::is_compiled_std(tcx, callee)
            && callee != size_hint;
        let by_ref = by_iterator && super::recognition::is_by_ref(tcx, callee);
        for &arg in args.iter() {
            let arg = match thir[strip(thir, arg)].kind {
                ExprKind::Borrow { arg, .. } => strip(thir, arg),
                _ => strip(thir, arg),
            };
            match () {
                // `it.by_ref()`, lent where it's stepped as a JS iterator
                // too, `for c in it.by_ref()`.
                _ if by_ref => lent.push((e, arg)),
                // Another of them made of it, `it.rev()`, which steps it as
                // a JS iterator where that's made in place.
                _ if super::recognition::makes_held_compiled(tcx, callee) => makers.push((arg, at)),
                _ if by_iterator || super::recognition::is_mem_drop(tcx, callee) => js_steps.push((arg, at)),
                _ => {}
            }
        }
    }
    let stepped_at: HashMap<ExprId, Span> = js_steps.iter().copied().collect();
    let mut stepped: HashMap<LocalVarId, Vec<Span>> = HashMap::new();
    let mut note = |e: ExprId, at: Span| {
        if let ExprKind::VarRef { id } = thir[e].kind
            && made.contains_key(&id)
        {
            stepped.entry(id).or_default().push(at);
        }
    };
    for &(arg, at) in &js_steps {
        note(arg, at);
    }
    for &(by_ref, arg) in &lent {
        if let Some(&at) = stepped_at.get(&by_ref) {
            note(arg, at);
        }
    }
    let mut made_of: HashMap<LocalVarId, Vec<Span>> = HashMap::new();
    for &(arg, at) in &makers {
        if let ExprKind::VarRef { id } = thir[arg].kind
            && made.contains_key(&id)
        {
            made_of.entry(id).or_default().push(at);
        }
    }
    // Each given to another made in place, through a chain of them,
    // `rev().rev()`'s first, and each variable whose reads are all such
    // steps, those of another made of it in place too, to the fixed point.
    loop {
        let before = found.len();
        for &(made, at, callee, index, maker) in &given {
            if steps_as_js(callee, index, &found, at, maker) {
                found.insert(made);
                steps.insert(at);
            }
        }
        for (&var, &at) in &made {
            let uses = stepped.get(&var).map_or(&[][..], Vec::as_slice);
            let of = made_of.get(&var).map_or(&[][..], Vec::as_slice);
            let all = uses.len() + of.len();
            if !found.contains(&at)
                && all > 0
                && reads.get(&var) == Some(&all)
                && of.iter().all(|maker| found.contains(maker))
            {
                found.insert(at);
                steps.extend(uses.iter().chain(of).copied());
            }
        }
        if found.len() == before {
            return (found, steps);
        }
    }
}

/// Each guard `if let Some(v) = value` of an arm of `match
/// js::classify(value) { .. }` whose variants are each one of what's there,
/// `Kind::Object(_) | Kind::Array(_)`: where it's written. A binding that's
/// its argument (`given_present`'s) says of a `None`, `undefined`, only a
/// variant that's `undefined` or `null` (ADR 0676), so where one of the
/// arm's matched, `value` is a `Some`, and the guard holds: it tests
/// nothing, and the arm is as one without it. Of a `value` the body never
/// changes, so what the guard binds is `value` itself wherever it's read.
fn held_guards<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    let changed = changes(thir);
    // The variable a call of a binding that's its argument, giving an
    // untagged enum, reads: `classify(value)`'s `value`, an `Option` of a
    // reference, whose `None` is `undefined`.
    let classified = |scrutinee: ExprId| {
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, scrutinee)].kind else {
            return None;
        };
        let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
            return None;
        };
        let identity = super::bindings::is_binding(tcx, def_id)
            && matches!(super::bindings::js_form(tcx, def_id), super::bindings::JsForm::This);
        let (adt, _) = super::untagged::untagged_adt(tcx, thir[scrutinee].ty)?;
        let [arg] = &args[..] else { return None };
        let ExprKind::VarRef { id } = thir[strip(thir, *arg)].kind else {
            return None;
        };
        let of_ref = matches!(thir[*arg].ty.kind(), ty::Adt(option, args)
            if tcx.is_lang_item(option.did(), LangItem::Option) && args.type_at(0).is_ref());
        // Of an enum with a variant of each of `undefined` and `null`: no
        // other is either, not one that's what the others aren't.
        let nullish = super::untagged::holds_undefined_and_null(tcx, adt);
        (identity && of_ref && nullish && !changed.contains_key(&id)).then_some((adt, id))
    };
    // Whether `pattern` is variants of `adt`, none `undefined` nor `null`.
    fn present<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>, pattern: &Pat<'tcx>) -> bool {
        match &pattern.kind {
            PatKind::Or { pats } => pats.iter().all(|p| present(tcx, adt, p)),
            PatKind::Variant {
                adt_def, variant_index, ..
            } => *adt_def == adt && !super::untagged::is_nullish(tcx, adt.variant(*variant_index)),
            _ => false,
        }
    }
    (thir.exprs.iter())
        .filter_map(|e| match e.kind {
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => Some((classified(scrutinee)?, arms)),
            _ => None,
        })
        .flat_map(|((adt, var), arms)| {
            (arms.iter()).filter_map(move |&arm| {
                let arm = &thir[arm];
                let guard = strip(thir, arm.guard?);
                let ExprKind::Let { expr, ref pat } = thir[guard].kind else {
                    return None;
                };
                let some = matches!(&pat.kind, PatKind::Variant { adt_def, variant_index, subpatterns, .. }
                    if tcx.is_lang_item(adt_def.variant(*variant_index).def_id, LangItem::OptionSome)
                        && matches!(&subpatterns[..], [only] if matches!(only.pattern.kind, PatKind::Binding { subpattern: None, .. } | PatKind::Wild)));
                let same = matches!(thir[strip(thir, expr)].kind, ExprKind::VarRef { id } if id == var);
                (some && same && present(tcx, adt, &arm.pattern)).then_some(thir[guard].span)
            })
        })
        .collect()
}

/// Where each `js::SymbolFor::from_value(value)` is called, its `fn_span`,
/// whose value the body only tests against the registry's symbols: what a
/// `match`, a `matches!` or an `if let` compares, of patterns each `_` or a
/// `Some` of them, `Some(REACT_MEMO_TYPE | REACT_LAZY_TYPE)`, binding
/// nothing. There, no other value is one of them, so the value itself
/// answers each as its `Some` would (ADR 0634).
fn registered_reads<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    (thir.exprs.indices())
        .filter_map(|e| match thir[e].kind {
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => {
                let span = registered_read(tcx, thir, scrutinee)?;
                (arms.iter())
                    .all(|&arm| tests_registered(tcx, &thir[arm].pattern))
                    .then_some(span)
            }
            ExprKind::Let { expr, ref pat } => tests_registered(tcx, pat).then_some(registered_read(tcx, thir, expr)?),
            _ => registered_equality(tcx, thir, e).map(|(read, _)| read),
        })
        .collect()
}

/// Where each `==` or `!=` `registered_reads` reads one for is, its
/// `fn_span`: compared by `===`, whatever the `Some` is (ADR 0771).
fn registered_equalities<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> HashSet<Span> {
    (thir.exprs.indices())
        .filter_map(|e| registered_equality(tcx, thir, e).map(|(_, equality)| equality))
        .collect()
}

/// The `fn_span` of `e`, a call of `js::SymbolFor::from_value`, in what
/// holds no more than its value.
fn registered_read<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, mut e: ExprId) -> Option<Span> {
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
}

/// Of `e`, an `==` or `!=` of a `from_value` read and a `Some(..)`, either
/// way round, which says what `matches!(.., Some(s))` does, the value the
/// symbol `s` is: the read's `fn_span` and the comparison's (ADR 0771).
fn registered_equality<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> Option<(Span, Span)> {
    let ExprKind::Call {
        fun, ref args, fn_span, ..
    } = thir[e].kind
    else {
        return None;
    };
    let &[a, b] = &args[..] else {
        return None;
    };
    if !is_partial_eq(tcx, thir[fun].ty) {
        return None;
    }
    let (a, b) = (borrowed(thir, a)?, borrowed(thir, b)?);
    let read = match (registered_read(tcx, thir, a), registered_read(tcx, thir, b)) {
        (Some(read), None) if is_some(tcx, thir, b) => read,
        (None, Some(read)) if is_some(tcx, thir, a) => read,
        _ => return None,
    };
    Some((read, fn_span))
}

/// Whether `fun` is `PartialEq`'s `eq` or `ne`, an `==`'s or a `!=`'s.
fn is_partial_eq<'tcx>(tcx: TyCtxt<'tcx>, fun: ty::Ty<'tcx>) -> bool {
    matches!(*fun.kind(), ty::FnDef(def_id, _)
        if tcx.trait_of_assoc(def_id).is_some_and(|id| tcx.is_lang_item(id, LangItem::PartialEq)))
}

/// What `e`, a shared borrow, `==`'s of each side, borrows.
fn borrowed(thir: &Thir<'_>, mut e: ExprId) -> Option<ExprId> {
    while let ExprKind::Scope { value: inner, .. } = thir[e].kind {
        e = inner;
    }
    match thir[e].kind {
        ExprKind::Borrow {
            borrow_kind: BorrowKind::Shared,
            arg,
        } => Some(arg),
        _ => None,
    }
}

/// Whether `e` makes a `Some(..)`.
fn is_some(tcx: TyCtxt<'_>, thir: &Thir<'_>, mut e: ExprId) -> bool {
    while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
        e = inner;
    }
    matches!(&thir[e].kind, ExprKind::Adt(adt)
        if tcx.is_lang_item(adt.adt_def.did(), LangItem::Option) && adt.variant_index.as_u32() == 1)
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

/// The value `pat` tests, of a number, a `char` or the registry's symbol,
/// itself, not what's in it: the one answer to which value a constant
/// pattern is (`PatternValue`).
pub(super) fn pattern_value<'tcx>(tcx: TyCtxt<'tcx>, pat: &Pat<'tcx>) -> Option<PatternValue<'tcx>> {
    match &pat.kind {
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

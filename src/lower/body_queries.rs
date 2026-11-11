//! Read-only questions about a captured THIR body. No emission state, names,
//! dependencies or JavaScript: these answers cannot change lowering as a side effect.

use super::bindings::named_callback;
use super::fn_def;
use super::recognition::{SliceLength, slice_length};
use crate::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::{BindingMode, ByRef, Mutability};

use rustc_middle::mir::{BinOp, BorrowKind, UnOp};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, LogicalOp, Pat, PatKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::{DefId, LocalModId};

mod collect_sources;
mod constants;
mod getter_unwraps;
mod in_place;
mod method_calls;
mod ranges;
mod set_reads;
mod total_orders;
mod values;
mod whole_casts;

use constants::constant_patterns;
pub(super) use constants::{
    ConstantPatterns, PatternStep, PatternValue, pattern_places, pattern_value, transparent_subpattern,
};
pub(in crate::lower) use in_place::decide as decide_in_place;
pub(super) use ranges::{present_item_of_sized, ranges};
pub(in crate::lower) use set_reads::is_option_unwrap;
pub(super) use total_orders::orders_totally;
pub(in crate::lower) use values::{assigned_once, indirect, through_box, value_local};
use whole_casts::whole_casts;

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
    /// `Index::index(&v, i)`; and of each map's `m[k]` whose key its tests
    /// show the map holds (rd558).
    pub(super) in_bounds: IndexSet<ExprId>,
    /// Where each `+`, `-` and `*` of a small integer is written that the
    /// body's ranges show never leaves its type's range (`ranges`).
    pub(super) exact: IndexSet<Span>,
    /// The closures a `let` names that a hook's function is, each written
    /// there, named (`named_callbacks`).
    pub(super) named_callbacks: IndexSet<LocalVarId>,
    /// The casts whose value is what they cast as it is: of an `f64` known
    /// to be a whole number in the integer's range (`whole_casts`), of an
    /// integer the body's ranges show in its target's (`ranges`), and of a
    /// signed one that's the count of text's `repeat` (`repeat_counts`).
    pub(super) kept_casts: IndexSet<ExprId>,
    /// The function of each `OnceCell::set` whose cell the body's ranges
    /// show still empty there, every time it's reached (`ranges`, ADR 0847).
    pub(super) set_empty: IndexSet<ExprId>,
    /// Where each read of a registered symbol is that's only tested against
    /// the registry's, `registered_reads`'s (ADR 0634).
    pub(super) registered_reads: IndexSet<Span>,
    /// Where each `==` or `!=` of such a read and a `Some(..)` is,
    /// `registered_equalities`'s (ADR 0771).
    pub(super) registered_equalities: IndexSet<Span>,
    /// Where each guard is that holds where its arm matched,
    /// `held_guards`'s.
    pub(super) held_guards: IndexSet<Span>,
    /// Where each call is whose value is told present, `present_calls`'s.
    pub(super) present_calls: IndexSet<Span>,
    /// Where each `resume_unwind(x)` is of a payload as it was given,
    /// `resumed_as_given`'s (rd550).
    pub(super) resumed_as_given: IndexSet<Span>,
    /// Where each `unwrap` or `expect` is of a cell's `get` read right
    /// after it's set to a `Some`, `set_then_unwrapped`'s (rd331e).
    pub(super) set_then_unwrapped: IndexSet<Span>,
    /// Where each `unwrap()` is of a binding getter's function or object
    /// read only as the callee or the `this` of the call it's made for,
    /// `getter_unwraps`'s (rd519).
    pub(super) getter_unwraps: IndexSet<Span>,
    /// Where each `function::call` is of a `js::method` of one step whose
    /// `this` is the path's value, the method's call, and where each such
    /// `js::method` is, `method_calls`'s (rd334).
    pub(super) method_calls: IndexSet<Span>,
    pub(super) called_methods: IndexSet<Span>,
    /// Where each `rev()`, `enumerate()` or `map()` is whose adapter a std
    /// iterator's method steps where it's made, `in_place_adapters`'s: its
    /// maker marks what it makes so (`made_in_place`, #730). Decided once the
    /// crate's `Recognition` exists (`decide_in_place`), none reads it before.
    pub(super) in_place_adapters: InPlace,
    /// The `Vec` each `collect()` std may make in place is made of, by the
    /// call's span, `in_place_collect_sources`'s (#1259).
    pub(super) in_place_collect_sources: IndexMap<Span, collect_sources::CollectSource<'tcx>>,
    /// Each value the body's patterns test and the `const` they name it by,
    /// where it's tested, `constant_patterns`'s (ADRs 0590, 0634).
    pub(super) constant_patterns: ConstantPatterns<'tcx>,
    /// The props given to the crate's components, and what's a base of
    /// them, `jsx_given_props`'s.
    pub(super) jsx_given: IndexSet<ExprId>,
    pub(super) jsx_bases: IndexSet<ExprId>,
}

impl<'tcx> BodyFacts<'tcx> {
    /// Its facts but its ranges, which are the crate's (`ranges`): each
    /// read of its own THIR once, before it's lowered, so a call it lowers
    /// looks its answer up, and a closure's are its own body's.
    pub(super) fn collect(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Self {
        let (jsx_given, jsx_bases) = jsx_given_props(tcx, thir);
        let (method_calls, called_methods) = method_calls::method_calls(tcx, thir);
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
            resumed_as_given: resumed_as_given(tcx, thir),
            set_then_unwrapped: set_reads::set_then_unwrapped(tcx, thir),
            getter_unwraps: getter_unwraps::getter_unwraps(tcx, thir),
            method_calls,
            called_methods,
            in_place_adapters: InPlace(None),
            in_place_collect_sources: collect_sources::in_place_collect_sources(tcx, thir),
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
fn named_callbacks<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<LocalVarId> {
    let closures: IndexSet<LocalVarId> = thir
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
    let mut found = IndexSet::default();
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

/// Each `resume_unwind(x)` of a variable of the body's own, `x`, in a body
/// that lends no `&mut` to what holds a `dyn Any` (`lends_dyn_any`): only
/// through one is what a `Box<dyn Any>` holds written, so `x`'s holds what
/// it was given, `catch_unwind`'s `Err` of what was thrown, whichever
/// variables held it before (rd550); `x` itself may be set again, which
/// its JS says, a `let`. Its `fn_span`. One of a variable a closure
/// captured is its family's, `resumes_as_given`'s.
fn resumed_as_given<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
    match lends_dyn_any(tcx, thir) {
        true => IndexSet::default(),
        false => resumes(tcx, thir, false),
    }
}

/// Each `resume_unwind(x)` of a variable a closure captured, of a body
/// whose family, the function it's in and each closure in that, lends no
/// `&mut` to what holds a `dyn Any`: where the payload it captured is made
/// and changed, if anywhere (rd550). Before lowering, of the crate's
/// bodies.
fn resumes_as_given<'tcx>(tcx: TyCtxt<'tcx>, bodies: &mut [super::Body<'tcx>]) {
    let lending: IndexSet<DefId> = (bodies.iter())
        .filter(|body| lends_dyn_any(tcx, &body.thir))
        .map(|body| tcx.typeck_root_def_id(body.def_id))
        .collect();
    for body in bodies.iter_mut() {
        if !lending.contains(&tcx.typeck_root_def_id(body.def_id)) {
            let captured = resumes(tcx, &body.thir, true);
            body.facts.resumed_as_given.extend(captured);
        }
    }
}

/// Whether `thir` lends a `&mut` to what holds a `dyn Any`, anywhere in a
/// value's type, `Option<&mut Box<dyn Any>>`'s too: the `&mut *x`
/// `downcast_mut` is given, a `ref mut` binding's, a closure's capture by
/// `&mut`, a `mem::swap`'s, one made of a raw pointer.
fn lends_dyn_any<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> bool {
    use super::recognition::holds_dyn_any;
    thir.exprs.iter().any(|expr| {
        (expr.ty.walk())
            .filter_map(|part| part.as_type())
            .any(|part| matches!(*part.kind(), ty::Ref(_, to, Mutability::Mut) if holds_dyn_any(tcx, to)))
    })
}

/// The `fn_span` of each `resume_unwind(x)` of a variable, of the body's own
/// or, `captured`, one a closure captured.
fn resumes<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, captured: bool) -> IndexSet<Span> {
    use super::recognition::unwind_op;
    use super::std_types::unwind::UnwindOp;
    // A variable, as it is or as the payload's type, `dyn Any + Send +
    // 'static` of `dyn Any + Send`, which its `Box` is coerced to.
    let variable = |e: ExprId| {
        let e = match thir[strip(thir, e)].kind {
            ExprKind::PointerCoercion {
                cast: PointerCoercion::Unsize,
                source,
                ..
            } => source,
            _ => e,
        };
        match thir[strip(thir, e)].kind {
            ExprKind::VarRef { .. } => !captured,
            ExprKind::UpvarRef { .. } => captured,
            _ => false,
        }
    };
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::Call {
                fun, ref args, fn_span, ..
            } if matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if unwind_op(tcx, id) == Some(UnwindOp::Resume))
                && matches!(**args, [payload] if variable(payload)) =>
            {
                Some(fn_span)
            }
            _ => None,
        })
        .collect()
}

/// Where each variable is changed, as the body's order has it: set or lent
/// as `&mut`, in whole or in part. A variable of the
/// enclosing function a closure captures is the closure's too.
pub(in crate::lower) fn changes(thir: &Thir<'_>) -> IndexMap<LocalVarId, Vec<Span>> {
    let at = |span: Span| span.source_callsite();
    let root = |mut e: ExprId| loop {
        match thir[strip(thir, e)].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => break Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => break None,
        }
    };
    let mut changes: IndexMap<LocalVarId, Vec<Span>> = IndexMap::default();
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
pub(super) fn tail(thir: &Thir<'_>, e: ExprId) -> ExprId {
    let e = strip(thir, e);
    match thir[e].kind {
        ExprKind::Block { block } if let Some(value) = thir[block].expr => tail(thir, value),
        _ => e,
    }
}

/// What a body's facts take of the crate's other bodies, before lowering:
/// each resume of a payload a closure captured (`resumes_as_given`). Which
/// adapters each makes in place is decided later, once the crate's
/// `Recognition` exists (`decide_in_place`).
pub(super) fn across_bodies<'tcx>(tcx: TyCtxt<'tcx>, bodies: &mut [super::Body<'tcx>]) {
    resumes_as_given(tcx, bodies);
}

/// Which of `HELD_COMPILED`'s adapters a body makes in place, decided once
/// the crate's `Recognition` exists (`decide_in_place`, #1327): before then
/// none, and nothing reads it, as its readers ask only `contains`, which an
/// undecided one refuses. Only this module makes or decides one.
#[derive(Default)]
pub(in crate::lower) struct InPlace(Option<IndexSet<Span>>);

impl InPlace {
    fn decided(set: IndexSet<Span>) -> Self {
        InPlace(Some(set))
    }

    /// Whether the adapter made at `span` is made in place.
    pub(in crate::lower) fn contains(&self, span: &Span) -> bool {
        let Some(set) = &self.0 else {
            panic!("rust-js: a body's in-place adapters read before they're decided (decide_in_place)");
        };
        set.contains(span)
    }
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
/// are, in order, those from the `from`th: `ptr::eq(self, other)`, each
/// reborrowed or made a raw pointer, and its generic arguments (ADR 0864);
/// a namespace's method's, past its `self`, `batched_updates(f, a)` (rd292b,
/// `passed_on::forwarded_function`). `None` of a body that does anything
/// else.
pub(super) fn forwards_to<'tcx>(thir: &Thir<'tcx>, from: usize) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
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
    let params: Vec<LocalVarId> = (thir.params.iter().skip(from))
        .map(|p| match p.pat.as_deref().map(|pat| &pat.kind) {
            Some(rustc_middle::thir::PatKind::Binding { var, .. }) => Some(*var),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let given: Vec<LocalVarId> = args.iter().map(|&a| param(a)).collect::<Option<_>>()?;
    (given == params).then(|| super::fn_def(thir[fun].ty)).flatten()
}

/// Whether `thir`, a closure's body, orders its two parameters by their
/// UTF-16 units, first then second: `|a, b|
/// a.encode_utf16().cmp(b.encode_utf16())`, JS's own order of strings,
/// which its `sort()` sorts by (ADR 0945). Each parameter a string or what
/// derefs to one, `&&str` or `&String`.
pub(super) fn orders_by_utf16_units<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, root: ExprId) -> bool {
    use super::recognition::{compares_utf16_units, is_string_deref, makes_utf16_units};
    // The parameter a string's `encode_utf16()` is given, through its
    // borrows and derefs, a `String`'s, std's, too.
    let units_of = |e: ExprId| -> Option<LocalVarId> {
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let (called, _) = super::fn_def(thir[fun].ty)?;
        if !makes_utf16_units(tcx, called, thir[strip(thir, e)].ty) {
            return None;
        }
        let mut a = *args.first()?;
        loop {
            a = strip(thir, a);
            match thir[a].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => a = arg,
                ExprKind::Call { fun, ref args, .. }
                    if args.len() == 1
                        && super::fn_def(thir[fun].ty).is_some_and(|(f, args)| is_string_deref(tcx, f, args)) =>
                {
                    a = args[0]
                }
                ExprKind::VarRef { id } => return Some(id),
                _ => return None,
            }
        }
    };
    let params: Vec<LocalVarId> = (thir.params.iter())
        .filter_map(|p| match p.pat.as_deref().map(|pat| &pat.kind) {
            Some(PatKind::Binding { var, .. }) => Some(*var),
            _ => None,
        })
        .collect();
    let e = tail(thir, root);
    let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
        return false;
    };
    let [first, second] = args[..] else { return false };
    super::fn_def(thir[fun].ty).is_some_and(|(cmp, _)| compares_utf16_units(tcx, cmp, thir[strip(thir, first)].ty))
        && params.len() == 2
        && units_of(first) == Some(params[0])
        && units_of(second) == Some(params[1])
}

/// The fields of structs `thir` writes a `js::null()` to, which may then
/// hold `null` where a plain `Option` field holds `undefined` (ADR 0935,
/// amended): a struct literal's, `Fiber { name: js::null() }`; an
/// assignment's, `fiber.name = js::null()`, through a `&mut` too; and one
/// lent mutably to a call given a `js::null()`, `mem::replace(&mut
/// fiber.name, js::null())`. And a cell's that's the property it holds
/// (rd415): one made of a `js::null()`, `Cell::new(js::null())`, one lent
/// to a call given one, `fiber.state.set(js::null())`, and what its
/// `borrow_mut()` lends, assigned one. Each is declared `| null`.
pub(super) fn null_written_fields(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> Vec<DefId> {
    use super::recognition::{StdItem, is_std_def};
    let is_std = |ty: ty::Ty<'_>, items: &[StdItem]| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if items.iter().any(|&item| is_std_def(tcx, adt.did(), item)));
    let is_cell = |e: ExprId| is_std(thir[strip(thir, e)].ty, &[StdItem::Cell, StdItem::RefCell]);
    let is_null = |e: ExprId| match thir[strip(thir, e)].kind {
        ExprKind::Call { fun, .. } => {
            super::fn_def(thir[fun].ty).is_some_and(|(f, _)| super::bindings::is_js_null(tcx, f))
        }
        _ => false,
    };
    // `js::null()`, or a cell made of one, `Cell::new(js::null())`.
    let made_null = |e: ExprId| {
        is_null(e)
            || matches!(thir[strip(thir, e)].kind, ExprKind::Call { ref args, .. }
                if is_cell(e) && matches!(args[..], [arg] if is_null(arg)))
    };
    // What each variable is made of, `let slot = &mut fiber.name;`.
    let made: IndexMap<LocalVarId, ExprId> = (thir.stmts.iter())
        .filter_map(|stmt| match stmt.kind {
            thir::StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                ..
            } => match pattern.kind {
                PatKind::Binding { var, .. } => Some((var, init)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    // The field a place is, through its borrows and derefs, and a variable
    // made of one.
    let field_of = |mut e: ExprId| loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            ExprKind::VarRef { id } if thir[e].ty.is_ref() => e = *made.get(&id)?,
            // What a cell's `borrow_mut()` lends, the cell's, through its
            // guard's `deref_mut`.
            ExprKind::Call { ref args, .. }
                if let [lent] = args[..]
                    && (is_cell(lent) || is_std(thir[strip(thir, lent)].ty, &[StdItem::RefCellRefMut])) =>
            {
                e = lent
            }
            ExprKind::Field {
                lhs,
                variant_index,
                name,
            } => {
                return match thir[lhs].ty.peel_refs().kind() {
                    ty::Adt(adt, _) => Some(adt.variant(variant_index).fields[name].did),
                    _ => None,
                };
            }
            _ => return None,
        }
    };
    let lent_mutably = |e: ExprId| matches!(thir[strip(thir, e)].kind, ExprKind::Borrow { borrow_kind, .. } if borrow_kind.mutability().is_mut());
    let mut written = Vec::new();
    for e in thir.exprs.iter() {
        match e.kind {
            ExprKind::Adt(ref made) => {
                let fields = &made.adt_def.variant(made.variant_index).fields;
                written.extend(
                    (made.fields.iter())
                        .filter(|f| made_null(f.expr))
                        .map(|f| fields[f.name].did),
                );
            }
            ExprKind::Assign { lhs, rhs } if is_null(rhs) => written.extend(field_of(lhs)),
            ExprKind::Call { ref args, .. } if args.iter().any(|&a| is_null(a)) => written.extend(
                (args.iter())
                    .filter(|&&a| lent_mutably(a) || is_cell(a))
                    .filter_map(|&a| field_of(a)),
            ),
            _ => {}
        }
    }
    written
}

/// Is `thir`, a constant's initializer, written of other constants that
/// are read by their names (`by_name`), `A | B` or `EventPriority(NO_LANE)`,
/// and of no call of a function `by_name` says is compiled, whose body would
/// run where its module loads: so its JS is as it's written, not rustc's
/// value (ADR 0031, amended). A call `in_place` says is written as the value
/// it gives, `js::null()`'s `null`, is such a constant, which rustc's value
/// would leave out (ADR 0935, amended).
pub(super) fn written_of_constants(
    thir: &Thir<'_>,
    by_name: &dyn Fn(DefId) -> bool,
    in_place: &dyn Fn(DefId) -> bool,
) -> bool {
    let calls_compiled = thir.exprs.iter().any(|e| match e.kind {
        ExprKind::Call { fun, .. } => super::fn_def(thir[fun].ty).is_none_or(|(def_id, _)| by_name(def_id)),
        ExprKind::Closure(_) => true,
        _ => false,
    });
    let names_constant = (thir.exprs.iter()).any(|e| match e.kind {
        ExprKind::NamedConst { def_id, .. } => by_name(def_id),
        ExprKind::Call { fun, .. } => super::fn_def(thir[fun].ty).is_some_and(|(def_id, _)| in_place(def_id)),
        _ => false,
    });
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

/// The casts to `usize` of a signed integer of 32 bits or fewer that are
/// the count of a `repeat` of text that isn't empty, `"  ".repeat(indent as
/// usize)`, each the integer as it is (ADR 0774, amended): a negative one is
/// `2^31` or more as a `usize`, past `isize::MAX` bytes of text that isn't
/// empty, which std panics of and JS's `repeat` throws its `RangeError` of,
/// as it does of a negative count.
fn repeat_counts<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<ExprId> {
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

/// Each struct JSX gives a component as its props, `<*>`'s second, with
/// its base, `{..Default::default()}`, and its flattened fields' values,
/// made there as structs or defaults: JSX takes them apart (ADR 0213); and
/// what's a base of them, `..props.html`, where a flattened field read
/// whole is the object its parent is, `{...props}`.
fn jsx_given_props<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> (IndexSet<ExprId>, IndexSet<ExprId>) {
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
    let (mut props, mut bases) = (IndexSet::default(), IndexSet::default());
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

/// The `const`s `thir` names: read, `URGENT_LANES`, and tested, a
/// pattern's, `IMMEDIATE_PRIORITY =>`, which a test names too (ADR 0590).
pub(super) fn consts_named(thir: &Thir<'_>) -> IndexSet<DefId> {
    let mut named: IndexSet<DefId> = (thir.exprs.iter())
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
fn present_calls<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
    (thir.exprs.indices())
        .filter_map(|e| given_present(tcx, thir, e))
        .collect()
}

/// Each guard `if let Some(v) = value` of an arm of `match
/// js::classify(value) { .. }` whose variants are each one of what's there,
/// `Kind::Object(_) | Kind::Array(_)`: where it's written. A binding that's
/// its argument (`given_present`'s) says of a `None`, `undefined`, only a
/// variant that's `undefined` or `null` (ADR 0676), so where one of the
/// arm's matched, `value` is a `Some`, and the guard holds: it tests
/// nothing, and the arm is as one without it. What the guard binds is
/// `value`, read where it's bound: of a `value` the arm then sets, a copy
/// of it there, `const value$1 = value;`, as any variable set again is
/// read where it is (ADR 0853).
fn held_guards<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
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
        (identity && of_ref && nullish).then_some((adt, id))
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
fn registered_reads<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
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
        .chain(registered_lets(tcx, thir).into_iter().map(|(read, _)| read))
        .collect()
}

/// Where each `==` or `!=` `registered_reads` reads one for is, its
/// `fn_span`: compared by `===`, whatever the `Some` is (ADR 0771).
fn registered_equalities<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
    (thir.exprs.indices())
        .filter_map(|e| registered_equality(tcx, thir, e).map(|(_, equality)| equality))
        .chain(
            registered_lets(tcx, thir)
                .into_iter()
                .flat_map(|(_, equalities)| equalities),
        )
        .collect()
}

/// Of each `let` of a `from_value` read, `let r#typeof =
/// SymbolFor::from_value(..)`, whose every read is an `==` or `!=` of it
/// and a `Some(..)`, React's mountLazyComponent's `if ($$typeof ===
/// REACT_FORWARD_REF_TYPE) .. else if ($$typeof === REACT_MEMO_TYPE)`: the
/// read's `fn_span`, and each comparison's, as of the read in place (ADR
/// 0771, amended).
fn registered_lets<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> Vec<(Span, Vec<Span>)> {
    let compared = |e: ExprId| -> Option<(LocalVarId, Span)> {
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
        let var = |e: ExprId| match thir[strip(thir, e)].kind {
            ExprKind::VarRef { id } => Some(id),
            _ => None,
        };
        let (a, b) = (borrowed(thir, a)?, borrowed(thir, b)?);
        match (var(a), var(b)) {
            (Some(id), None) if is_some(tcx, thir, b) => Some((id, fn_span)),
            (None, Some(id)) if is_some(tcx, thir, a) => Some((id, fn_span)),
            _ => None,
        }
    };
    let comparisons: Vec<(LocalVarId, Span)> = thir.exprs.indices().filter_map(compared).collect();
    (thir.stmts.iter())
        .filter_map(|stmt| {
            let thir::StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                ..
            } = stmt.kind
            else {
                return None;
            };
            let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, _),
                subpattern: None,
                ..
            } = pattern.kind
            else {
                return None;
            };
            let read = registered_read(tcx, thir, init)?;
            let reads = (thir.exprs.iter())
                .filter(|e| matches!(e.kind, ExprKind::VarRef { id } if id == var))
                .count();
            let equalities: Vec<Span> = (comparisons.iter())
                .filter(|(id, _)| *id == var)
                .map(|&(_, span)| span)
                .collect();
            (reads > 0 && reads == equalities.len()).then_some((read, equalities))
        })
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

/// The text a pattern of a string, `"react.memo"`, is.
pub(super) fn pattern_text(pattern: &Pat<'_>) -> Option<String> {
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

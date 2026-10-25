//! Iterators: a chain of std's adapters on an array or a JS iterator, and
//! what ends it (ADRs 0036, 0055, 0128), lazy where Rust's order can be seen
//! (ADR 0139), and what a chain that owns its items drops (ADR 0098).

mod dictionaries;
mod eager;
mod ends;
mod in_place;
mod knowing;
mod stepping;

pub(super) use self::{eager::*, in_place::*};

use super::calls::Call;
use super::combinators::IterSource;
use super::fn_def;
use super::recognition::{StdItem, std_item, trait_method};
use super::representation::Num;
use super::std_types::range::RangeKind;
use super::{FnCx, R, Std};
use crate::js;
use crate::js::{Expr, Op, Stmt, StmtKind};

use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// `$iterator(value, next)`: a JS iterator of `value`, a Rust iterator,
/// stepped by `next`, which gives its `Option`; of one whose `Some` may be
/// boxed (ADR 0051), `boxed`, unboxed; given `back`, its `next_back`, one
/// that knows its back, which `$nextBack` steps (ADR 0429). The one JS form
/// of it.
pub(in crate::lower) fn stepped(value: Expr, next: Expr, boxed: bool, back: Option<Expr>) -> Expr {
    stepped_ends(value, next, boxed, back, None, None, OwnMethods::default())
}

/// `it`, an iterator of the crate's made a JS one, behind a `dyn Iterator`:
/// std's `Box<I>` and `&mut I` give `nth` to its own, but `fold` is std's,
/// of `next`, which `$iterator` isn't given then (#162).
pub(in crate::lower) fn behind_a_dyn(mut it: Expr) -> Expr {
    if let js::ExprKind::Call(callee, given) = &mut it.kind
        && callee.helper_name() == Some("$iterator")
        && given.len() > 6
    {
        given[6] = Expr::undefined();
        fewest_iterator_args(given);
    }
    it
}

/// A `$iterator` call's arguments, `given`, without those at the end that
/// are its defaults: an `undefined`, and `boxed`'s `false`.
pub(in crate::lower) fn fewest_iterator_args(given: &mut Vec<Expr>) {
    while given.len() > 2 && matches!(given.last().map(|e| &e.kind), Some(js::ExprKind::Undefined)) {
        given.pop();
    }
    if given.len() == 3 && matches!(given[2].kind, js::ExprKind::Bool(false)) {
        given.pop();
    }
}

/// The methods the impl of a crate's iterator writes itself that std
/// routes other calls through, each where it writes it: what `$iterator`
/// is given, and its dictionary (#162, #182); its `rfold` and `nth_back`,
/// which std's adapters stepped from their back route through (#220).
#[derive(Default)]
pub(in crate::lower) struct OwnMethods {
    pub(in crate::lower) fold: Option<Expr>,
    pub(in crate::lower) nth: Option<Expr>,
    pub(in crate::lower) last: Option<Expr>,
    pub(in crate::lower) rfold: Option<Expr>,
    pub(in crate::lower) nth_back: Option<Expr>,
}

/// `stepped`'s, given `len` too, its length, one that knows it, an
/// `ExactSizeIterator`'s (ADR 0456), `hint`, its own `size_hint`, an
/// iterator of the crate's whose impl has one: what `size_hint()` of it
/// gives in generic code, as in its own; and `own`, its own `fold` and
/// `nth`, which std's consumers call (#162).
pub(in crate::lower) fn stepped_ends(
    value: Expr,
    next: Expr,
    boxed: bool,
    back: Option<Expr>,
    len: Option<Expr>,
    hint: Option<Expr>,
    own: OwnMethods,
) -> Expr {
    // Each given, and each before it, `undefined` where it isn't.
    let OwnMethods {
        fold,
        nth,
        last,
        rfold,
        nth_back,
    } = own;
    let given = [
        boxed.then(|| Expr::bool(true)),
        back,
        len,
        hint,
        fold,
        nth,
        last,
        rfold,
        nth_back,
    ];
    let Some(last) = given.iter().rposition(Option::is_some) else {
        return Expr::call(Expr::helper("$iterator"), vec![value, next]);
    };
    let mut list = vec![value, next];
    for (at, arg) in given.into_iter().enumerate().take(last + 1) {
        list.push(arg.unwrap_or_else(|| match at {
            0 => Expr::bool(false),
            _ => Expr::undefined(),
        }));
    }
    Expr::call(Expr::helper("$iterator"), list)
}

/// What a JS iterator `stepped_ends` made steps, `value` of `$iterator(value,
/// next, ..)`. A map's source made a JS iterator before it's asked its back,
/// `$iterator(range, $rangeNext)`, is stepped again from what it steps, not
/// from the JS iterator (ADR 0535). Not one made in place (`made_in_place`):
/// a `rev()` of the crate's iterator, `$iterator(it, nextBack)`, is its own
/// form, which only `stepped_form` reads into (#257).
pub(in crate::lower) fn stepped_value(e: &Expr) -> Option<Expr> {
    if made_in_place(e) {
        return None;
    }
    stepped_form(e).cloned()
}

/// `value` of `$iterator(value, next, ..)` or `$given(value, ..)`, whatever
/// it stands for: the one reader of its form.
pub(in crate::lower) fn stepped_form(e: &Expr) -> Option<&Expr> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    matches!(callee.helper_name(), Some("$iterator" | "$given")).then(|| args.first())?
}

/// What a JS iterator closed to drop it is made of, `it` and `drop` of
/// `$closing(it, drop)`, or of `$closing(it, drop, true)`, a `Vec`'s that
/// drops what it passes over as one slice (#234), as `closing` makes it:
/// the one reader of its form.
pub(in crate::lower) fn closed_parts(e: &Expr) -> Option<(&Expr, &Expr)> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    match args.as_slice() {
        [it, drop] | [it, drop, _] if callee.helper_name() == Some("$closing") => Some((it, drop)),
        _ => None,
    }
}

/// Whether a bound of `tr` reads its value as its own type, `Clone`'s or
/// `Debug`'s, or the crate's trait that extends `Iterator` (ADR 0511): not
/// a marker's, nor `Iterator`'s or std's extension of it, whose methods
/// its JS iterator answers (ADRs 0164, 0456). The one rule of what's
/// given as it is, of an iterator and of what an `IntoIterator` makes.
fn reads_its_value(tcx: ty::TyCtxt<'_>, tr: DefId) -> bool {
    tr != std_item(tcx, StdItem::Iterator)
        && !super::recognition::is_iterator_extension(tcx, tr)
        && (tcx.associated_items(tr).in_definition_order()).any(|item| item.is_fn())
}

/// Whether `id` holds what `into` makes as an `IntoIterator`, its
/// `IntoIter`, as it is: bounds it by a trait that reads its value,
/// `<T as IntoIterator>::IntoIter: Clone`, or its signature holds it in a
/// type of the crate's or a compiled std type, `Pair<I::IntoIter>` given
/// back, whose struct holds it as it is (ADRs 0411, 0561); not in a std
/// adapter, `Zip<I::IntoIter, J>`, a JS iterator of it. Then `into`, a type
/// parameter or an associated type, is given as it is, with its
/// `IntoIterator` dictionary, which makes its `IntoIter` as it is (ADR 0552).
/// An impl's bound is its dictionary's, which each of its methods is given:
/// of an impl and of its methods alike, whether any of them holds it.
pub(in crate::lower) fn into_iter_held_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, into: Ty<'tcx>) -> bool {
    let impl_of = match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Impl { .. } => Some(id),
        rustc_hir::def::DefKind::AssocFn => tcx.impl_of_assoc(id),
        _ => None,
    };
    match impl_of {
        Some(imp) => (tcx.associated_item_def_ids(imp).iter())
            .any(|&f| matches!(tcx.def_kind(f), rustc_hir::def::DefKind::AssocFn) && held_by(tcx, f, into)),
        None => held_by(tcx, id, into),
    }
}

/// `into_iter_held_as_it_is`'s, of one function or item `id` alone.
fn held_by<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, into: Ty<'tcx>) -> bool {
    // An associated type's, `I::Item: IntoIterator`'s, whose values no call
    // makes a JS iterator, an iterator's items, an `Option`'s `T |
    // undefined` among them, given as they are (#278); not what an
    // `into_iter()` makes, `I::IntoIter`, the JS iterator it gives.
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = super::recognition::trait_method(tcx, into_iterator, "IntoIter");
    if matches!(*into.kind(), ty::Alias(_, ty::AliasTy { kind: ty::Projection { def_id }, .. }) if def_id != into_iter)
    {
        return true;
    }
    let is_its_iter = |ty: Ty<'tcx>| {
        matches!(*ty.kind(), ty::Alias(_, ty::AliasTy {
            kind: ty::Projection { def_id }, args, ..
        }) if def_id == into_iter && args.type_at(0) == into)
    };
    let read = (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .any(|p| is_its_iter(p.self_ty().skip_binder()) && reads_its_value(tcx, p.def_id()));
    let holds = |ty: Ty<'tcx>| {
        ty.walk().any(|part| match part.as_type().map(|t| *t.kind()) {
            Some(ty::Adt(adt, args))
                if adt.did().is_local() || super::recognition::is_compiled_std_type(tcx, adt.did()) =>
            {
                args.types()
                    .any(|arg| arg.walk().any(|part| part.as_type().is_some_and(is_its_iter)))
            }
            _ => false,
        })
    };
    let held = matches!(
        tcx.def_kind(id),
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
    ) && tcx
        .fn_sig(id)
        .instantiate_identity()
        .skip_binder()
        .inputs_and_output
        .iter()
        .any(holds);
    read || held
}

/// Whether a bound of `id` names `param` the `IntoIter` of an
/// `IntoIterator` it holds as it is, `U` of `I::Item: IntoIterator<IntoIter
/// = U>`: the same value, given as it is (ADR 0552, #278).
fn is_held_into_iter(tcx: ty::TyCtxt<'_>, id: DefId, param: ty::ParamTy) -> bool {
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = super::recognition::trait_method(tcx, into_iterator, "IntoIter");
    (tcx.clauses_of(id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_projection_clause())
        .map(|p| p.skip_binder())
        .any(|p| {
            p.def_id() == into_iter
                && p.term
                    .as_type()
                    .is_some_and(|t| matches!(t.kind(), ty::Param(q) if q.index == param.index))
                && into_iter_held_as_it_is(tcx, id, p.self_ty())
        })
}

/// Whether `ty` is what a type parameter of `id` makes as an `IntoIterator`,
/// `<I as IntoIterator>::IntoIter`, held as it is (ADR 0552): stepped by the
/// `Iterator` its `IntoIterator` dictionary carries of it.
pub(in crate::lower) fn into_iter_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, ty: Ty<'tcx>) -> bool {
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    let into_iter = super::recognition::trait_method(tcx, into_iterator, "IntoIter");
    matches!(*ty.kind(), ty::Alias(_, ty::AliasTy {
        kind: ty::Projection { def_id }, args, ..
    }) if def_id == into_iter && into_iter_held_as_it_is(tcx, id, args.type_at(0)))
}

/// Whether `callee`'s type parameter `index` is its trait's, `Self` or a
/// parameter of the trait a method of it is called through: given as the
/// trait's impl takes it, never made a JS iterator for it, which only a
/// method's own parameters are (ADR 0551).
pub(in crate::lower) fn given_by_its_trait(tcx: ty::TyCtxt<'_>, callee: DefId, index: u32) -> bool {
    tcx.trait_of_assoc(callee).is_some() && (index as usize) < tcx.generics_of(callee).parent_count
}

/// Whether `id`'s type parameter `param`, bound by `Iterator`, is given as it
/// is (ADR 0411): one an impl's type holds, `I` of `Doubled<I>`, or the
/// function's signature has in anything but itself, `Wrap<I>` or a result,
/// or another bound reads as its own type, `I: Clone`'s `clone(&it)` (ADR
/// 0474), whose value its caller may read as its own type, and so stepped
/// by its `Iterator` dictionary, `{ next }`, where it's stepped. One given
/// only itself, `i: I`, is a JS iterator (ADR 0061). The one answer, of a
/// callee's evidence and its caller's argument alike.
pub(in crate::lower) fn given_as_it_is<'tcx>(tcx: ty::TyCtxt<'tcx>, id: DefId, param: ty::ParamTy) -> bool {
    // Bound by `Iterator` itself, or a trait that extends it, std's or the
    // crate's, whose `Iterator` `bounds` gives it (ADRs 0488, 0511), as
    // `bounds` reads its clauses, which make its evidence.
    // An `IntoIterator` whose `IntoIter` it holds as it is (ADR 0552).
    if into_iter_held_as_it_is(tcx, id, Ty::new_param(tcx, param.index, param.name)) {
        return true;
    }
    // What another's `IntoIterator` makes as it's held, `U` of `I::Item:
    // IntoIterator<IntoIter = U>`, `Flatten`'s (#278).
    if is_held_into_iter(tcx, id, param) {
        return true;
    }
    let iterator = std_item(tcx, StdItem::Iterator);
    let of_param: Vec<DefId> = tcx
        .clauses_of(id)
        .instantiate_identity(tcx)
        .into_iter()
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .filter(|p| p.self_ty().skip_binder() == Ty::new_param(tcx, param.index, param.name))
        .map(|p| p.def_id())
        .collect();
    if !of_param
        .iter()
        .any(|&tr| tr == iterator || super::recognition::implies_iterator(tcx, tr))
    {
        return false;
    }
    // A bound whose methods read its value as its own type, `Clone`'s or
    // `Debug`'s, or the crate's trait that extends `Iterator` (ADR 0511):
    // not a marker's, nor std's extension of `Iterator`,
    // `DoubleEndedIterator`, whose methods its JS iterator answers (ADRs
    // 0164, 0456).
    let read = of_param.iter().any(|&tr| reads_its_value(tcx, tr));
    // A value of it, not its `Item`'s, `Vec<I::Item>`: what's in a
    // projection isn't given back.
    let has = |ty: Ty<'tcx>| {
        let mut walk = ty.walk();
        while let Some(arg) = walk.next() {
            match arg.as_type().map(|t| *t.kind()) {
                Some(ty::Alias(..)) => walk.skip_current_subtree(),
                Some(ty::Param(p)) if p.index == param.index => return true,
                _ => {}
            }
        }
        false
    };
    let held = match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Impl { .. } => Some(id),
        _ => tcx.impl_of_assoc(id),
    }
    .is_some_and(|imp| has(tcx.type_of(imp).instantiate_identity().skip_normalization()));
    // A trait's own method takes `Self` as its dictionary gives it, but one
    // compiled into `@rust-js/std` as a function over `Self`, `peekable`'s
    // (ADR 0185, 0561), as a function's signature says.
    let signature = matches!(
        tcx.def_kind(id),
        rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
    ) && (tcx.trait_of_assoc(id).is_none() || super::recognition::is_compiled_std(tcx, id))
        && tcx.trait_impl_of_assoc(id).is_none()
        && {
            let sig = tcx.fn_sig(id).instantiate_identity().skip_binder();
            has(sig.output())
                || sig.inputs().iter().any(|&input| {
                    !matches!(input.peel_refs().kind(), ty::Param(p) if p.index == param.index) && has(input)
                })
        };
    held || signature || read
}

/// The value `(p) => p === value` or `(p) => value === p` looks for, if
/// it doesn't depend on `p` and reads the same however often it's read.
fn sought(test: &Expr) -> Option<Expr> {
    let js::ExprKind::Arrow(params, body) = &test.kind else {
        return None;
    };
    let (
        [js::Pattern::Name(param)],
        [
            Stmt {
                kind: StmtKind::Return(Some(e)),
                ..
            },
        ],
    ) = (params.as_slice(), body.as_slice())
    else {
        return None;
    };
    let js::ExprKind::Binary(Op::Eq, a, b) = &e.kind else {
        return None;
    };
    let is_param = |e: &Expr| matches!(&e.kind, js::ExprKind::Var(name) if name == param);
    let value = match (is_param(a), is_param(b)) {
        (true, false) => b,
        (false, true) => a,
        _ => return None,
    };
    (value.reads_same() && !value.mentions_var(param)).then(|| (**value).clone())
}

/// Whether `e` is an array as it's made: `[a, b]`, `Array.from(x)`, a
/// string's `split` or a helper's, and through an array's methods that make one,
/// `.filter(f)`, not an iterator's of the same names, `$iter(xs).filter(f)`.
pub(super) fn made_array(e: &Expr) -> bool {
    match &e.kind {
        js::ExprKind::Array(_) => true,
        // A slice's items in a range in bounds, `items.slice(a, b)` (ADR 0806).
        _ if e.facts.has(js::Known::Sliced) => true,
        // An `Option`'s items, `o == null ? [] : [o]`.
        js::ExprKind::Cond(..) => fresh(e),
        // A helper's that makes one, `$windows(v, 2)`.
        js::ExprKind::Call(callee, _) if matches!(callee.kind, js::ExprKind::Var(_)) => fresh(e),
        js::ExprKind::Call(callee, _) => match &callee.kind {
            js::ExprKind::Member(object, name) => match name.as_str() {
                "split" => true,
                "from" => matches!(&object.kind, js::ExprKind::Var(v) if v == "Array"),
                "map" | "filter" | "slice" | "toReversed" | "toSorted" | "concat" | "flat" | "flatMap" => {
                    made_array(object)
                }
                _ => false,
            },
            _ => false,
        },
        _ => false,
    }
}

/// Whether `e` is `$iter(items)`, `stepped_items`'s.
/// `$iter(v).map(f)`, a map of an array's items, made and not yet stepped:
/// nothing runs till it is.
pub(in crate::lower) fn is_items_map(e: &Expr) -> bool {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && let [f] = args.as_slice()
        && let js::ExprKind::Member(source, method) = &callee.kind
        && let js::ExprKind::Call(iter, items) = &source.kind
    {
        method == "map"
            && f.reads_only_vars()
            && iter.helper_name() == Some("$iter")
            && items.iter().all(Expr::reads_only_vars)
    } else {
        false
    }
}

pub(super) fn is_stepped_items(e: &Expr) -> bool {
    fresh_iter_source(e).is_some()
}

/// The runtime's helpers that make a JS iterator: `$iterator`, `$given`, `$intoIter`, `$cycleOf`, `$repeating` and `$repeatingN` (ADR 0513),
/// `$rev` (ADR 0516), `$closing` (ADR 0518), `$mapEnds` (ADR 0455),
/// `$filterEnds` (ADR 0526), `$enumerate` (ADR 0528), `$taking` and
/// `$skipping` (ADR 0542), `$memchrIter` (ADR 0529), `$stdIterator` (ADR
/// 0561), `$lazyScan` (ADR 0517) and `$lazyMapWhile`, not fused, and each of
/// its generators,
/// which the runtime package's test holds this list to; a range's, its own.
pub(crate) const ITERATOR_HELPERS: [&str; 31] = [
    "$stdIterator",
    "$memchrIter",
    "$mapEnds",
    "$flattenEnds",
    "$filterEnds",
    "$enumerate",
    "$stepBy",
    "$taking",
    "$skipping",
    "$iterator",
    "$given",
    "$intoIter",
    "$cycle",
    "$cycleOf",
    "$repeating",
    "$repeatingN",
    "$rev",
    "$closing",
    super::std_types::range::LAZY_RANGE,
    super::std_types::range::LAZY_RANGE_BACK,
    "$lazyTakeWhile",
    "$lazySkipWhile",
    "$lazyScan",
    "$lazyMapWhile",
    "$lazyChain",
    "$lazyZip",
    "$zipBack",
    "$flatMap",
    "$repeatingWith",
    "$successors",
    "$keysIn",
];

/// Whether `e` is a JS iterator as it's made: `Iterator.from(..)`, a
/// `$iter`, an `ITERATOR_HELPERS`' own, a range's `$iterator` or a
/// `$lazyChain`'s generator, a collection's `values()`, `keys()` or
/// `entries()`, or an iterator helper's of one, `.map(f)` of a range's.
pub(super) fn is_js_iterator(e: &Expr) -> bool {
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return false;
    };
    if let Some(helper) = callee.helper_name() {
        return ITERATOR_HELPERS.contains(&helper) || is_stepped_items(e);
    }
    match &callee.kind {
        // Not `Object.keys(o)`'s, an array.
        js::ExprKind::Member(object, name) => match &object.kind {
            js::ExprKind::Var(v) if v == "Object" => false,
            js::ExprKind::Var(v) if v == "Iterator" => name == "from",
            _ if matches!(name.as_str(), "map" | "filter" | "flatMap" | "take" | "drop") => is_js_iterator(object),
            _ => matches!(name.as_str(), "values" | "keys" | "entries"),
        },
        _ => false,
    }
}

/// What `Iterator.from(x)` makes a JS iterator of, `x`: `Ok` of it, or
/// `Err` of `e` itself, of anything else.
pub(super) fn from_iterator(e: Expr) -> Result<Expr, Expr> {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && matches!(&callee.kind, js::ExprKind::Member(object, name)
                    if name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")) =>
        {
            Ok(args.pop().expect("one"))
        }
        kind => Err(Expr {
            facts: js::Facts::NONE,
            kind,
            span: e.span,
        }),
    }
}

/// A range's numbers as `stepped_by` steps them, `$iterator({ start: a,
/// end: b }, $rangeNext)`: its bounds, whether its end is in it, `a..=b`'s,
/// and whether it's stepped from its end.
pub(super) struct SteppedRange<'e> {
    pub(super) start: &'e Expr,
    pub(super) end: &'e Expr,
    pub(super) inclusive: bool,
    pub(super) back: bool,
}

pub(super) fn stepped_range(e: &Expr) -> Option<SteppedRange<'_>> {
    let js::ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let (Some(iterator), [range, next]) = (callee.helper_name(), &args[..]) else {
        return None;
    };
    let (js::ExprKind::Object(bounds, ..), Some(next)) = (&range.kind, next.helper_name()) else {
        return None;
    };
    let [js::Prop::Field(s, start), js::Prop::Field(f, end)] = &bounds[..] else {
        return None;
    };
    let (inclusive, back) = [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .find(|&(inclusive, back)| {
            let kind = if inclusive {
                RangeKind::Inclusive
            } else {
                RangeKind::Exclusive
            };
            kind.step(back) == Some(next)
        })?;
    (iterator == "$iterator" && (s.as_str(), f.as_str()) == ("start", "end")).then_some(SteppedRange {
        start,
        end,
        inclusive,
        back,
    })
}

/// What `for (x of e)` steps through of `e`, `$iter(xs)`'s or
/// `Iterator.from(xs)`'s: `xs` itself.
pub(super) fn iterated(e: Expr) -> Expr {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && match &callee.kind {
                    _ if callee.helper_name().is_some() => callee.helper_name() == Some("$iter"),
                    js::ExprKind::Member(object, from) => {
                        from == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")
                    }
                    _ => false,
                } =>
        {
            args.pop().expect("one")
        }
        kind => Expr {
            facts: js::Facts::NONE,
            kind,
            span: e.span,
        },
    }
}

/// The keys `for (const key in object)` visits, `js::keys_in(object)`'s:
/// `$keysIn(object)`, a JS iterator of them, which a `for` loop of it is
/// that loop (ADR 0719).
pub(in crate::lower) fn keys_in(object: Expr) -> Expr {
    Expr::call(Expr::helper("$keysIn"), vec![object])
}

/// What `e`, a `keys_in` one, steps the keys of: `object` of
/// `$keysIn(object)`. The one reader of its form.
pub(in crate::lower) fn keys_in_source(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$keysIn") && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// What `it`, an iterator made here of an array, `$iter(v)`, steps: `v`,
/// all of it left.
pub(super) fn fresh_iter_source(it: &Expr) -> Option<&Expr> {
    match &it.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$iter") && args.len() == 1 => args.first(),
        _ => None,
    }
}

/// `$iter(items)`, `stepped_items`'s, where no `FnCx` is.
pub(super) fn array_stepped(items: Expr) -> Expr {
    Expr::call(Expr::helper("$iter"), vec![items])
}

/// `it.enumerate()`: each item and its index, `(x, i) => [i, x]`, in
/// place of which a callback that takes the pair apart is given each item
/// and its index, of an array (`eager_chains`).
pub(in crate::lower) fn enumerated(it: Expr) -> Expr {
    Expr::call(Expr::member(it, "map"), vec![enumerating()])
}

/// `(x, i) => [i, x]`, what `enumerated` maps each item by.
fn enumerating() -> Expr {
    let pair = Expr::array(vec![Expr::var("i"), Expr::var("x")]);
    Expr::arrow(
        vec!["x".into(), "i".into()],
        vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
    )
}

/// Whether `e`, of a `Rev`, an `Enumerate` or a `Map`, is what the
/// recognizers made of it in place, never std's struct, which one held is:
/// as the call that took it marked it (`Known::InPlace`, ADR 0871).
pub(in crate::lower) fn made_in_place(e: &Expr) -> bool {
    e.facts.has(js::Known::InPlace)
}

/// What `e`, an `enumerated` one, enumerates: `src` of `src.map((x, i) =>
/// [i, x])`. The one reader of its form.
pub(in crate::lower) fn enumeration_source(e: &Expr) -> Option<&Expr> {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(source, method) = &callee.kind
                && method == "map"
                && args.len() == 1
                && args[0] == enumerating() =>
        {
            Some(source)
        }
        _ => None,
    }
}

/// A statement of an iterator of an array made here, `$iter(v.splice(0,
/// i));` of a `drain(..i)` nothing steps: the array's making alone, as
/// what's made of it does nothing more (ADR 0881).
pub(in crate::lower) fn discarded_iterator(e: &mut Expr) {
    if let js::ExprKind::Call(callee, args) = &mut e.kind
        && callee.helper_name() == Some("$iter")
        && let [items] = args.as_mut_slice()
        && fresh(items)
    {
        *e = Expr {
            span: e.span,
            ..std::mem::replace(items, Expr::undefined())
        };
    }
}

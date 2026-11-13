//! More adapters and `Vec`'s methods: `dedup`, `resize`, a flat map, a
//! closed inner iterator, and `iter_comb`, which lowers an adapter.

use super::*;
use crate::lower::recognition::{OwnStep, Reach};
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The helper of items a closure takes `&mut`: of objects, `objects`;
    /// of what a `&mut` to is a cell, a number or text, `cells` (ADR 0315).
    pub(super) fn by_cells(&self, items: Ty<'tcx>, objects: &'static str, cells: &'static str) -> &'static str {
        // As a `&mut` to one is given, the closure's argument: a cell where
        // its form is one, an `Rc`'s, though it's an object (#906).
        match self
            .slice_item(items)
            .is_some_and(|item| self.mut_ref_form(item).is_cell())
        {
            true => cells,
            false => objects,
        }
    }

    /// The function that drops each item a slice, an array or a `Vec` of
    /// type `ty` holds, where they have a destructor: what `truncate`,
    /// `retain`, `dedup_by` and `resize_with` drop what they remove with,
    /// last (ADR 0098).
    pub(in crate::lower) fn items_drop(&mut self, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let item = self.slice_item(ty).expect("a slice's, an array's or a `Vec`'s items");
        self.std_drop_function(item, span)
    }

    /// `v.dedup()`, std's `dedup_by(|a, b| a == b)`: `$dedupBy(v, eq, drop)`,
    /// or of what `===` compares and drops nothing, `$dedup(v)` (ADR 0507).
    pub(super) fn dedup_call(&mut self, items: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let item = (self.slice_item(ty)).ok_or_else(|| self.unsupported(span, "`dedup` of this"))?;
        Ok(match self.items_drop(ty, span)? {
            None if self.eq_is_identity(item) => Expr::call(Expr::helper("$dedup"), vec![items]),
            drop => {
                let same = self.eq_fn(item, span)?;
                Expr::call(
                    Expr::helper("$dedupBy"),
                    Expr::optional_args(vec![items, same], vec![drop]),
                )
            }
        })
    }

    /// `v.resize(n, x)`, `list`: each new item its own, a clone, but the
    /// last, which is `x`, as std moves it; one whose copies can't be told
    /// apart, it. Of items with a destructor, what it cuts off and then `x`
    /// dropped, as std's are (ADR 0507); room for them asked first, where
    /// it may not fit (ADR 0774).
    pub(super) fn resize_call(&mut self, list: Vec<Expr>, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let item = (self.slice_item(ty)).ok_or_else(|| self.unsupported(span, "`resize` of this"))?;
        let clone = match self.needs_clone(item) {
            true => Some(self.clone_fn("item", item, span)?),
            false => None,
        };
        let drop = self.items_drop(ty, span)?;
        let size = self.room_size_of(&list[1], item);
        Ok(Expr::call(
            Expr::helper("$resize"),
            Expr::optional_args(list, vec![size, clone, drop]),
        ))
    }

    /// `v.resize_with(n, f)`, `list`: `$resizeWith(v, n, f)`, room for `n`
    /// items asked first, where it may not fit (ADR 0774), and what it cuts
    /// off dropped.
    pub(super) fn resize_with_call(&mut self, list: Vec<Expr>, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let item = (self.slice_item(ty)).ok_or_else(|| self.unsupported(span, "`resize_with` of this"))?;
        let size = self.room_size_of(&list[1], item);
        let drop = self.items_drop(ty, span)?;
        Ok(Expr::call(
            Expr::helper("$resizeWith"),
            Expr::optional_args(list, vec![size, drop]),
        ))
    }

    /// `inserted` of a `Vec`'s `insert` its body's ranges don't show in
    /// bounds, of type `ty`, given its items' drop, which drops what it took
    /// as an index past the end panics (#1171).
    pub(in crate::lower) fn inserted_dropping(
        &mut self,
        subject: Expr,
        at: Expr,
        value: Expr,
        (ty, span): (Ty<'tcx>, Span),
    ) -> R<Expr> {
        let drop = self.items_drop(ty, span)?;
        Ok(inserted(subject, at, value, false, drop))
    }

    /// `list`, a call's arguments, and last, `items_drop` of `ty`, where it
    /// has one.
    pub(in crate::lower) fn dropping(&mut self, mut list: Vec<Expr>, ty: Ty<'tcx>, span: Span) -> R<Vec<Expr>> {
        list.extend(self.items_drop(ty, span)?);
        Ok(list)
    }

    /// What a slice, an array or a `Vec` holds.
    pub(in crate::lower) fn slice_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = ty.peel_refs();
        match ty.kind() {
            ty::Slice(item) | ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// Does `includes` find a `ty` as `==` does, `js_eq`'s SameValueZero?
    pub(in crate::lower) fn eq_is_identity(&self, ty: Ty<'tcx>) -> bool {
        self.js_eq(ty) == Some(crate::lower::std_impls::JsEq::SameValue)
    }

    /// `a === b` of `ty`s JS's `===` compares as Rust's `==` does
    /// (`js_eq`, or by address): of what's never a NaN, text or an object
    /// equal only to itself, known so, as an array's `includes` finds what
    /// it does (`Known::NeverNaN`, rd353).
    pub(in crate::lower) fn strict_eq(&self, ty: Ty<'tcx>, a: Expr, b: Expr) -> Expr {
        let mut same = Expr::bin(js::Op::Eq, a, b);
        if self.eq_is_identity(ty) || self.eq_by_address(ty) {
            same.facts = same.facts.with(js::Known::NeverNaN);
        }
        same
    }

    /// The drop of a `receiver`'s items, an iterator's, where they have a
    /// destructor.
    fn item_drop_of(&mut self, receiver: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        match self.iterator_item(receiver.peel_refs()) {
            Some(item) => self.std_drop_function(item, span),
            None => Ok(None),
        }
    }

    /// `item_drop_of`'s, run on `item`, as `std_drop_call` runs it.
    fn item_drop_call(&mut self, receiver: Ty<'tcx>, item: Expr, span: Span) -> R<Option<Expr>> {
        match self.iterator_item(receiver.peel_refs()) {
            Some(ty) => self.std_drop_call(ty, item, span),
            None => Ok(None),
        }
    }

    /// What `flat_map(f)` and `flatten()`, of no `f`, give JS's `flatMap`
    /// of an `inner`, an `IntoIterator` that owns items with a destructor:
    /// `(i) => $closing($iter(..), drop)`, each closed to drop what it
    /// hasn't given, as `flatMap` closes it where it's closed early, and
    /// Rust drops what's left of it with the adapter (ADR 0510); `f`'s own
    /// parameter where it's an arrow of one value. `None` of one with
    /// nothing to drop.
    /// What `flat_map` and `flatten` step of an inner `value` of type
    /// `inner`, as a JS iterable: a range's numbers, the crate's iterator's
    /// items and an escape's `char`s, `as_js_iterator`'s, as `chain` steps
    /// them; a `Result`'s `Ok` value or none; a B-tree's, in order; else
    /// the value, an array's or a JS iterator's own. `None` where it's the
    /// value.
    fn inner_iterable(&mut self, value: Expr, inner: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        // An associated type's, `I::Item`'s, its dictionary's (#278).
        if let Some(items) = self.items_by_dictionary(value.clone(), inner, span)? {
            return Ok(Some(items));
        }
        if self.range_kind(inner).is_some() || self.is_user_iterator(inner) {
            return self.as_js_iterator(value, inner, span).map(Some);
        }
        if self.is_std_type(inner, crate::lower::recognition::StdItem::Result) {
            return Ok(Some(self.result_items(value, inner)));
        }
        // A B-tree's, in its keys' order: a JS `Set` a caller gives may be in
        // any (ADR 0404, amended).
        if self.is_sorted(inner) {
            return self.in_order_of(value, inner, span).map(Some);
        }
        Ok(None)
    }

    /// The items of `result`, a `Result` of type `ty`: its `Ok`'s value, by
    /// the one test of it, or none; a write's result's, `undefined` or its
    /// error, its `()` (#1410).
    pub(in crate::lower) fn result_items(&self, result: Expr, ty: Ty<'tcx>) -> Expr {
        let ty::Adt(adt, _) = *ty.kind() else {
            unreachable!("a `Result`")
        };
        let ok = self.variant_test(result.clone(), ty, adt, rustc_abi::VariantIdx::ZERO, false);
        let held = match self.is_write_result(ty) {
            true => Expr::undefined(),
            false => crate::js::result::held(result),
        };
        Expr::cond(ok, Expr::array(vec![held]), Expr::array(Vec::new()))
    }

    /// `(item) => <inner_iterable of f(item)>`, of `f`'s returned value,
    /// where it needs making one; `None` where it's the value.
    fn iterable_of_each(&mut self, f: Option<Expr>, inner: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let marks = f
            .as_ref()
            .map_or(js::Facts::NONE, |f| crate::lower::iterators::its_facts(f.facts));
        let (param, value) = match f {
            Some(f) => match &f.kind {
                js::ExprKind::Arrow(params, body)
                    if let [js::Pattern::Name(name)] = params.as_slice()
                        && let [
                            Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] = body.as_slice() =>
                {
                    (name.clone(), value.clone())
                }
                _ => ("item".to_string(), Expr::call(f, vec![Expr::var("item")])),
            },
            None => ("item".to_string(), Expr::var("item")),
        };
        // A `Result`'s is read twice, its tag and its value: a variable.
        let twice = self.is_std_type(inner, crate::lower::recognition::StdItem::Result);
        let held = (twice && !matches!(value.kind, js::ExprKind::Var(_))).then(|| value.clone());
        let read = if held.is_some() { Expr::var("inner") } else { value };
        let Some(iterable) = self.inner_iterable(read, inner, span)? else {
            return Ok(None);
        };
        let mut body = Vec::new();
        if let Some(held) = held {
            body.push(StmtKind::Const("inner".into(), held).at(js::Span::NONE));
        }
        body.push(StmtKind::Return(Some(iterable)).at(js::Span::NONE));
        let mut each = Expr::arrow(vec![param.into()], body);
        each.facts = marks;
        Ok(Some(each))
    }

    fn closed_inner(&mut self, f: Option<Expr>, inner: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some(drop) = self.closing_drop(inner, span)? else {
            return Ok(None);
        };
        let marks = f
            .as_ref()
            .map_or(js::Facts::NONE, |f| crate::lower::iterators::its_facts(f.facts));
        let (param, value) = match f {
            Some(f) => match &f.kind {
                js::ExprKind::Arrow(params, body)
                    if let [js::Pattern::Name(name)] = params.as_slice()
                        && let [
                            Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ] = body.as_slice() =>
                {
                    (name.clone(), value.clone())
                }
                _ => ("item".to_string(), Expr::call(f, vec![Expr::var("item")])),
            },
            None => ("item".to_string(), Expr::var("item")),
        };
        // An associated type's, `I::Item`'s, its dictionary's `into_iter`,
        // `as_js_iterator`'s (#278): a where clause's `Self::Item:
        // IntoIterator` of a copied default, whose items are a `Vec`'s, not
        // a JS iterator (#829, #1109).
        let it = match self.slice_item(inner) {
            Some(_) => self.stepped_items(value),
            None => self.as_js_iterator(value, inner, span)?,
        };
        let closed = self.closed_by(it, drop, inner);
        let mut each = Expr::arrow(
            vec![param.into()],
            vec![StmtKind::Return(Some(closed)).at(js::Span::NONE)],
        );
        each.facts = marks;
        Ok(Some(each))
    }

    /// `items.flatMap(each)` of inner iterators of type `inner`, made by
    /// `each`; where `own` says, each closing to drop what it holds or one
    /// whose `count` is the crate's own (#736), or of items with a
    /// destructor, `$flatMap(items, each)`: its source
    /// closed first, as Rust drops a `Flatten`'s fields, its source before
    /// the inner iterator it's in, where JS's `flatMap` closes the inner one
    /// first, whatever the source is, an adapter of what it drops too
    /// (#644); its `last` std's, each inner iterator's own, which drops
    /// what it passes before the one kept before (#645) (ADR 0510); and its
    /// `count` std's, each inner iterator's own.
    fn flat_mapped(&self, items: Expr, each: Expr, inner: Ty<'tcx>, own: bool) -> Expr {
        let gives = self.iterable_item(inner);
        match own || self.drops(gives) != crate::lower::drops::Drops::Nothing {
            true => Expr::call_helper("$flatMap", vec![items, each]),
            false => Expr::call(Expr::member(items, "flatMap"), vec![each]),
        }
    }

    /// `a.zip(b)` of `a` and `b`, JS iterators of `IntoIterator`s of those
    /// types: of sides std
    /// reads by index, a step of `a` running code, `$trustedZip(a, b)`, `b`
    /// knowing its length, as std pairs them up to the shorter's (ADR 0973);
    /// else `$lazyZip(a, b)`, of items with a destructor, each of `a`'s it
    /// steps where `b`'s run out dropped (ADR 0465).
    pub(in crate::lower) fn zipped(&mut self, a: (Expr, Ty<'tcx>), b: (Expr, Ty<'tcx>), span: Span) -> R<Expr> {
        let ((a, a_ty), (b, b_ty)) = (a, b);
        if crate::lower::recognition::zips_by_index(self.tcx, self.typing_env, a_ty, b_ty) {
            let b_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, b_ty);
            let b = self.as_exact_size(b, b_ty, false, span)?;
            return Ok(Expr::call_helper("$trustedZip", vec![a, b]));
        }
        let mut args = vec![a, b];
        args.extend(self.item_drop_of(a_ty, span)?);
        Ok(Expr::call_helper("$lazyZip", args))
    }

    /// One of `IterComb`'s, on `items`, a JS iterator (ADR 0517).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn iter_comb(
        &mut self,
        comb: IterComb,
        items: Expr,
        mut rest: std::vec::IntoIter<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        (receiver_ty, output): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A flatten's, whose `count` is each inner iterator's own, the
        // crate's where one writes it (#736): `$flatMap`, which counts them so.
        let own_count = self.recognition().reaches_its_own(output, OwnStep::Count, Reach::Sure);
        let mut next = || rest.next().expect("rustc checked the arguments");
        let method = |items: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(items, name), list);
        // Whether the item is `Some`, `item != null`, whatever it holds
        // (ADR 0298, amended).
        let kept = generic_args.types().nth(1);
        let present = || {
            let test = Expr::presence(Expr::var("item"), true);
            Expr::arrow(
                vec!["item".into()],
                vec![StmtKind::Return(Some(test)).at(js::Span::NONE)],
            )
        };
        let item_ty = || self.iterator_item(receiver_ty);
        // Of a kept value that looks like `None`, each `Some` boxed, taken
        // out of its box (ADR 0515).
        let unboxed = |items: Expr, payload: Option<Ty<'tcx>>| match payload.is_some_and(|ty| self.boxed_payload(ty)) {
            true => method(items, "map", vec![Expr::helper("$someValue")]),
            false => items,
        };
        Ok(match comb {
            // `Some`s only: `.map(f).filter((item) => item != null)`; each
            // taken out of its box where it looks like `None` (ADR 0515).
            IterComb::FilterMap => unboxed(
                method(method(items, "map", vec![next()]), "filter", vec![present()]),
                kept,
            ),
            // Its `Option`, a `Some` boxed where what it holds looks like `None`.
            IterComb::FindMap => method(method(items, "map", vec![next()]), "find", vec![present()]),
            IterComb::FlatMap => {
                // A closure that returns an `Option` is a `filter_map`.
                let returned = generic_args.type_at(1);
                if let Some((_, payload, true)) = self.recognition().flattened_options(output) {
                    unboxed(
                        method(method(items, "map", vec![next()]), "filter", vec![present()]),
                        Some(payload),
                    )
                } else {
                    let f = next();
                    match self.closed_inner(Some(f.clone()), returned, span)? {
                        Some(each) => self.flat_mapped(items, each, returned, true),
                        None => match self.iterable_of_each(Some(f.clone()), returned, span)? {
                            Some(each) => self.flat_mapped(items, each, returned, own_count),
                            None => self.flat_mapped(items, f, returned, own_count),
                        },
                    }
                }
            }
            // Of `Option`s, through a reference too, `iter()`'s `&Some(x)`,
            // which is `x`: the `Some`s, each taken out of its box where it
            // looks like `None` (ADR 0515).
            IterComb::Flatten => match item_ty() {
                Some(_) if let Some((_, payload, false)) = self.recognition().flattened_options(output) => {
                    unboxed(method(items, "filter", vec![present()]), Some(payload))
                }
                // Of what owns items with a destructor, each closed (ADR 0510).
                Some(item) if let Some(each) = self.closed_inner(None, item, span)? => {
                    self.flat_mapped(items, each, item, true)
                }
                // Of arrays, each stepped as an array is, `$iter(item)`: what
                // `flat()` takes apart where the chain is made eager.
                Some(item)
                    if {
                        let item = item.peel_refs();
                        item.is_array() || item.is_slice() || self.is_vec_like(item)
                    } =>
                {
                    let stepped = self.stepped_items(Expr::var("item"));
                    let each = Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(stepped)).at(js::Span::NONE)],
                    );
                    self.flat_mapped(items, each, item, own_count)
                }
                Some(item) if let Some(each) = self.iterable_of_each(None, item, span)? => {
                    self.flat_mapped(items, each, item, own_count)
                }
                _ => {
                    let each = Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(Expr::var("item"))).at(js::Span::NONE)],
                    );
                    method(items, "flatMap", vec![each])
                }
            },
            IterComb::Zip => {
                let b = next();
                self.zipped((items, receiver_ty), (b, generic_args.type_at(1)), span)?
            }
            IterComb::Chain => Expr::call(Expr::helper("$lazyChain"), vec![items, next()]),
            // Not fused, as Rust's `MapWhile` isn't: `$lazyMapWhile`.
            IterComb::MapWhile => unboxed(
                Expr::call(Expr::helper("$lazyMapWhile"), vec![items, next(), present()]),
                kept,
            ),
            // And the drop of the item it stops at, or of each it skips, of
            // items with a destructor (#651, #817).
            IterComb::TakeWhile | IterComb::SkipWhile => {
                let mut args = vec![items, next()];
                args.extend(rest.next());
                let helper = if comb == IterComb::TakeWhile {
                    "$lazyTakeWhile"
                } else {
                    "$lazySkipWhile"
                };
                Expr::call(Expr::helper(helper), args)
            }
            // Lazy of a JS iterator, as Rust's is (ADR 0517).
            // Its state's `&mut` as `handles_mut` says, the object itself of
            // a struct or a tuple.
            // And of a function or a state with a destructor, what drops
            // them, given its state, once it's closed, after its source, as
            // `Scan { iter, f, state }` drops its fields (#760).
            IterComb::Scan => {
                let (init, f) = (next(), next());
                let (state_ty, function_ty) = (generic_args.types().nth(1), generic_args.types().nth(3));
                let drops =
                    |ty: Option<Ty<'tcx>>| ty.filter(|&ty| self.drops(ty) != crate::lower::drops::Drops::Nothing);
                let (state_drops, function_drops) = (drops(state_ty), drops(function_ty));
                let f = match (&f.kind, function_drops) {
                    (js::ExprKind::Var(_), _) | (_, None) => f,
                    _ => self.spill(Role::F, f, out),
                };
                let mut dropped = Vec::new();
                if let Some(function) = function_drops {
                    self.drop_value(f.clone(), function, span, &mut dropped)?;
                }
                if let Some(state) = state_drops {
                    self.drop_value(Expr::var("state"), state, span, &mut dropped)?;
                }
                let handle = state_ty.is_none_or(|state| self.handles_mut(state));
                let mut args = vec![items, init, f];
                if !handle || !dropped.is_empty() {
                    args.push(Expr::bool(handle));
                }
                if !dropped.is_empty() {
                    let param = if state_drops.is_some() { "state" } else { "_" };
                    args.push(Expr::arrow(vec![param.into()], dropped));
                }
                // Each `Some` taken out of its box, as `unboxed` does.
                let scanned = Expr::call(Expr::helper("$lazyScan"), args);
                match generic_args.types().nth(2).is_some_and(|ty| self.boxed_payload(ty)) {
                    true => method(scanned, "map", vec![Expr::helper("$someValue")]),
                    false => scanned,
                }
            }
            // Of items with a destructor, each it steps over dropped as it's
            // passed, as std's `nth` drops it (ADR 0465).
            IterComb::StepBy => {
                // Not 0, as std asserts as it's given: `$step` checks one that may be.
                let n = next();
                let n = match n.as_int() {
                    Some(step) if step != 0 => n,
                    _ => self.spill(Role::Step, Expr::call(Expr::helper("$step"), vec![n]), out),
                };
                // Of a source no array's items bound (`bounded_by_items`),
                // by its `nth`, as std's `StepBy` steps it: one that may give
                // more after a `None`, which a `None` stops (#161); one whose
                // `nth` reaches the crate's own, which std asks (#214); a
                // range's numbers, whose `nth` is of its bounds, which tidy
                // makes `$lazyRange(a, b, n)` where it's made there (#231);
                // a zip's, whose fold, std's `SpecFold`, stops where the
                // shorter ends (#654); a flatten's, what its inner iterator
                // hasn't given dropped after its source as a panic unwinds
                // (#668); and its hint std's, its source's count `n` apart,
                // which a collect asks its room by, where a filter's is none
                // at least (#1046).
                if !self.recognition().bounded_by_items(receiver_ty)
                    // And of a `Vec`'s reversed, whose `nth` drops what it
                    // passes over as one slice (#234), or of one whose first
                    // step asks a `Skip`'s `nth` of one (#294).
                    || self.recognition().nths_a_slice(receiver_ty)
                    || self.recognition().steps_past_a_slice(receiver_ty)
                    // And of one whose fold moves a function an adapter holds
                    // with a destructor, which a filter's fold would drop
                    // before what's left, where std's `StepBy` steps it by
                    // `nth`, so it drops after (#710).
                    || self.folds_held_function(receiver_ty.peel_refs())
                    // And of a `Vec`'s, whose `nth` drops them as one slice
                    // in order, of items whose drop may panic, past which
                    // the slice's goes on (#668).
                    || (self.iterator_item(receiver_ty.peel_refs())).is_some_and(|item| self.drop_may_panic(item))
                        && self.recognition().nths_a_slice_in_order(receiver_ty)
                    // And of a source whose steps may panic: std's `StepBy`
                    // takes its first `next` while its fold's frame holds only
                    // its parameters, so what drops with the consumer's `f`
                    // drops before the accumulator, `$stepBy`'s
                    // `$stepHolding`, where a filter's first step is inside
                    // its source's fold, which drops the accumulator first
                    // (#773).
                    || self.steps_may_panic(receiver_ty.peel_refs())
                    // And of one generic code asks the hint of, given it,
                    // whose hint `$stepBy` knows as std's (#1236).
                    || self.made_knowing_its_hint(output)
                {
                    let mut list = vec![items, n];
                    list.extend(self.item_drop_of(receiver_ty, span)?);
                    return Ok(crate::lower::iterators::stepped_every(list));
                }
                let keep = Expr::bin(Op::Eq, Expr::bin(Op::Rem, Expr::var("i"), n), Expr::int(0));
                let body = match self.item_drop_call(receiver_ty, Expr::var("item"), span)? {
                    Some(dropped) => vec![
                        StmtKind::If(
                            keep,
                            vec![StmtKind::Return(Some(Expr::bool(true))).at(js::Span::NONE)],
                            None,
                        )
                        .at(js::Span::NONE),
                        StmtKind::Expr(dropped).at(js::Span::NONE),
                        StmtKind::Return(Some(Expr::bool(false))).at(js::Span::NONE),
                    ],
                    None => vec![StmtKind::Return(Some(keep)).at(js::Span::NONE)],
                };
                let param = if body.len() > 1 { "item" } else { "_" };
                method(items, "filter", vec![Expr::arrow(vec![param.into(), "i".into()], body)])
            }
            // Each passed over dropped as it's passed, of items with a
            // destructor (ADR 0098); a key made once for each, as std's.
            IterComb::MaxByKey(max) | IterComb::MaxBy(max) => {
                let item = self
                    .iterator_item(receiver_ty.peel_refs())
                    .ok_or_else(|| self.unsupported(span, "this iterator's items"))?;
                // Its items as they come, which the helper steps (#179).
                let key = match comb {
                    IterComb::MaxBy(_) => None,
                    _ => Some(
                        generic_args
                            .types()
                            .nth(1)
                            .ok_or_else(|| self.unsupported(span, "this key"))?,
                    ),
                };
                self.extreme_by_call((max, key), (items, next()), item, None, span)?
            }
            IterComb::Product => {
                let ty = generic_args
                    .types()
                    .nth(1)
                    .ok_or_else(|| self.unsupported(span, "this product"))?;
                let num = self.num(ty, span)?;
                let times = match num {
                    Num::F64 => Expr::bin(Op::Mul, Expr::var("a"), Expr::var("b")),
                    Num::I32 | Num::U32 => num.wrap(Expr::call(
                        Expr::member(Expr::js_global("Math"), "imul"),
                        vec![Expr::var("a"), Expr::var("b")],
                    )),
                    _ => num.wrap(Expr::bin(Op::Mul, Expr::var("a"), Expr::var("b"))),
                };
                let f = Expr::arrow(
                    vec!["a".into(), "b".into()],
                    vec![StmtKind::Return(Some(times)).at(js::Span::NONE)],
                );
                method(items, "reduce", vec![f, num.literal(1)])
            }
            // Of one whose `nth` reaches the crate's own, its `nth`, as
            // `next()` is `$next(it)` (#182); its `Some` boxed where the item
            // looks like `None` (ADR 0051).
            IterComb::Nth
                if self
                    .recognition()
                    .reaches_its_own(receiver_ty, OwnStep::Nth, Reach::Sure) =>
            {
                let boxed = item_ty().is_some_and(|item| self.boxed_payload(item));
                Expr::call_helper(if boxed { "$nthSome" } else { "$nth" }, vec![items, next()])
            }
            IterComb::Nth => {
                let n = next();
                let mut first = method(method(items, "drop", vec![n]), "take", vec![Expr::int(1)]);
                if item_ty().is_some_and(|item| self.boxed_payload(item)) {
                    first = method(first, "map", vec![Expr::helper("$some")]);
                }
                Expr::index(Expr::collected(first), Expr::int(0))
            }
            // Of items with a destructor and an `f` that may throw, a panic,
            // the item it was given dropped as it unwinds, as std's
            // `Inspect` holds it (#423).
            IterComb::Inspect => {
                let f = next();
                let throws = crate::lower::iterators::may_throw(&f);
                let f = if matches!(f.kind, js::ExprKind::Var(_)) {
                    f
                } else {
                    self.spill(Role::Inspect, f, out)
                };
                let item = Expr::var("item");
                let inspected = StmtKind::Expr(Expr::call(f, vec![item.clone()])).at(js::Span::NONE);
                let mut inspected = vec![inspected];
                if throws && let Some(dropped) = self.item_drop_call(receiver_ty, item.clone(), span)? {
                    let dropped = vec![StmtKind::Expr(dropped).at(js::Span::NONE)];
                    inspected = crate::lower::drops::dropped_unwinding(inspected, dropped, js::Span::NONE);
                }
                inspected.push(StmtKind::Return(Some(item)).at(js::Span::NONE));
                let each = Expr::arrow(vec!["item".into()], inspected);
                method(items, "map", vec![each])
            }
            // Of `&T`s into `partition::<B, F>`'s `B` of `T`s, after `Self`,
            // each its bits' copy where it needs one, as std's
            // `extend_one` pushes `*item` (`bits_copy_fn`, ADR 0623); of
            // items with a destructor, given their drop, which a panic in
            // `keep` drops them by (#423).
            IterComb::Partition => {
                let into = generic_args.types().nth(1).and_then(|into| self.slice_item(into));
                let copy = match self.iterator_item(receiver_ty.peel_refs()).map(|from| *from.kind()) {
                    Some(ty::Ref(_, item, _)) if into == Some(item) => self.bits_copy_fn(item),
                    _ => None,
                };
                let items = match copy {
                    Some(copy) => method(items, "map", vec![copy]),
                    None => items,
                };
                let mut args = vec![items, next()];
                args.extend(self.item_drop_of(receiver_ty, span)?);
                Expr::call(Expr::helper("$partition"), args)
            }
            // Each side a `Vec`, or a map or a set made empty and extended
            // by its items, as std's `unzip` does: `unzip::<A, B, FromA,
            // FromB>`'s last two.
            IterComb::Unzip => {
                let sides: Vec<Ty<'tcx>> = generic_args.types().skip(3).collect();
                if !sides.iter().all(|&side| self.is_vec_like(side) || self.is_map(side)) {
                    return Err(self.unsupported(span, "`unzip` into what isn't a `Vec`, a map or a set"));
                }
                // What each side collected dropped as a panic stepping it
                // unwinds, `A`'s then `B`'s (#1150).
                let mut args = vec![items];
                let drops = [generic_args.types().nth(1), generic_args.types().nth(2)]
                    .into_iter()
                    .map(|side| match side {
                        Some(side) => self.std_drop_function(side, span),
                        None => Ok(None),
                    })
                    .collect::<R<Vec<_>>>()?;
                if let Some(last) = drops.iter().rposition(Option::is_some) {
                    args.extend(
                        drops
                            .into_iter()
                            .take(last + 1)
                            .map(|drop| drop.unwrap_or_else(Expr::undefined)),
                    );
                }
                let pairs = Expr::call(Expr::helper("$unzip"), args);
                if sides.iter().all(|&side| self.is_vec_like(side)) {
                    return Ok(pairs);
                }
                let pairs = self.spill(Role::Unzipped, pairs, out);
                let mut parts = Vec::new();
                for (i, &side) in sides.iter().enumerate() {
                    let part = Expr::index(pairs.clone(), Expr::int(i as i128));
                    parts.push(match side.kind() {
                        ty::Adt(_, args) if self.is_map(side) => {
                            let set = self.is_set(side);
                            let entry = (args.types().next(), args.types().nth(1).filter(|_| !set));
                            let class = self.map_class(set, self.is_sorted(side), entry, span)?;
                            let made = Expr::new_(class, Vec::new());
                            self.map_values(
                                crate::lower::std_types::map::MapOp::Extend,
                                (vec![made, part], &[side, side]),
                                generic_args,
                                span,
                                out,
                            )?
                        }
                        _ => part,
                    });
                }
                Expr::array(parts)
            }
        })
    }
}

/// `v.insert(at, value)`, the one form of it each way it's lowered: at the
/// front, which no length refuses, `v.unshift(value)`; where the body's
/// ranges show `at` at most the length (`in_bounds`, ADR 0576),
/// `v.splice(at, 0, value)`; else `$insertAt`, which refuses an index past
/// the end, dropping `value` by its item's `drop` as it panics, as Rust's
/// `insert` drops what it took (#1171).
pub(in crate::lower) fn inserted(subject: Expr, at: Expr, value: Expr, in_bounds: bool, drop: Option<Expr>) -> Expr {
    // An item, never left out, as `splice` counts them (ADR 0330, amended).
    let value = value.given_item();
    match at.as_int() {
        Some(0) => Expr::call(Expr::member(subject, "unshift"), vec![value]),
        _ if in_bounds => Expr::call(Expr::member(subject, "splice"), vec![at, Expr::int(0), value]),
        _ => Expr::call(
            Expr::helper("$insertAt"),
            [vec![subject, at, value], drop.into_iter().collect()].concat(),
        ),
    }
}

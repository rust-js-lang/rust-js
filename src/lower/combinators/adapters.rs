//! More adapters and `Vec`'s methods: `dedup`, `resize`, a flat map, a
//! closed inner iterator, and `iter_comb`, which lowers an adapter.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The helper of items a closure takes `&mut`: of objects, `objects`;
    /// of what a `&mut` to is a cell, a number or text, `cells` (ADR 0315).
    pub(super) fn by_cells(&self, items: Ty<'tcx>, objects: &'static str, cells: &'static str) -> &'static str {
        match self.slice_item(items).is_some_and(|item| self.is_object(item)) {
            true => objects,
            false => cells,
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
    /// dropped, as std's are (ADR 0507).
    pub(super) fn resize_call(&mut self, list: Vec<Expr>, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let item = (self.slice_item(ty)).ok_or_else(|| self.unsupported(span, "`resize` of this"))?;
        let clone = match self.needs_clone(item) {
            true => Some(self.clone_fn("item", item, span)?),
            false => None,
        };
        let drop = self.items_drop(ty, span)?;
        Ok(Expr::call(
            Expr::helper("$resize"),
            Expr::optional_args(list, vec![clone, drop]),
        ))
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

    /// The drop of a `receiver`'s items, an iterator's, where they have a
    /// destructor.
    fn item_drop_of(&mut self, receiver: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        match self.iterator_item(receiver.peel_refs()) {
            Some(item) => self.std_drop_function(item, span),
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
        let inner = self.reveal(inner);
        if self.range_kind(inner).is_some()
            || self.is_user_iterator(inner)
            || crate::lower::recognition::is_text_escape(self.tcx, inner)
        {
            return self.as_js_iterator(value, inner, span).map(Some);
        }
        if self.is_std_type(inner, crate::lower::recognition::StdItem::Result) {
            let ok = crate::js::result::is(value.clone(), true);
            let items = Expr::array(vec![crate::js::result::held(value)]);
            return Ok(Some(Expr::cond(ok, items, Expr::array(Vec::new()))));
        }
        // A B-tree's, in its keys' order, which its JS map isn't (ADR 0059).
        if self.is_sorted(inner) {
            return self.in_order_of(value, inner, span).map(Some);
        }
        Ok(None)
    }

    /// `(item) => <inner_iterable of f(item)>`, of `f`'s returned value,
    /// where it needs making one; `None` where it's the value.
    fn iterable_of_each(&mut self, f: Option<Expr>, inner: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
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
        let twice = self.is_std_type(self.reveal(inner), crate::lower::recognition::StdItem::Result);
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
        Ok(Some(Expr::arrow(vec![param.into()], body)))
    }

    fn closed_inner(&mut self, f: Option<Expr>, inner: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some(drop) = self.closing_drop(inner, span)? else {
            return Ok(None);
        };
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
        let it = match self.slice_item(inner) {
            Some(_) => self.stepped_items(value),
            None => self.as_js_iterator(value, inner, span)?,
        };
        let closed = Expr::call_helper("$closing", vec![it, drop]);
        Ok(Some(Expr::arrow(
            vec![param.into()],
            vec![StmtKind::Return(Some(closed)).at(js::Span::NONE)],
        )))
    }

    /// `items.flatMap(each)`, `each` closing each inner iterator, of a
    /// `source` that may hold what has a destructor too: closed first, as
    /// Rust drops a `FlatMap`'s source before its inner iterator, where
    /// JS's `flatMap` closes the inner one first, `$flatMap(items, each)`
    /// (ADR 0510).
    fn flat_mapped(&mut self, items: Expr, each: Expr, source: Ty<'tcx>, span: Span) -> R<Expr> {
        Ok(match self.closing_drop(source, span)? {
            Some(_) => Expr::call_helper("$flatMap", vec![items, each]),
            None => Expr::call(Expr::member(items, "flatMap"), vec![each]),
        })
    }

    /// One of `IterComb`'s, on `items`, a JS iterator (ADR 0517).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower) fn iter_comb(
        &mut self,
        comb: IterComb,
        items: Expr,
        mut rest: std::vec::IntoIter<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        receiver_ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut next = || rest.next().expect("rustc checked the arguments");
        let method = |items: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(items, name), list);
        // Whether the item is `Some`: by its truth where it's never falsy,
        // `.filter(Boolean)` once prepared (ADR 0298).
        let kept = generic_args.types().nth(1);
        let present = || {
            let test = match kept {
                Some(kept) => self.present(Expr::var("item"), kept),
                None => Expr::bin(Op::LooseNe, Expr::var("item"), Expr::null()),
            };
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
            // Of one that knows its back, its `Some`s from either end, a map
            // filtered, `$mapEnds` and `$filterEnds` (ADR 0526).
            IterComb::FilterMap if self.knows_its_back(receiver_ty) => {
                let some = Expr::call_helper(
                    "$filterEnds",
                    vec![Expr::call_helper("$mapEnds", vec![items, next()]), present()],
                );
                match kept.is_some_and(|ty| self.boxed_payload(ty)) {
                    true => Expr::call_helper("$mapEnds", vec![some, Expr::helper("$someValue")]),
                    false => some,
                }
            }
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
                if let Some(payload) = self.option_of(returned) {
                    unboxed(
                        method(method(items, "map", vec![next()]), "filter", vec![present()]),
                        Some(payload),
                    )
                } else {
                    let f = next();
                    match self.closed_inner(Some(f.clone()), returned, span)? {
                        Some(each) => self.flat_mapped(items, each, receiver_ty, span)?,
                        None => match self.iterable_of_each(Some(f.clone()), returned, span)? {
                            Some(each) => method(items, "flatMap", vec![each]),
                            None => method(items, "flatMap", vec![f]),
                        },
                    }
                }
            }
            // Of `Option`s, through a reference too, `iter()`'s `&Some(x)`,
            // which is `x`: the `Some`s, each taken out of its box where it
            // looks like `None` (ADR 0515).
            IterComb::Flatten => match item_ty() {
                Some(item) if let Some(payload) = self.option_of(item.peel_refs()) => {
                    unboxed(method(items, "filter", vec![present()]), Some(payload))
                }
                // Of what owns items with a destructor, each closed (ADR 0510).
                Some(item) if let Some(each) = self.closed_inner(None, item, span)? => {
                    self.flat_mapped(items, each, receiver_ty, span)?
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
                    method(items, "flatMap", vec![each])
                }
                Some(item) if let Some(each) = self.iterable_of_each(None, item, span)? => {
                    method(items, "flatMap", vec![each])
                }
                _ => {
                    let each = Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(Expr::var("item"))).at(js::Span::NONE)],
                    );
                    method(items, "flatMap", vec![each])
                }
            },
            // Of items with a destructor, each of its own it steps where the
            // other's run out dropped (ADR 0465).
            IterComb::Zip => {
                let mut args = vec![items, next()];
                args.extend(self.item_drop_of(receiver_ty, span)?);
                Expr::call(Expr::helper("$lazyZip"), args)
            }
            IterComb::Chain => Expr::call(Expr::helper("$lazyChain"), vec![items, next()]),
            // Not fused, as Rust's `MapWhile` isn't: `$lazyMapWhile`.
            IterComb::MapWhile => unboxed(
                Expr::call(Expr::helper("$lazyMapWhile"), vec![items, next(), present()]),
                kept,
            ),
            IterComb::TakeWhile => Expr::call(Expr::helper("$lazyTakeWhile"), vec![items, next()]),
            IterComb::SkipWhile => Expr::call(Expr::helper("$lazySkipWhile"), vec![items, next()]),
            // Lazy of a JS iterator, as Rust's is (ADR 0517).
            // Its state's `&mut` as `handles_mut` says, the object itself of
            // a struct or a tuple.
            IterComb::Scan => {
                let (init, f) = (next(), next());
                let mut args = vec![items, init, f];
                if generic_args
                    .types()
                    .nth(1)
                    .is_some_and(|state| !self.handles_mut(state))
                {
                    args.push(Expr::bool(false));
                }
                unboxed(Expr::call(Expr::helper("$lazyScan"), args), generic_args.types().nth(2))
            }
            // Of items with a destructor, each it steps over dropped as it's
            // passed, as std's `nth` drops it (ADR 0465).
            IterComb::StepBy => {
                // Not 0, as std asserts as it's given: `$step` checks one that may be.
                let n = next();
                let n = match n.as_int() {
                    Some(step) if step != 0 => n,
                    _ => self.spill("step", Expr::call(Expr::helper("$step"), vec![n]), out),
                };
                // Of a source that may give more after a `None`, as std's
                // steps it, by `nth`, which a `None` stops (#161); of one
                // whose `nth` reaches the crate's own, which std asks (#214);
                // and of a range's numbers, whose `nth` is of its bounds,
                // which tidy makes `$lazyRange(a, b, n)` where it's made
                // there (#231).
                if !self.recognition().is_fused_iterator(receiver_ty.peel_refs())
                    || self.recognition().nths_its_own(receiver_ty)
                    || self.steps_a_range(receiver_ty.peel_refs())
                {
                    let mut list = vec![items, n];
                    list.extend(self.item_drop_of(receiver_ty, span)?);
                    return Ok(Expr::call(Expr::helper("$stepBy"), list));
                }
                let keep = Expr::bin(Op::Eq, Expr::bin(Op::Rem, Expr::var("i"), n), Expr::int(0));
                let body = match self.item_drop_of(receiver_ty, span)? {
                    Some(drop) => vec![
                        StmtKind::If(
                            keep,
                            vec![StmtKind::Return(Some(Expr::bool(true))).at(js::Span::NONE)],
                            None,
                        )
                        .at(js::Span::NONE),
                        StmtKind::Expr(Expr::call(drop, vec![Expr::var("item")])).at(js::Span::NONE),
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
                let f = next();
                match comb {
                    IterComb::MaxBy(_) => {
                        let helper = if max { "$maxBy" } else { "$minBy" };
                        self.extreme_by(helper, vec![items, f], item, span)?
                    }
                    _ => {
                        let key_ty = generic_args
                            .types()
                            .nth(1)
                            .ok_or_else(|| self.unsupported(span, "this key"))?;
                        if self.drops(key_ty) != crate::lower::drops::Drops::Nothing {
                            return Err(self.unsupported(span, "a key with a destructor"));
                        }
                        let compare = self.cmp_fn(key_ty, false, span)?;
                        let helper = if max { "$maxByKey" } else { "$minByKey" };
                        self.extreme_by(helper, vec![items, f, compare], item, span)?
                    }
                }
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
                        Expr::member(Expr::var("Math"), "imul"),
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
            IterComb::Nth if self.recognition().nths_its_own(receiver_ty) => {
                let boxed = item_ty().is_some_and(|item| self.boxed_payload(item));
                Expr::call_helper(if boxed { "$nthSome" } else { "$nth" }, vec![items, next()])
            }
            IterComb::Nth => {
                let n = next();
                let mut first = method(method(items, "drop", vec![n]), "take", vec![Expr::int(1)]);
                if item_ty().is_some_and(|item| self.boxed_payload(item)) {
                    first = method(first, "map", vec![Expr::helper("$some")]);
                }
                Expr::index(method(first, "toArray", Vec::new()), Expr::int(0))
            }
            IterComb::Inspect => {
                let f = next();
                let f = if matches!(f.kind, js::ExprKind::Var(_)) {
                    f
                } else {
                    self.spill("inspect", f, out)
                };
                let item = Expr::var("item");
                let each = Expr::arrow(
                    vec!["item".into()],
                    vec![
                        StmtKind::Expr(Expr::call(f, vec![item.clone()])).at(js::Span::NONE),
                        StmtKind::Return(Some(item)).at(js::Span::NONE),
                    ],
                );
                method(items, "map", vec![each])
            }
            // Of `&T`s, each a copy where it needs one (`copied_into`), into
            // `partition::<B, F>`'s `B`, after `Self`.
            IterComb::Partition => {
                let into = generic_args.types().nth(1).and_then(|into| self.slice_item(into));
                let items = match self.iterator_item(receiver_ty.peel_refs()) {
                    Some(from) => self.copied_into(items, from, into, span)?,
                    None => items,
                };
                Expr::call(Expr::helper("$partition"), vec![items, next()])
            }
            // Each side a `Vec`, or a map or a set made empty and extended
            // by its items, as std's `unzip` does: `unzip::<A, B, FromA,
            // FromB>`'s last two.
            IterComb::Unzip => {
                let sides: Vec<Ty<'tcx>> = generic_args.types().skip(3).collect();
                if !sides.iter().all(|&side| self.is_vec_like(side) || self.is_map(side)) {
                    return Err(self.unsupported(span, "`unzip` into what isn't a `Vec`, a map or a set"));
                }
                let pairs = Expr::call(Expr::helper("$unzip"), vec![items]);
                if sides.iter().all(|&side| self.is_vec_like(side)) {
                    return Ok(pairs);
                }
                let pairs = self.spill("unzipped", pairs, out);
                let mut parts = Vec::new();
                for (i, &side) in sides.iter().enumerate() {
                    let part = Expr::index(pairs.clone(), Expr::int(i as i128));
                    parts.push(match side.kind() {
                        ty::Adt(_, args) if self.is_map(side) => {
                            let class =
                                self.map_class(self.is_set(side), self.is_sorted(side), args.types().next(), span)?;
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

/// `v.insert(at, value)`: `$insertAt`, which refuses an index past the end;
/// at the front, which no length refuses, `v.unshift(value)`.
pub(super) fn inserted(subject: Expr, at: Expr, value: Expr) -> Expr {
    match at.as_int() {
        Some(0) => Expr::call(Expr::member(subject, "unshift"), vec![value]),
        _ => Expr::call(Expr::helper("$insertAt"), vec![subject, at, value]),
    }
}

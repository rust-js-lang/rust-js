//! Iterators from MIR (ADR 0364): each a JS iterator, whose helpers are
//! lazy as Rust's adapters are, so what each closure does happens in the
//! order Rust's does, item by item. A range is its `{ start, end }` until
//! it's iterated, and an iterator of the crate's its impl's `next`, made a
//! JS iterator where one's wanted. What's taken by a `&mut`, `any` of
//! `&mut it`, steps it through `$lent`, which a helper that stops early
//! closes in its place, as Rust leaves the iterator to go on.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Mutability, Ty};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::iterators::made_array;
use crate::lower::recognition::{StdItem, is_std_def, std_item};
use crate::lower::representation::Num;
use crate::lower::std_types::heap::HeapOp;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R, bindings};
use crate::runtime::Helper;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of an `Iterator`'s or an `IntoIterator`'s method, of a std
    /// iterator, from MIR: `None` if it's another.
    pub(in crate::lower) fn mir_iter_call(
        &mut self,
        def_id: DefId,
        arg_tys: &[Ty<'tcx>],
        values: Vec<Expr>,
        output: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let Some(trait_id) = tcx.trait_of_assoc(def_id) else {
            return Ok(None);
        };
        let name = tcx.item_name(def_id);
        let name = name.as_str();
        let receiver_ty = arg_tys.first().copied().unwrap_or(tcx.types.unit);
        let iter_ty = receiver_ty.peel_refs();
        // One taken by value, `items.fold(..)`'s, dropped as it's done, which
        // JS doesn't: of a generic iterator, one its callers give none with a
        // destructor (ADR 0190); of the crate's own with one, refused, THIR's.
        if !receiver_ty.is_ref()
            && is_std_def(tcx, trait_id, StdItem::Iterator)
            && self.drops(receiver_ty) != crate::lower::drops::Drops::Nothing
        {
            if self.drop_query().drops_but_params(receiver_ty) == crate::lower::drops::Drops::Nothing {
                self.require_no_drops(receiver_ty);
            } else if self.is_user_iterator(receiver_ty) {
                return Err(self.unsupported(
                    span,
                    &format!("`{name}` of an iterator with a destructor, from its MIR"),
                ));
            }
        }
        // `x.into_iter()`: a range stays one, an iterator is itself, and a
        // collection's items are a JS iterator of them.
        if is_std_def(tcx, trait_id, StdItem::IntoIterator) && name == "into_iter" {
            let value = values.into_iter().next().expect("the receiver");
            // A generic one is an array or a JS iterator, as its caller has
            // it: `Iterator.from` takes either (ADR 0061).
            if matches!(receiver_ty.kind(), ty::Param(_)) {
                return Ok(Some(self.js_iterator(value)));
            }
            if self.range_kind(iter_ty).is_some() || self.implements_iterator(receiver_ty) {
                return Ok(Some(value));
            }
            // `for x in &mut v` of numbers: handles on its items, as
            // `iter_mut()`'s are, `mir_std_call`'s (ADR 0099).
            let handles = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut))
                && self.iterator_item(output).is_some_and(|item| self.is_cell(item));
            if handles {
                return Ok(None);
            }
            if iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty) {
                return Ok(Some(self.std_iterator(value, output)));
            }
            return Ok(None);
        }
        // `String::from_iter(items)`, `items.collect()` into a `String`: an
        // array's joined, as THIR's, or a JS iterator's items.
        if self.recognition().is_from_iterator(trait_id)
            && name == "from_iter"
            && self.is_lang_adt(output, LangItem::String)
        {
            let items = values.into_iter().next().expect("the items");
            let given = receiver_ty.peel_refs();
            let items = match given.is_array() || given.is_slice() || self.is_vec_like(given) {
                true => items,
                false => Expr::call(Expr::member(self.js_iterator(items), "toArray"), Vec::new()),
            };
            return Ok(Some(Expr::call(Expr::member(items, "join"), vec![Expr::str("")])));
        }
        // `next_back(&mut it)` and `len(&it)`: of what knows its back, a
        // range or a `$iter` (ADR 0071); not a lazy or a generic one.
        let recognition = self.recognition();
        let (double_ended, exact) = (
            recognition.double_ended_iterator() == Some(trait_id),
            recognition.exact_size_iterator() == Some(trait_id),
        );
        if (double_ended && name == "next_back") || (exact && name == "len") {
            // The crate's own `len()`, std's: its `size_hint()`, exact, as THIR's.
            let own_len = || -> R<bool> {
                let args = self.args_of(trait_id, iter_ty);
                Ok(self
                    .resolve_instance(def_id, args)?
                    .is_some_and(|i| self.is_rust_fn(i.def_id())))
            };
            if self.is_user_iterator(iter_ty) && name == "len" && !own_len()? {
                let it = values.into_iter().next().expect("the receiver");
                let mut made = Vec::new();
                let len = self.exact_len_of(it, iter_ty, span, &mut made)?;
                if !made.is_empty() {
                    return Err(self.unsupported(span, "this `len()`, from its MIR"));
                }
                return Ok(Some(len));
            }
            if self.is_user_iterator(iter_ty) {
                return Ok(None);
            }
            let it = values.into_iter().next().expect("the receiver");
            let boxed = self.option_of(output).is_some_and(|item| self.boxed_payload(item));
            // A string's `chars()`, a `$iter` of its array (`std_iterator`).
            let array = self.array_source(iter_ty).is_some()
                || self.recognition().is_str_chars(iter_ty)
                || self.recognition().is_str_bytes(iter_ty)
                || self.recognition().is_slice_iter_mut(iter_ty)
                || self.recognition().is_tree_iter(iter_ty)
                || self.is_str_split(iter_ty)
                // A `Peekable` of an array's, the same `$iter`.
                || (self.recognition().is_peekable(iter_ty)
                    && matches!(iter_ty.kind(), ty::Adt(_, args)
                        if args.types().next().is_some_and(|inner| self.array_source(inner).is_some())));
            return Ok(Some(match (name, self.range_kind(iter_ty)) {
                ("next_back", Some(RangeKind::Exclusive)) => {
                    self.helper(Helper::RangeNextBack, "$rangeNextBack", vec![it])
                }
                ("next_back", Some(RangeKind::Inclusive)) => {
                    self.helper(Helper::RangeInclusiveNextBack, "$rangeInclusiveNextBack", vec![it])
                }
                // A map of an array's items, `v.iter().map(f)`: its source's
                // last item, mapped, as Rust's `Map::next_back` maps that one
                // alone.
                ("next_back", None)
                    if !boxed
                        && crate::lower::iterators::is_items_map(&it)
                        && let js::ExprKind::Call(callee, args) = &it.kind
                        && let js::ExprKind::Member(source, _) = &callee.kind =>
                {
                    let last = self.helper(Helper::NextBack, "$nextBack", vec![(**source).clone()]);
                    let last = self.spill("last", last, made);
                    let none = Expr::bin(Op::Eq, last.clone(), Expr::undefined());
                    Expr::cond(
                        none,
                        Expr::undefined(),
                        crate::lower::calls::apply(args[0].clone(), vec![last]),
                    )
                }
                ("next_back", None) if array && boxed => self.helper(Helper::NextBackSome, "$nextBackSome", vec![it]),
                ("next_back", None) if array => self.helper(Helper::NextBack, "$nextBack", vec![it]),
                ("len", None) if array => Expr::bin(Op::Sub, Expr::member(it.clone(), "end"), Expr::member(it, "at")),
                // A chain made here of an array's items, which nothing else
                // steps, skipped or taken from, no closure of its to run: its
                // items' count, `$iter(xs).drop(1).toArray().length`, which is
                // `xs.slice(1).length`, as THIR's.
                ("len", None) if counted_chain(&it) => {
                    Expr::member(Expr::call(Expr::member(it, "toArray"), Vec::new()), "length")
                }
                // A range of numbers' count, none where it ends before it starts.
                ("len", Some(RangeKind::Exclusive))
                    if let Some((start, end)) = range_bounds(&it)
                        && self
                            .range_index(iter_ty)
                            .and_then(Num::of)
                            .is_some_and(|num| !num.big()) =>
                {
                    let count = Expr::bin(Op::Sub, end, start);
                    Expr::call(Expr::member(Expr::var("Math"), "max"), vec![count, Expr::int(0)])
                }
                _ => {
                    return Err(self.unsupported(
                        span,
                        &format!("`{name}()` of a lazy or generic `{iter_ty}`, from its MIR"),
                    ));
                }
            }));
        }
        // `rev()` and `peekable()`: of the crate's iterator, its own
        // `next_back`; of a range, stepped from its end; of items made
        // already, an array's, a `$iter` of them; not of a lazy one, whose
        // closures would run in another order (ADR 0164).
        if is_std_def(tcx, trait_id, StdItem::Iterator) && matches!(name, "rev" | "peekable") {
            let receiver = values.into_iter().next().expect("the receiver");
            if self.is_user_iterator(iter_ty) {
                return match self.own_next_back(iter_ty)? {
                    Some(double_ended) if name == "rev" => self
                        .user_iterator(receiver, iter_ty, double_ended, "next_back", span)
                        .map(Some),
                    _ => Err(self.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR"))),
                };
            }
            let range = self.range_kind(iter_ty);
            return Ok(Some(match (name, range) {
                ("rev", Some(RangeKind::Exclusive)) => {
                    self.runtime.insert(Helper::RangeNextBack);
                    self.helper(
                        Helper::Iterator,
                        "$iterator",
                        vec![receiver, Expr::var("$rangeNextBack")],
                    )
                }
                ("rev", Some(RangeKind::Inclusive)) => {
                    self.runtime.insert(Helper::RangeInclusiveNextBack);
                    let next_back = Expr::var("$rangeInclusiveNextBack");
                    self.helper(Helper::Iterator, "$iterator", vec![receiver, next_back])
                }
                (_, Some(_)) => {
                    return Err(self.unsupported(span, &format!("`{name}` of a `{iter_ty}`, from its MIR")));
                }
                _ => {
                    let items = match self.array_source(iter_ty) {
                        Some(_) => match self.std_iterator(receiver, iter_ty) {
                            it if name == "peekable" => return Ok(Some(it)),
                            // Of one just made, the array, which `toReversed`
                            // copies anyway.
                            it if name == "rev" && crate::lower::iterators::is_stepped_items(&it) => {
                                crate::lower::iterators::iterated(it)
                            }
                            it => self.helper(Helper::Rest, "$rest", vec![it]),
                        },
                        // A `$iter` of an array THIR's lowering made, just made: the
                        // array.
                        None if crate::lower::iterators::is_stepped_items(&receiver) => match name {
                            "peekable" => return Ok(Some(receiver)),
                            _ => crate::lower::iterators::iterated(receiver),
                        },
                        // Of such an array's skipped and taken from, the array
                        // sliced, as THIR's: no closure runs over it.
                        None if name == "rev" && counted_chain(&receiver) => sliced(&receiver),
                        None => match from_iterator(receiver) {
                            Ok(items) => Expr::call(Expr::member(Expr::var("Array"), "from"), vec![items]),
                            Err(_) => {
                                return Err(
                                    self.unsupported(span, &format!("`{name}` of a lazy iterator, from its MIR"))
                                );
                            }
                        },
                    };
                    let items = match name {
                        "rev" => Expr::call(Expr::member(items, "toReversed"), Vec::new()),
                        _ => items,
                    };
                    self.stepped_items(items)
                }
            }));
        }
        // `size_hint()` of a `$iter`, exact, or of a generic one, as `$sizeHint`
        // finds it, as THIR's (ADR 0170).
        if is_std_def(tcx, trait_id, StdItem::Iterator) && name == "size_hint" && !self.is_user_iterator(iter_ty) {
            let mut it = values.into_iter().next().expect("the receiver");
            // A `map`'s is its source's, as Rust's `Map` passes it on; its
            // closure, unrun, is made for nothing.
            let mut mapped = false;
            while let js::ExprKind::Call(callee, args) = &it.kind
                && let js::ExprKind::Member(source, method) = &callee.kind
                && method == "map"
                && args.iter().all(|f| !f.has_effects())
            {
                it = (**source).clone();
                mapped = true;
            }
            let known = match mapped {
                true => crate::lower::iterators::is_stepped_items(&it),
                false => self.array_source(iter_ty).is_some() || matches!(iter_ty.kind(), ty::Param(_)),
            };
            if !known {
                return Err(self.unsupported(span, &format!("`size_hint()` of a `{iter_ty}`, from its MIR")));
            }
            return Ok(Some(self.helper(Helper::SizeHint, "$sizeHint", vec![it])));
        }
        // `by_ref()` of the crate's iterator: a `&mut` to it, it itself, which
        // its own `next` steps.
        if name == "by_ref" && self.is_user_iterator(iter_ty) {
            return Ok(values.into_iter().next());
        }
        // `next` of a `&mut` to the crate's iterator, `by_ref()`'s: std's of a
        // `&mut I`, which is `I`'s own, of what the `&mut` is to.
        if name == "next"
            && self.is_user_iterator(iter_ty)
            && let ty::Ref(_, through, _) = receiver_ty.kind()
            && through.is_ref()
        {
            let next = def_id;
            let args = self.args_of(trait_id, iter_ty);
            let mut made = Vec::new();
            let value = values.into_iter().next().expect("the receiver");
            let call = self.trait_call(next, args, vec![value], span, &mut made)?;
            return match call {
                Some(call) if made.is_empty() => Ok(Some(call)),
                _ => Err(self.unsupported(span, "`next` of this `&mut`, from its MIR")),
            };
        }
        // Of the crate's iterator, its own `next` and what its impl writes
        // itself are its trait's; the rest, `map`, `fuse`, `collect`, std's
        // of a JS iterator its `next` steps (`as_js_iterator`), as THIR's.
        let own = self.is_user_iterator(iter_ty)
            && (name == "next" || self.recognition().user_impl_defines(trait_id, iter_ty, name));
        if !is_std_def(tcx, trait_id, StdItem::Iterator) || own {
            return Ok(None);
        }
        let mut values = values.into_iter();
        let receiver = values.next().expect("the receiver");
        // `next(&mut it)`: a range's start moved, or the JS iterator stepped.
        if name == "next" {
            let item = self.option_of(output).expect("`next` gives an `Option`");
            return Ok(Some(match self.range_kind(iter_ty) {
                Some(RangeKind::Exclusive) => self.helper(Helper::RangeNext, "$rangeNext", vec![receiver]),
                Some(RangeKind::From) => self.helper(Helper::RangeFromNext, "$rangeFromNext", vec![receiver]),
                Some(RangeKind::Inclusive) => {
                    self.helper(Helper::RangeInclusiveNext, "$rangeInclusiveNext", vec![receiver])
                }
                Some(_) => return Ok(None),
                None if self.boxed_payload(item) => self.helper(Helper::NextSome, "$nextSome", vec![receiver]),
                None => self.helper(Helper::Next, "$next", vec![receiver]),
            }));
        }
        // Taken by a `&mut`, it's stepped through what can't close it.
        let lent = matches!(receiver_ty.kind(), ty::Ref(_, _, Mutability::Mut));
        let it = self.as_js_iterator(receiver, iter_ty, span)?;
        let it = match lent {
            true => self.lent_iterator(it),
            false => it,
        };
        let item = self.iterator_item(iter_ty);
        // An item with a destructor a method may leave undropped, `filter`'s
        // or `count`'s: what std drops, refused till it's dropped here.
        let keeps_each = matches!(
            name,
            "map"
                | "for_each"
                | "fold"
                | "collect"
                | "any"
                | "all"
                | "position"
                | "enumerate"
                | "by_ref"
                | "fuse"
                | "sum"
                | "product"
                | "count"
        );
        let dropping = item.filter(|&item| self.drops(item) != crate::lower::drops::Drops::Nothing);
        // `filter` and `skip_while` drop each they discard, below and THIR's.
        if !keeps_each && !matches!(name, "filter" | "skip_while") && dropping.is_some() {
            return Err(self.unsupported(span, &format!("`{name}` of items with a destructor, from its MIR")));
        }
        let mut arg = || values.next().expect("rustc checked the arguments");
        // A function of the crate given as the callback, `filter(is_rule)`:
        // itself, as THIR's, which takes no more than it's given (ADR 0279).
        let own = arg_tys.get(1).is_some_and(|ty| self.is_crate_fn_item(*ty));
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        Ok(Some(match name {
            "map" => method("map", it, vec![self.unary_callback(arg(), own)]),
            // Of items with a destructor: each it doesn't keep dropped as it
            // discards it, THIR's (ADR 0098).
            "filter" => {
                let test = self.unary_callback(arg(), own);
                let test = match dropping {
                    Some(item) => self.dropping_discarded(test, item, false, span, made)?,
                    None => test,
                };
                method("filter", it, vec![test])
            }
            "flat_map" => method("flatMap", it, vec![self.unary_callback(arg(), own)]),
            "take" => method("take", it, vec![arg()]),
            "skip" => method("drop", it, vec![arg()]),
            "for_each" => method("forEach", it, vec![self.unary_callback(arg(), own)]),
            "any" => method("some", it, vec![self.unary_callback(arg(), own)]),
            "all" => method("every", it, vec![self.unary_callback(arg(), own)]),
            // Of what looks like `None`: the first kept, boxed, THIR's.
            "find" => {
                let test = self.unary_callback(arg(), own);
                match item.is_some_and(|item| self.boxed_payload(item)) {
                    true => self.helper(Helper::NextSome, "$nextSome", vec![method("filter", it, vec![test])]),
                    false => method("find", it, vec![test]),
                }
            }
            "position" => {
                let found = self.unary_callback(arg(), own);
                self.helper(Helper::Position, "$position", vec![it, found])
            }
            // Each item of both in turn, till one differs or either ends, as
            // Rust's `eq` steps them; `ne` its negation.
            "eq" | "ne" => {
                let Some(item) = item else { return Ok(None) };
                let other = arg();
                let mut list = vec![it, other];
                list.extend(self.item_eq(item, span)?);
                let same = self.helper(Helper::CmpItems, "$iterEq", list);
                match name {
                    "eq" => same,
                    _ => Expr::unary(js::UnaryOp::Not, same),
                }
            }
            // THIR's pairing, `(x, i) => [i, x]`, in place of which a
            // callback that takes the pair apart is given each item and its
            // index, of an array (`eager_chains`).
            "enumerate" => {
                let pair = Expr::array(vec![Expr::var("i"), Expr::var("x")]);
                let f = Expr::arrow(
                    vec!["x".into(), "i".into()],
                    vec![StmtKind::Return(Some(pair)).at(js::Span::NONE)],
                );
                method("map", it, vec![f])
            }
            "fold" => {
                let init = arg();
                // Given after what it starts from.
                let own = arg_tys.get(2).is_some_and(|ty| self.is_crate_fn_item(*ty));
                let f = self.binary_callback(arg(), own);
                method("reduce", it, vec![f, init])
            }
            // Each item counted, and dropped as `count` drops it.
            "count" => {
                let n = self.fresh("n");
                let x = self.fresh("item");
                let mut body = Vec::new();
                if let Some(item) = item {
                    self.drop_value(Expr::var(&x), item, span, &mut body)?;
                }
                let mut params = vec![n.clone().into()];
                if !body.is_empty() {
                    params.push(x.into());
                }
                body.push(StmtKind::Return(Some(Expr::bin(Op::Add, Expr::var(&n), Expr::int(1)))).at(js::Span::NONE));
                method("reduce", it, vec![Expr::arrow(params, body), Expr::int(0)])
            }
            "sum" | "product" => {
                let num = Num::of(output)
                    .ok_or_else(|| self.unsupported(span, &format!("`{name}` of a `{output}`, from its MIR")))?;
                let (op, start) = match name {
                    "sum" => (Op::Add, 0),
                    _ => (Op::Mul, 1),
                };
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let total = num.wrap(Expr::bin(op, Expr::var(&a), Expr::var(&b)));
                let f = Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(total)).at(js::Span::NONE)],
                );
                // Rust's float `Sum` starts at -0.0, an empty sum's and one of
                // negative zeros' sign kept, as THIR's.
                let start = match (name, num.float()) {
                    ("sum", true) => Expr::num(-0.0),
                    _ => num.literal(start),
                };
                method("reduce", it, vec![f, start])
            }
            "copied" => it,
            "cloned" => match item {
                Some(item) if self.is_copy(item.peel_refs()) => it,
                Some(item) => {
                    let x = self.fresh("item");
                    let mut body = Vec::new();
                    let copy = self.clone_value(Expr::var(&x), item.peel_refs(), span, &mut body)?;
                    body.push(StmtKind::Return(Some(copy)).at(js::Span::NONE));
                    method("map", it, vec![Expr::arrow(vec![x.into()], body)])
                }
                None => return Ok(None),
            },
            // Of all the items, in order, as THIR's of an array.
            "max" | "min" => {
                let items = method("toArray", it, Vec::new());
                self.extreme_of(name == "max", items, item, span)?
            }
            "last" => {
                let items = method("toArray", it, Vec::new());
                self.last_of(items, item, made)
            }
            "by_ref" => it,
            "fuse" => it,
            "collect" => {
                // Into a heap: its items in heap order, `$heapFrom`, THIR's.
                if self.is_std_type(output, StdItem::BinaryHeap) {
                    let items = method("toArray", it, Vec::new());
                    self.heap_values(HeapOp::From, vec![items], output, span)?
                } else if self.is_vec_like(output) {
                    match self.mapped_range(&it, iter_ty) {
                        Some(items) => items,
                        None => method("toArray", it, Vec::new()),
                    }
                } else if self.is_lang_adt(output, LangItem::String) {
                    method("join", method("toArray", it, Vec::new()), vec![Expr::str("")])
                } else if self.option_of(output).is_some() || self.recognition().is_result(output) {
                    self.collect_fallible(it, output, item, span)?
                } else if self.is_map(output) {
                    // A map or a set of what its key is, as THIR's `From` of
                    // items makes one (ADR 0121).
                    let key = match output.kind() {
                        ty::Adt(_, args) => args.types().next(),
                        _ => None,
                    };
                    let class = self.map_class(self.is_set(output), key);
                    Expr::new_(class, vec![it])
                } else {
                    return Err(self.unsupported(span, &format!("`collect` into a `{output}`, from its MIR")));
                }
            }
            _ => return Ok(None),
        }))
    }

    /// `value`, an iterator of type `ty`, as a JS iterator: a range stepped
    /// by its `next`, the crate's by its impl's, a JS one as it is.
    pub(super) fn as_js_iterator(&mut self, value: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        // The crate's own behind an `impl Iterator` too.
        let ty = self.reveal(ty);
        match self.range_kind(ty) {
            Some(RangeKind::Exclusive) => {
                self.runtime.insert(Helper::RangeNext);
                Ok(self.helper(Helper::Iterator, "$iterator", vec![value, Expr::var("$rangeNext")]))
            }
            Some(RangeKind::From) => {
                self.runtime.insert(Helper::RangeFromNext);
                Ok(self.helper(Helper::Iterator, "$iterator", vec![value, Expr::var("$rangeFromNext")]))
            }
            Some(RangeKind::Inclusive) => {
                self.runtime.insert(Helper::RangeInclusiveNext);
                Ok(self.helper(
                    Helper::Iterator,
                    "$iterator",
                    vec![value, Expr::var("$rangeInclusiveNext")],
                ))
            }
            Some(_) => Err(self.unsupported(span, "iterating this range, from its MIR")),
            None if self.is_user_iterator(ty) => {
                let trait_id = std_item(self.tcx, StdItem::Iterator);
                self.user_iterator(value, ty, trait_id, "next", span)
            }
            // A generic one, which may be given an array (ADR 0061): JS's
            // iterator of it, as THIR's.
            None if matches!(ty.kind(), ty::Param(_)) && !is_js_iterator(&value) => Ok(self.js_iterator(value)),
            None => Ok(value),
        }
    }

    /// A std iterator of type `ty` made of `value`, the array or the text
    /// THIR's lowering makes of it, as MIR steps it: over an array, a
    /// `$iter`, which knows where it is, so a clone of it does (ADR 0181);
    /// another, the JS iterator of `value`.
    pub(super) fn std_iterator(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
        match self.array_source(ty) {
            Some(_) => match from_iterator(value) {
                Ok(items) | Err(items) if !is_js_iterator(&items) => self.stepped_items(items),
                Ok(items) | Err(items) => items,
            },
            None if is_js_iterator(&value) => value,
            // An array THIR's lowering makes of it, `s.split(",")`'s, or a
            // `VecDeque`'s or a slice's `iter_mut()`'s, an array: a `$iter`
            // of it, as of an array's.
            None if made_array(&value)
                || self.recognition().is_deque_iter(ty)
                || self.recognition().is_slice_iter_mut(ty)
                || self.recognition().is_tree_range(ty) =>
            {
                self.stepped_items(value)
            }
            None => self.js_iterator(value),
        }
    }

    /// `$iterator({ start: a, end: b }, $rangeNext).map(f)`, a range of
    /// numbers `iter_ty` steps, each item mapped, all collected: its array's,
    /// `$range(a, b).map(f)`, as THIR's, made as big as what's collected
    /// anyway; what more than one map does in Rust's order, one item through
    /// them all at a time, only lazily. Not filtered, which may keep few of
    /// many, `$lazyRange(a, b).filter(f)` (`range_items`). Of `a..=b`,
    /// `$range(a, b + 1)`; stepped from its end, `.rev()`, `.toReversed()`
    /// first.
    fn mapped_range(&mut self, it: &Expr, iter_ty: Ty<'tcx>) -> Option<Expr> {
        let mut maps = Vec::new();
        let mut at = it;
        while let js::ExprKind::Call(callee, args) = &at.kind
            && let js::ExprKind::Member(source, name) = &callee.kind
            && name == "map"
            && let [f] = &args[..]
        {
            maps.push((name.clone(), f.clone()));
            at = source;
        }
        let js::ExprKind::Call(callee, args) = &at.kind else {
            return None;
        };
        let (js::ExprKind::Var(iterator), [range, next]) = (&callee.kind, &args[..]) else {
            return None;
        };
        let (js::ExprKind::Object(bounds), js::ExprKind::Var(next)) = (&range.kind, &next.kind) else {
            return None;
        };
        let [js::Prop::Field(s, start), js::Prop::Field(e, end)] = &bounds[..] else {
            return None;
        };
        let numbers = (iter_ty.walk())
            .filter_map(|arg| arg.as_type())
            .find(|&t| matches!(self.range_kind(t), Some(RangeKind::Exclusive | RangeKind::Inclusive)))
            .and_then(|t| self.range_index(t))
            .and_then(Num::of)
            .is_some_and(|num| !num.big());
        let (inclusive, back) = match next.as_str() {
            "$rangeNext" => (false, false),
            "$rangeNextBack" => (false, true),
            "$rangeInclusiveNext" => (true, false),
            "$rangeInclusiveNextBack" => (true, true),
            _ => return None,
        };
        if iterator != "$iterator"
            || (s.as_str(), e.as_str()) != ("start", "end")
            || !numbers
            || maps.iter().filter(|(_, f)| crate::lower::iterators::does(f)).count() > 1 && maps.len() > 1
        {
            return None;
        }
        let end = match inclusive {
            true => match end.as_int() {
                Some(n) => Expr::int(n + 1),
                None => Expr::bin(js::Op::Add, end.clone(), Expr::int(1)),
            },
            false => end.clone(),
        };
        let mut items = self.helper(Helper::Range, "$range", vec![start.clone(), end]);
        if back {
            items = Expr::call(Expr::member(items, "toReversed"), Vec::new());
        }
        Some(maps.into_iter().rev().fold(items, |items, (name, f)| {
            Expr::call(Expr::member(items, &name), vec![f])
        }))
    }

    /// Whether `ty` is an iterator: one whose `Item` is a type.
    pub(in crate::lower) fn implements_iterator(&self, ty: Ty<'tcx>) -> bool {
        self.iterator_item(ty)
            .is_some_and(|item| !matches!(item.kind(), ty::Alias(..)))
            || matches!(ty.kind(), ty::Param(_))
    }

    /// A function JS's helpers give an item, and its index too, as one
    /// that takes the item alone: an arrow of one parameter as it is,
    /// anything else called with the item alone.
    fn unary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 1 => f,
            js::ExprKind::Var(_) if own => f,
            _ => {
                let x = self.fresh("item");
                let call = Expr::call(f, vec![Expr::var(&x)]);
                Expr::arrow(vec![x.into()], vec![StmtKind::Return(Some(call)).at(js::Span::NONE)])
            }
        }
    }

    /// Is `ty` a function item of the crate's own, not a JS binding's or a
    /// trait's method, whose JS is called with what Rust gives it alone?
    fn is_crate_fn_item(&self, ty: Ty<'tcx>) -> bool {
        match *ty.kind() {
            ty::FnDef(def_id, _) => {
                def_id.is_local()
                    && self.tcx.trait_of_assoc(def_id).is_none()
                    && !bindings::is_binding(self.tcx, def_id)
            }
            _ => false,
        }
    }

    /// As `unary_callback`, of a function of two: `reduce`'s, given an index
    /// too. `f64::max`'s and `f64::min`'s helpers take two alone, as THIR
    /// gives them.
    fn binary_callback(&mut self, f: Expr, own: bool) -> Expr {
        match &f.kind {
            js::ExprKind::Arrow(params, _) if params.len() <= 2 => f,
            js::ExprKind::Var(_) if own => f,
            js::ExprKind::Var(name) if name == "$f64Max" || name == "$f64Min" => f,
            _ => {
                let (a, b) = (self.fresh("a"), self.fresh("b"));
                let call = Expr::call(f, vec![Expr::var(&a), Expr::var(&b)]);
                Expr::arrow(
                    vec![a.into(), b.into()],
                    vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
        }
    }

    pub(super) fn helper(&mut self, helper: Helper, name: &str, args: Vec<Expr>) -> Expr {
        self.runtime.insert(helper);
        Expr::call(Expr::var(name), args)
    }
}

/// Whether `e` is a JS iterator as it's made: `Iterator.from(..)`, a
/// `$iter` or a range's `$iterator`, a collection's `values()`, `keys()`
/// or `entries()`, or an iterator helper's of one, `.map(f)` of a range's.
/// Whether `e` is an array's items made there, `$iter(xs)`, skipped or taken
/// from, `.drop(1)`, by a count that does nothing (`len()`'s).
/// `counted_chain`'s `$iter(xs).drop(1).take(2)`: the array it steps,
/// `xs.slice(1).slice(0, 2)`.
fn sliced(e: &Expr) -> Expr {
    match &e.kind {
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(source, name) = &callee.kind
                && let [n] = &args[..] =>
        {
            let items = sliced(source);
            match name.as_str() {
                "drop" => Expr::call(Expr::member(items, "slice"), vec![n.clone()]),
                _ => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), n.clone()]),
            }
        }
        _ => crate::lower::iterators::iterated(e.clone()),
    }
}

fn counted_chain(e: &Expr) -> bool {
    let mut at = e;
    let mut stages = 0;
    while let js::ExprKind::Call(callee, args) = &at.kind
        && let js::ExprKind::Member(source, name) = &callee.kind
        && matches!(name.as_str(), "drop" | "take")
        && let [n] = &args[..]
        && !n.has_effects()
    {
        stages += 1;
        at = source;
    }
    stages > 0 && crate::lower::iterators::is_stepped_items(at)
}

/// A range's bounds, each read once: of `{ start: a, end: b }` made there,
/// `a` and `b`, or of one that reads the same, `r.start` and `r.end`.
fn range_bounds(range: &Expr) -> Option<(Expr, Expr)> {
    match &range.kind {
        js::ExprKind::Object(props) => match &props[..] {
            [js::Prop::Field(s, start), js::Prop::Field(e, end)] if s == "start" && e == "end" => {
                Some((start.clone(), end.clone()))
            }
            _ => None,
        },
        _ if range.reads_same() => Some((Expr::member(range.clone(), "start"), Expr::member(range.clone(), "end"))),
        _ => None,
    }
}

fn is_js_iterator(e: &Expr) -> bool {
    let js::ExprKind::Call(callee, _) = &e.kind else {
        return false;
    };
    match &callee.kind {
        js::ExprKind::Var(v) => v == "$iterator" || crate::lower::iterators::is_stepped_items(e),
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
fn from_iterator(e: Expr) -> Result<Expr, Expr> {
    match e.kind {
        js::ExprKind::Call(callee, mut args)
            if args.len() == 1
                && matches!(&callee.kind, js::ExprKind::Member(object, name)
                    if name == "from" && matches!(&object.kind, js::ExprKind::Var(v) if v == "Iterator")) =>
        {
            Ok(args.pop().expect("one"))
        }
        kind => Err(Expr {
            reads: false,
            kind,
            span: e.span,
        }),
    }
}

//! What ends a chain: its extremes, its last, its sort, and a call of its
//! source's that runs Rust.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `it.filter(test)`; of items with a destructor, `dropping`, each it
    /// doesn't keep dropped by the runtime's `filter`, which drops it past a
    /// fold closure's result, as `$kept` says, never JS's own, which drops
    /// it inside the test, where no consumer can tell its panic from the
    /// test's: `$filterDropping(it, test, drop)` (#651).
    pub(in crate::lower) fn filtered(
        &mut self,
        it: Expr,
        test: Expr,
        dropping: Option<ty::Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let drop = match dropping {
            Some(item) => self.std_drop_function(item, span)?,
            None => None,
        };
        Ok(match drop {
            Some(drop) => Expr::call_helper("$filterDropping", vec![it, test, drop]),
            None => Expr::call(Expr::member(it, "filter"), vec![test]),
        })
    }

    /// What a `skip_while` or a `take_while` of `item`s with a destructor
    /// is given, `exprs`, after its test: the drop of each item it discards,
    /// each it skips or the one it stops at, which `$lazySkipWhile` and
    /// `$lazyTakeWhile` run as `$kept` says, the runtime's one answer of what
    /// a discarding stage drops as its test panics (#651, #817).
    pub(in crate::lower) fn dropping_discarded(&mut self, item: ty::Ty<'tcx>, exprs: &mut Vec<Expr>, span: Span) -> R<()> {
        exprs.extend(self.std_drop_function(item, span)?);
        Ok(())
    }

    /// `max()` or `min()` of `items`, an iterator of `item`s, or an array:
    /// of
    /// what JS's `<` doesn't order, by its `cmp` (ADR 0057), each passed over
    /// dropped as it's passed, of items with a destructor (ADR 0098).
    pub(in crate::lower) fn extreme_of(
        &mut self,
        max: bool,
        items: Expr,
        item: Option<ty::Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        Ok(match item {
            Some(item) if !self.is_primitive_ord(item) => {
                let compare = self.cmp_fn(item, false, span)?;
                let helper = if max { "$maxBy" } else { "$minBy" };
                self.extreme_by(helper, vec![items, compare], item, (None, None), span)?
            }
            _ => Expr::call(Expr::helper(if max { "$max" } else { "$min" }), vec![items]),
        })
    }

    /// `max_by(f)` or `min_by(f)` of `items`, of `item`s, `max`'s or not;
    /// or `max_by_key(f)` or `min_by_key(f)`, of a `key`, made once for
    /// each, as std's, compared by its `cmp`: each passed over dropped as
    /// it's passed, of items with a destructor (ADR 0098); `ends`, the drop
    /// of `f` where its fold drops it before what's left of `items` (#734).
    /// The one form of each, the recognizers' and MIR's.
    pub(in crate::lower) fn extreme_by_call(
        &mut self,
        (max, key): (bool, Option<ty::Ty<'tcx>>),
        (items, f): (Expr, Expr),
        item: ty::Ty<'tcx>,
        ends: Option<Expr>,
        span: Span,
    ) -> R<Expr> {
        let Some(key) = key else {
            let helper = if max { "$maxBy" } else { "$minBy" };
            return self.extreme_by(helper, vec![items, f], item, (ends, None), span);
        };
        // A key with a destructor, given its drop: each pair passed over
        // drops its key, then its item, and the kept one's key drops before
        // its item is given back, as std's `(key, item)` pairs do (#776).
        let key_drop = self.std_drop_function(key, span)?;
        let compare = self.cmp_fn(key, false, span)?;
        let helper = if max { "$maxByKey" } else { "$minByKey" };
        self.extreme_by(helper, vec![items, f, compare], item, (ends, key_drop), span)
    }

    /// `helper`, `$maxBy` or one of its kind, given `args`, then whether an
    /// `item` is boxed where it may look like `None` (ADR 0051), what
    /// drops each passed over, of one with a destructor (ADR 0098), and
    /// `ends`, what drops with its function (#734), and a key's drop, of
    /// one with a destructor (#776).
    fn extreme_by(
        &mut self,
        helper: &'static str,
        mut args: Vec<Expr>,
        item: ty::Ty<'tcx>,
        (ends, key_drop): (Option<Expr>, Option<Expr>),
        span: Span,
    ) -> R<Expr> {
        let boxed = self.boxed_payload(item);
        let drop = self.std_drop_function(item, span)?;
        if boxed || drop.is_some() || ends.is_some() || key_drop.is_some() {
            args.push(Expr::bool(boxed));
        }
        match (drop, ends, key_drop) {
            (drop, ends, Some(key_drop)) => args.extend([
                drop.unwrap_or_else(Expr::undefined),
                ends.unwrap_or_else(Expr::undefined),
                key_drop,
            ]),
            (drop, Some(ends), None) => args.extend([drop.unwrap_or_else(Expr::undefined), ends]),
            (drop, None, None) => args.extend(drop),
        }
        Ok(Expr::call(Expr::helper(helper), args))
    }

    /// `last()` of `items`, an array of `item`s: one that looks like `None`
    /// boxed (ADR 0051).
    pub(in crate::lower) fn last_of(&mut self, items: Expr, item: Option<ty::Ty<'tcx>>, out: &mut Vec<Stmt>) -> Expr {
        match item {
            Some(item) if self.boxed_payload(item) => {
                let items = if items.reads_same() {
                    items
                } else {
                    self.spill("items", items, out)
                };
                let last = Expr::bin(Op::Sub, Expr::member(items.clone(), "length"), Expr::int(1));
                self.some_at(items, last)
            }
            _ => Expr::call(Expr::member(items, "at"), vec![Expr::int(-1)]),
        }
    }

    /// `v.sort()`, or `v.sort_by_key(key)`: in place (ADR 0036), of the items'
    /// value. JS's `sort()` compares as strings: right for `bool`s, and
    /// numbers need `a - b`, strings `$cmp`, by code point (ADR 0183).
    pub(in crate::lower) fn sort_values(
        &mut self,
        key: Option<Expr>,
        items: Expr,
        receiver_ty: ty::Ty<'tcx>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let method = |items: Expr, name: &str, list: Vec<Expr>| Expr::call(Expr::member(items, name), list);
        let (a, b) = (Expr::var("a"), Expr::var("b"));
        Ok(match key {
            // Sorting, in place (ADR 0036). JS's `sort()` compares as strings:
            // right for `bool`s, and numbers need `a - b`, strings `$cmp`, by
            // code point (ADR 0183).
            None => {
                let elem = match receiver_ty.peel_refs().kind() {
                    ty::Slice(t) | ty::Array(t, _) => *t,
                    _ => return Err(self.unsupported(span, "sorting this")),
                };
                // A comparator's answer is a number, so a BigInt's is its `cmp`.
                if Num::of(elem).is_some_and(|n| !n.big()) {
                    let js_span = self.js_span(span);
                    let f = Expr::arrow(
                        vec!["a".into(), "b".into()],
                        vec![StmtKind::Return(Some(Expr::bin(Op::Sub, a, b))).at(js_span)],
                    );
                    method(items, "sort", vec![f])
                } else if elem.is_bool() {
                    method(items, "sort", vec![])
                } else {
                    // By its `cmp`: JS's `sort` is stable too (ADR 0057).
                    let compare = self.cmp_fn(elem, false, span)?;
                    self.sort_by_compare(items, compare, elem)
                }
            }
            Some(key) => {
                let key = if matches!(key.kind, js::ExprKind::Var(_)) {
                    key
                } else {
                    self.spill("key", key, out)
                };
                let js_span = self.js_span(span);
                // The keys' `cmp`, which is `$cmp` for what JS orders.
                let key_ty = generic_args.types().nth(1).expect("`sort_by_key` names its key");
                let mut body = Vec::new();
                let compare = self.cmp_value(
                    Expr::call(key.clone(), vec![a]),
                    Expr::call(key, vec![b]),
                    key_ty,
                    false,
                    span,
                    &mut body,
                )?;
                // A key read more than once, like a tuple's, is a `const` first.
                body.push(StmtKind::Return(Some(compare)).at(js_span));
                let f = Expr::arrow(vec!["a".into(), "b".into()], body);
                match receiver_ty.peel_refs().kind() {
                    ty::Slice(elem) | ty::Array(elem, _) => self.sort_by_compare(items, f, *elem),
                    _ => method(items, "sort", vec![f]),
                }
            }
        })
    }

    /// Whether a closure of type `compare_ty` sorting strings, `elem`s, is
    /// JS's own order of strings, by their UTF-16 units, `|a, b|
    /// a.encode_utf16().cmp(b.encode_utf16())`: its `sort()`, given no
    /// function (ADR 0945).
    pub(in crate::lower) fn sorts_as_js_strings(&self, compare_ty: ty::Ty<'tcx>, elem: ty::Ty<'tcx>) -> bool {
        let ty::Closure(id, _) = *compare_ty.kind() else {
            return false;
        };
        self.is_string_like(elem)
            && !elem.peel_refs().is_char()
            && (self.krate.closures.get(&id))
                .is_some_and(|body| crate::lower::body_queries::orders_by_utf16_units(self.tcx, &body.thir, body.expr))
    }

    /// `items` sorted in place by `compare`, stably: JS's `sort`, or
    /// `$sort` of items that can be `undefined`, a `None`, which JS's
    /// `sort` puts last without asking `compare`.
    pub(in crate::lower) fn sort_by_compare(&self, items: Expr, compare: Expr, elem: ty::Ty<'tcx>) -> Expr {
        match self.can_be_nullish(elem) {
            true => Expr::call(Expr::helper("$sort"), vec![items, compare]),
            false => Expr::call(Expr::member(items, "sort"), vec![compare]),
        }
    }
    /// One of std's iterator sources (ADR 0128): `None` if `known` is another.
    pub(in crate::lower) fn iter_source_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match known {
            // Each's items, as `zip`'s argument's are (ADR 0528): of items
            // with no destructor, which `a.zip(b)` drops where `b` runs out.
            Std::IterSource(IterSource::Zip) => {
                let (a, b) = (arg(), arg());
                let held = [tys[0], tys[1]].map(|ty| self.iterable_item(ty));
                if held
                    .iter()
                    .any(|&item| self.drops(item) != crate::lower::drops::Drops::Nothing)
                {
                    return Err(self.unsupported(span, "`std::iter::zip` of items with a destructor"));
                }
                let a = self.iterable_items(a, tys[0], span, out)?;
                let b = self.iterable_items(b, tys[1], span, out)?;
                self.zipped((a, tys[0]), (b, tys[1]), span)?
            }
            Std::IterSource(IterSource::Once) => Expr::array(vec![arg()]),
            Std::IterSource(IterSource::Empty) => Expr::array(vec![]),
            // Its value's drop too, which it runs when it's closed (ADR 0422);
            // `repeat_n`'s, its count's, the last its value (ADR 0513).
            Std::IterSource(source @ (IterSource::Repeat | IterSource::RepeatN)) => {
                let mut list = vec![arg()];
                if source == IterSource::RepeatN {
                    list.push(arg());
                }
                let item = generic_args.type_at(0);
                let clone = match self.needs_clone(item) {
                    true => Some(self.clone_fn("value", item, span)?),
                    false => None,
                };
                let drop = self.drop_function(item, span)?;
                let args = Expr::optional_args(list, vec![clone, drop]);
                match source {
                    IterSource::Repeat => Expr::call(Expr::helper("$repeating"), args),
                    _ => repeating_n(args),
                }
            }
            Std::IterSource(IterSource::RepeatWith) => Expr::call(Expr::helper("$repeatingWith"), vec![arg()]),
            // Their closures' `Option`s: a `Some` that looks like `None` is boxed (ADR 0051).
            Std::IterSource(source @ (IterSource::Successors | IterSource::FromFn)) => {
                let item = generic_args.type_at(0);
                let name = match source {
                    IterSource::Successors => "$successors",
                    _ => "$fromFn",
                };
                let boxed = self.boxed_payload(item);
                let mut list: Vec<Expr> = (0..tys.len()).map(|_| arg()).collect();
                // What it holds dropped where it's closed, `successors`'
                // next item (ADR 0465).
                let drop = match source {
                    IterSource::Successors if self.drops(item) != crate::lower::drops::Drops::Nothing => {
                        self.drop_function(item, span)?
                    }
                    _ => None,
                };
                if boxed || drop.is_some() {
                    list.push(Expr::bool(boxed));
                }
                list.extend(drop);
                Expr::call(Expr::helper(name), list)
            }
            _ => return Ok(None),
        }))
    }
}

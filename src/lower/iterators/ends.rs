//! What ends a chain: its extremes, its last, its sort, and a call of its
//! source's that runs Rust.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `test`, a stage's predicate, as one that drops the `item` it
    /// discards: one `filter` doesn't keep, or one `skip_while` skips.
    pub(in crate::lower) fn dropping_discarded(
        &mut self,
        test: Expr,
        item: ty::Ty<'tcx>,
        discard_when: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let test = if matches!(test.kind, js::ExprKind::Var(_)) {
            test
        } else {
            self.spill(if discard_when { "skip" } else { "keep" }, test, out)
        };
        let js_span = self.js_span(span);
        let mut drop = Vec::new();
        self.drop_value(Expr::var("item"), item, span, &mut drop)?;
        let asked = Expr::call(test, vec![Expr::var("item")]);
        let answer = |b: bool| StmtKind::Return(Some(Expr::bool(b))).at(js_span);
        let body = if discard_when {
            drop.push(answer(true));
            vec![StmtKind::If(asked, drop, None).at(js_span), answer(false)]
        } else {
            let mut rest = vec![StmtKind::If(asked, vec![answer(true)], None).at(js_span)];
            rest.extend(drop);
            rest.push(answer(false));
            rest
        };
        Ok(Expr::arrow(vec!["item".into()], body))
    }
    /// `max()` or `min()` of `items`, an array of `item`s: of
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
                self.extreme_by(helper, vec![items, compare], item, span)?
            }
            _ => Expr::call(Expr::helper(if max { "$max" } else { "$min" }), vec![items]),
        })
    }

    /// `helper`, `$maxBy` or one of its kind, given `args`, then whether an
    /// `item` is boxed where it may look like `None` (ADR 0051), and what
    /// drops each passed over, of one with a destructor (ADR 0098).
    pub(in crate::lower) fn extreme_by(
        &mut self,
        helper: &'static str,
        mut args: Vec<Expr>,
        item: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let boxed = self.boxed_payload(item);
        let drop = self.std_drop_function(item, span)?;
        if boxed || drop.is_some() {
            args.push(Expr::bool(boxed));
        }
        args.extend(drop);
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
                Expr::call_helper("$lazyZip", vec![a, b])
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
                let name = match source {
                    IterSource::Repeat => "$repeating",
                    _ => "$repeatingN",
                };
                Expr::call(Expr::helper(name), Expr::optional_args(list, vec![clone, drop]))
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

    /// Does closure `e` call a function of the crate, or a closure, a generic
    /// `impl Fn`'s too, whose JS takes the arguments Rust gives it alone? And,
    /// `dyn_fn`, a `dyn Fn`, a prop's, a JS caller's function as its type says.
    pub(in crate::lower) fn calls_rust(&self, e: ExprId, dyn_fn: bool) -> bool {
        let ExprKind::Closure(ref closure) = self.thir[self.strip(e)].kind else {
            return false;
        };
        self.closure_calls_rust(self.krate.closures[&closure.closure_id.to_def_id()], dyn_fn)
    }

    /// `calls_rust`, of a closure's body.
    pub(in crate::lower) fn closure_calls_rust(&self, body: &crate::lower::Body<'tcx>, dyn_fn: bool) -> bool {
        let thir = &body.thir;
        let Some(root) = body.root() else { return false };
        let mut call = crate::lower::body_queries::strip(thir, root);
        if let ExprKind::Block { block } = thir[call].kind
            && thir[block].stmts.is_empty()
            && let Some(value) = thir[block].expr
        {
            call = crate::lower::body_queries::strip(thir, value);
        }
        let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
            return false;
        };
        let Some((def_id, _)) = fn_def(thir[fun].ty) else {
            return false;
        };
        match self.tcx.trait_of_assoc(def_id) {
            Some(fn_trait) if self.tcx.fn_trait_kind_from_def_id(fn_trait).is_some() => {
                match thir[args[0]].ty.peel_refs().kind() {
                    ty::Closure(..) | ty::Param(_) => true,
                    ty::Dynamic(..) => dyn_fn,
                    ty::Adt(adt, args) if adt.is_box() => dyn_fn && matches!(args.type_at(0).kind(), ty::Dynamic(..)),
                    _ => false,
                }
            }
            _ => def_id.is_local() && !crate::lower::bindings::is_binding(self.tcx, def_id),
        }
    }
}

//! What ends a chain from MIR by its items: a sum, a collection made, a
//! position found, a fold.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `it.sum()`, or `product()`, `sum`, into an `output` of `item`s: of a
    /// number, `reduce` by its operation; of a type parameter's, its
    /// dictionary's, `SSum.sum(it)`, its caller's (ADR 0476).
    pub(super) fn accumulated(
        &mut self,
        (it, method): (Expr, DefId),
        (output, item): (Ty<'tcx>, Option<Ty<'tcx>>),
        sum: bool,
        span: Span,
    ) -> R<Expr> {
        let name = if sum { "sum" } else { "product" };
        if let Some(num) = Num::of(output) {
            return Ok(self.accumulated_numbers(it, num, sum));
        }
        // Into an `Option` or a `Result` of numbers: the first `None` or
        // `Err`, where it stops, or the numbers' own, as `collect()` into one
        // gives them.
        let option = self.option_of(output);
        let result = (self.recognition().is_result(output))
            .then(|| match output.kind() {
                ty::Adt(_, args) => Some(args.type_at(0)),
                _ => None,
            })
            .flatten();
        if let Some(num) = option.or(result).and_then(Num::of) {
            let values = self.fresh("values");
            let total = self.accumulated_numbers(Expr::var(&values), num, sum);
            let into = Expr::arrow(
                vec![values.into()],
                vec![StmtKind::Return(Some(total)).at(js::Span::NONE)],
            );
            return Ok(match option {
                Some(_) => Expr::call_helper("$collectOptions", vec![it, Expr::bool(false), into]),
                None => Expr::call_helper("$collectResults", vec![it, into]),
            });
        }
        let unsupported = || self.unsupported(span, &format!("`{name}` of a `{output}`, from its MIR"));
        let (Some(item), Some(accumulation)) = (item, crate::lower::recognition::accumulation_of(self.tcx, method))
        else {
            return Err(unsupported());
        };
        let dictionary = self.dictionary(ty::TraitRef::new(self.tcx, accumulation, [output, item]), span)?;
        Ok(Expr::call(Expr::member(dictionary, name), vec![it]))
    }

    /// `it.sum()`, or `product()`, `sum`, of numbers of `num`'s type:
    /// `reduce` by its wrapping operation, from its zero or one; a float's
    /// sum from -0.0, as Rust's keeps an empty sum's and one of negative
    /// zeros' sign.
    pub(in crate::lower) fn accumulated_numbers(&mut self, it: Expr, num: Num, sum: bool) -> Expr {
        let (op, start) = match sum {
            true => (Op::Add, 0),
            false => (Op::Mul, 1),
        };
        let (a, b) = (self.fresh("a"), self.fresh("b"));
        let total = num.wrap(Expr::bin(op, Expr::var(&a), Expr::var(&b)));
        let f = Expr::arrow(
            vec![a.into(), b.into()],
            vec![StmtKind::Return(Some(total)).at(js::Span::NONE)],
        );
        let start = match (sum, num.float()) {
            (true, true) => Expr::num(-0.0),
            _ => num.literal(start),
        };
        Expr::call(Expr::member(it, "reduce"), vec![f, start])
    }

    /// `it.collect()` into `output`: a heap of its items in heap order,
    /// `$heapFrom`; an array, a range's mapped where it's made; text, its
    /// pieces joined; a `Result` or an `Option` of one of these, the first
    /// `Err` or `None` or all; a map or a set, as a `From` of items makes one
    /// (ADR 0121). Of the `Ok` values of a fallible one, an array already,
    /// `iter_ty` is none, and an array is itself.
    /// `B::from_iter(items)`, `output` of `items` of type `given`: an
    /// array's, a `Vec`'s or a slice's items as they are, another's iterator
    /// as `collect` steps it, collected into `B` as `collect` collects.
    pub(super) fn collected_from(
        &mut self,
        values: Vec<Expr>,
        given: Ty<'tcx>,
        output: Ty<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        let items = values.into_iter().next().expect("the items");
        let iter_ty = given.peel_refs();
        if iter_ty.is_array() || iter_ty.is_slice() || self.is_vec_like(iter_ty) {
            let item = match *iter_ty.kind() {
                ty::Array(item, _) | ty::Slice(item) => Some(item),
                ty::Adt(_, args) => args.types().next(),
                _ => None,
            };
            return Ok(Some(self.collected_into(items, output, item, None, span)?));
        }
        // A map's or a set's items, in its order: its entries, a map's, or
        // its items, a set's.
        if self.is_map(iter_ty)
            && let ty::Adt(_, args) = iter_ty.kind()
        {
            let item = match self.is_set(iter_ty) {
                true => args.type_at(0),
                false => Ty::new_tup(self.tcx, &[args.type_at(0), args.type_at(1)]),
            };
            let items = self.map_items(items, iter_ty, crate::lower::std_types::map::Part::Entries, span)?;
            return Ok(Some(self.collected_into(items, output, Some(item), None, span)?));
        }
        if !matches!(given.kind(), ty::Param(_)) && !self.implements_iterator(given) {
            return Ok(None);
        }
        let it = self.as_js_iterator(items, given, span)?;
        let item = self.iterator_item(given);
        Ok(Some(self.collected_into(it, output, item, Some(given), span)?))
    }

    pub(in crate::lower) fn collected_into(
        &mut self,
        it: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let method = |name: &str, it: Expr, args: Vec<Expr>| Expr::call(Expr::member(it, name), args);
        let items = |it: Expr| match iter_ty {
            Some(_) => method("toArray", it, Vec::new()),
            None => it,
        };
        // Into what holds a type parameter that a bound says it's collected
        // into, a `B: FromIterator<A>` or a `Result<U, E>`'s: its
        // dictionary's `from_iter`, its caller's (ADRs 0445, 0476).
        if output.has_param()
            && let Some(item) = item
            && let Some(dictionary) = self.collected_evidence(output, item)
        {
            let from_iter = std_item(self.tcx, StdItem::FromIterator);
            let from_iter = crate::lower::bindings::trait_method_name(self.tcx, from_iter, "from_iter");
            return Ok(Expr::call(Expr::member(dictionary, from_iter), vec![it]));
        }
        Ok(if self.is_std_type(output, StdItem::BinaryHeap) {
            self.heap_values(HeapOp::From, vec![items(it)], output, span)?
        // A `Vec`, or a `Box<[T]>`, an array too (ADR 0530).
        } else if self.is_vec_like(output) || output.boxed_ty().is_some_and(|inner| inner.is_slice()) {
            let Some(iter_ty) = iter_ty else { return Ok(it) };
            match self.mapped_range(&it, iter_ty) {
                Some(items) => items,
                None => match self.collected_items(&it, iter_ty) {
                    Some(items) => items,
                    None => method("toArray", it, Vec::new()),
                },
            }
        } else if self.is_lang_adt(output, LangItem::String) {
            method("join", items(it), vec![Expr::str("")])
        } else if self.option_of(output).is_some() || self.recognition().is_result(output) {
            // What's left of what it steps, dropped where it stops: closed.
            let it = match iter_ty {
                Some(iter_ty) => self.closing(it, iter_ty, span)?,
                None => it,
            };
            self.collect_fallible(it, output, item, span)?
        } else if self.is_map(output) {
            let set = self.is_set(output);
            self.made_of(set, self.tcx.mk_args(&[output.into()]), it, span)?
        } else if let Some(made) = self.made_by_own_from_iter(it.clone(), output, item, span)? {
            made
        } else {
            return Err(self.unsupported(span, &format!("`collect` into a `{output}`, from its MIR")));
        })
    }

    /// `items` made a collection of the crate's own, `output`, by its
    /// `FromIterator<item>`'s `from_iter`, the function rust-js compiled.
    /// The `FromIterator<item>` evidence of what holds a type parameter,
    /// `output`, that generic code collects into (ADRs 0445, 0476).
    fn collected_evidence(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> Option<Expr> {
        let from_iterator = std_item(self.tcx, StdItem::FromIterator);
        self.evidence_for(ty::TraitRef::new(self.tcx, from_iterator, [output, item]))
    }

    fn made_by_own_from_iter(
        &mut self,
        items: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Option<Expr>> {
        let Some((from_iter, args)) = item.and_then(|item| self.recognition().values_from_iter(output, item)) else {
            return Ok(None);
        };
        let Some(instance) = self
            .resolve_instance(from_iter, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
        else {
            return Ok(None);
        };
        let mut values = vec![items];
        values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
        Ok(Some(Expr::call(self.fn_ref(instance.def_id()), values)))
    }

    /// `position(f)`: where the first that `f` finds is, `$position`; and
    /// `rposition(f)`, of a slice's items, the last, `$rposition`, `f`
    /// called from the back as Rust's is, of a `$iter` lent as it is, which
    /// it steps from its back. Of a lazy chain, whose closures would run in
    /// another order, refused.
    pub(super) fn positioned(
        &mut self,
        it: Expr,
        f: Expr,
        (own, method): (bool, IterMethod),
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let found = self.unary_callback(f, own);
        if method == IterMethod::Position {
            return Ok(Expr::call_helper("$position", vec![it, found]));
        }
        // Of one that knows its back and its length, stepped from its back.
        if self.array_source(iter_ty).is_none() {
            let back = self.knows_its_back(iter_ty) || self.user_double_ended(iter_ty);
            if !(back && (self.knows_its_length(iter_ty) || self.user_exact_size(iter_ty))) {
                return Err(self.unsupported(span, "`rposition` of a lazy iterator, from its MIR"));
            }
            // What a `&mut` lends, which it's stepped from its back as it is.
            let it = crate::lower::iterators::unlent(&it).clone();
            let it = self.as_exact_size(it, iter_ty, true, span)?;
            return Ok(Expr::call_helper("$rposition", vec![it, found]));
        }
        let items = crate::lower::iterators::unlent(&it).clone();
        Ok(Expr::call_helper("$rposition", vec![items, found]))
    }

    /// `it.fold(init, f)`: `it.reduce(f, init)`, `f` given after what it
    /// starts from. JS's `reduce` counts its arguments: a start of
    /// `undefined`, a `fold(None, ..)`'s, is given, or it starts from the
    /// first item (ADR 0330).
    pub(super) fn folded(
        &mut self,
        it: Expr,
        (init, f): (Option<Expr>, Expr),
        (arg_tys, item): (&[Ty<'tcx>], Option<Ty<'tcx>>),
    ) -> Expr {
        let own = arg_tys.last().is_some_and(|ty| self.is_crate_fn_item(*ty));
        let callback = self.binary_callback(f, own);
        let Some(init) = init else {
            // `reduce(f)`: from its first item, `None` of none (ADR 0479).
            let mut args = vec![it, callback];
            if item.is_some_and(|item| self.boxed_payload(item)) {
                args.push(Expr::bool(true));
            }
            return Expr::call_helper("$reduce", args);
        };
        let mut args = vec![callback, init];
        crate::lower::calls::given_to_js(&mut args);
        Expr::call(Expr::member(it, "reduce"), args)
    }

    /// `it.try_fold(init, f)` into `output`, an `Option` or a `Result`: each
    /// item folded in till `f` gives a `None` or an `Err`, the rest left to
    /// step, `$tryFoldSome` or `$tryFoldOk` (ADR 0458).
    pub(super) fn try_folded(
        &mut self,
        it: Expr,
        method: IterMethod,
        arg: &mut dyn FnMut() -> Expr,
        (own, output): (bool, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        // Into a `fmt::Result`, whose `Ok(())` is nothing and whose `Err` is
        // thrown (ADR 0187): `try_for_each(f)` is each item's call, as
        // `for_each`'s (ADR 0539).
        if self.recognition().is_fmt_result(output) {
            return match method {
                IterMethod::TryForEach => Ok(Expr::call(
                    Expr::member(it, "forEach"),
                    vec![self.unary_callback(arg(), own)],
                )),
                _ => Err(self.unsupported(span, "`try_fold` into a `fmt::Result`, from its MIR")),
            };
        }
        let helper = match self.option_of(output) {
            Some(_) => "$tryFoldSome",
            None if self.recognition().is_result(output) => "$tryFoldOk",
            _ => return Err(self.unsupported(span, &format!("`try_fold` into a `{output}`, from its MIR"))),
        };
        // `try_for_each(f)`, std's `try_fold((), |(), x| f(x))` (ADR 0539).
        let (init, f) = match method {
            IterMethod::TryForEach => (Expr::undefined(), self.unary_folded(arg())),
            _ => (arg(), arg()),
        };
        let mut args = vec![it, init, self.binary_callback(f, own)];
        crate::lower::calls::given_to_js(&mut args);
        Ok(Expr::call_helper(helper, args))
    }
}

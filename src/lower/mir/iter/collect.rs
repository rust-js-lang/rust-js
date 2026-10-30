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

    /// Whether std's `collect` into an `output` takes its items by `fold`,
    /// its `extend` by `for_each`: a string's, a hash map's or set's, a
    /// list's and `()`'s; not a `Vec`'s, a deque's, a heap's or a B-tree's,
    /// which step them by `next`.
    pub(in crate::lower) fn collects_by_fold(&self, output: Ty<'tcx>) -> bool {
        self.is_lang_adt(output, LangItem::String)
            || self.is_map(output) && !self.is_sorted(output)
            || self.is_std_type(output, StdItem::LinkedList)
            || output.is_unit()
    }

    /// Whether std's `method` of an iterator, giving `output`, takes its
    /// items by `fold`, `for_each` or `reduce`: what calls the crate's own
    /// `fold`, which takes the iterator and drops it (#162).
    pub(in crate::lower) fn consumes_by_fold(&self, method: DefId, output: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        recognition.folds_what_it_consumes(method)
            || recognition.iterator_method(method) == Some(IterMethod::Collect) && self.collects_by_fold(output)
    }

    /// `it`, an iterator of type `iter_ty`, as a consumer std routes through
    /// `fold` takes it: where that reaches the crate's own `fold` (#162), or
    /// asks its `nth`, as a `skip`'s and a `step_by`'s does (#182), or
    /// steps a lent inclusive range (#414), `$folded(it)`, its items as that
    /// gives them; else `it`.
    pub(in crate::lower) fn by_its_fold(&self, it: Expr, iter_ty: Ty<'tcx>) -> Expr {
        let recognition = self.recognition();
        match recognition.folds_its_own(iter_ty)
            || recognition.folds_by_its_own_nth(iter_ty)
            || self.steps_lent_inclusive(iter_ty)
        {
            true => Expr::call_helper("$folded", vec![it]),
            false => it,
        }
    }

    /// `collect()` of `it`, an iterator of type `iter_ty`, into `output` of
    /// `item`s: of a `Vec` of one made in place whose length `len_in_place`
    /// counts of a count the program gives, `repeat_n(x, n)`'s, a `take(n)`
    /// of an endless one, a range's `step_by(s)`, not an array that's there
    /// already, that nothing else asks room for, a range's `$range(a, b,
    /// size)` or a struct's `$roomForHint`, the
    /// room std's `SpecFromIter` asks first, `room_for`'s `$roomFor(n, size)`,
    /// a `capacity overflow` panic past `isize::MAX` bytes (#246). Of a
    /// `TrustedLen` one, before it's stepped; of another, which std steps
    /// once first, one whose steps run no code. What makes it, where that
    /// runs code, held first. `receiver`, the type `collect` is given, an
    /// inclusive range lent, `&mut r`, is what's collected as that, by its
    /// `fold` (#414).
    pub(in crate::lower::mir) fn collected_counted(
        &mut self,
        it: Expr,
        (output, item): (Ty<'tcx>, Option<Ty<'tcx>>),
        receiver: Ty<'tcx>,
        span: Span,
        made: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let iter_ty = self.reveal(receiver.peel_refs());
        let runs_code = |ty: Ty<'tcx>| {
            ty.walk()
                .any(|part| part.as_type().is_some_and(|t| t.is_closure() || t.is_fn()))
        };
        use crate::lower::recognition as r;
        let given_count = |ty: Ty<'tcx>| {
            r::is_repeat_n(self.tcx, ty)
                || r::stepped_by_source(self.tcx, ty).is_some()
                || r::counted_source(self.tcx, ty)
                    .is_some_and(|(source, skip)| !skip && self.recognition().endless_by_type(source))
        };
        let counted = item.is_some()
            && self.is_vec_like(output)
            && iter_ty.walk().any(|part| part.as_type().is_some_and(given_count))
            && (crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty) || !runs_code(iter_ty))
            && !crate::lower::iterators::is_stepped_items(&it)
            && self.range_kind(iter_ty).is_none()
            && self.mapped_range(&it, iter_ty, item).is_none()
            && !self.compiled_struct_at(iter_ty, span);
        let mut held = Vec::new();
        let room = match (counted, item) {
            (true, Some(item)) => self.len_in_place(&it, iter_ty, &mut held).and_then(|len| {
                let size = self.room_size_of(&len, item)?;
                Some((len, size))
            }),
            _ => None,
        };
        let it = match room {
            Some((len, size)) if held.is_empty() => {
                let it = match it.has_effects() {
                    true => self.spill("it", it, made),
                    false => it,
                };
                self.room_for((None, len), size, span, made);
                it
            }
            _ => it,
        };
        // Into a `Vec` alone, the array itself: a heap's is made of it.
        if self.is_std_type(output, crate::lower::recognition::StdItem::Vec)
            && let Some(items) = self.counted_range(&it, iter_ty, item, span, made)
        {
            return Ok(items);
        }
        let collected = match self.steps_lent_inclusive(receiver) {
            true => receiver,
            false => iter_ty,
        };
        self.collected_into(it, output, item, Some(collected), span)
    }

    /// Whether std collects an iterator of `iter_ty`, into a `Vec` of
    /// `item`s, in place, `recognition::collects_in_place`'s, where rustc
    /// knows both whole (#423).
    fn collects_in_place(&self, iter_ty: Option<Ty<'tcx>>, item: Option<Ty<'tcx>>) -> Option<bool> {
        let (iter_ty, item) = iter_ty.zip(item)?;
        let whole = |ty: Ty<'tcx>| !ty.has_param() && !ty.has_aliases();
        (whole(iter_ty) && whole(item)).then(|| crate::lower::recognition::collects_in_place(self.tcx, iter_ty, item))
    }

    /// Whether what's left of a collect's iterator, of `iter_ty`, is
    /// dropped before what it collected into `output` of `item`s, where a
    /// panic stops it: of a hash map or set, or a `LinkedList`, which std
    /// extends as each comes; of another, which it collects as a `Vec`,
    /// where it doesn't collect it in place (#423).
    fn rest_dropped_first(&self, output: Ty<'tcx>, iter_ty: Option<Ty<'tcx>>, item: Ty<'tcx>) -> bool {
        (self.is_map(output) && !self.is_sorted(output))
            || self.is_std_type(output, StdItem::LinkedList)
            || self.collects_in_place(iter_ty, Some(item)) == Some(false)
    }

    /// Of a collect into `output` of `item`s a panic may stop, `it` a
    /// chain whose stages may throw, from an owning source or of items with
    /// a destructor: how what it holds is dropped as std's unwinds (#422,
    /// #423). `None` where it holds nothing a panic would leave.
    fn unwinding_collect(
        &mut self,
        it: &Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Option<UnwindingCollect>> {
        if !crate::lower::iterators::chain_may_throw(it) {
            return Ok(None);
        }
        let drop = match item {
            Some(item) if self.drops(item) != crate::lower::drops::Drops::Nothing => self.drop_function(item, span)?,
            _ => None,
        };
        if drop.is_none() && !crate::lower::iterators::closes_a_source(it) {
            return Ok(None);
        }
        let rest_first = drop.is_some() && item.is_some_and(|item| self.rest_dropped_first(output, iter_ty, item));
        Ok(Some(UnwindingCollect { drop, rest_first }))
    }

    /// Whether stepping an iterator of `iter_ty` may panic by what it runs,
    /// as its type shows where its JS is still a variable: a closure or a
    /// function of its type's, each but one of the crate's functions that
    /// never unwinds (ADR 0902) or a constructor (#423).
    pub(in crate::lower::mir) fn steps_may_panic(&self, iter_ty: Ty<'tcx>) -> bool {
        iter_ty
            .walk()
            .filter_map(|part| part.as_type())
            .any(|part| match *part.kind() {
                ty::Closure(def_id, _) | ty::FnDef(def_id, _) => {
                    !self.krate.never_unwinding.contains(&def_id)
                        && !matches!(self.tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))
                }
                _ => false,
            })
    }

    /// `$foldToArray`'s arguments, `it` folded into a `Vec` of `item`s:
    /// of items with a destructor, from a type that runs a closure or a
    /// `fn` as it's stepped, which may panic, their drop, which what it
    /// collected is dropped by as std's unwinds, and whether what's left
    /// is first (#423).
    fn folded_unwinding(
        &mut self,
        it: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Vec<Expr>> {
        let runs_code = iter_ty
            .walk()
            .any(|part| part.as_type().is_some_and(|t| t.is_closure() || t.is_fn()));
        let drop = match item {
            Some(item) if runs_code && self.drops(item) != crate::lower::drops::Drops::Nothing => {
                self.drop_function(item, span)?
            }
            _ => None,
        };
        let mut args = vec![it];
        if let (Some(drop), Some(item)) = (drop, item) {
            args.push(drop);
            if self.rest_dropped_first(output, Some(iter_ty), item) {
                args.push(Expr::bool(true));
            }
        }
        Ok(args)
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
        let it = match iter_ty {
            Some(iter_ty) if self.collects_by_fold(output) => self.by_its_fold(it, iter_ty),
            _ => it,
        };
        let items = |it: Expr| match iter_ty {
            Some(_) => Expr::collected(it),
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
            let items = match self.unwinding_collect(&it, output, item, iter_ty, span)? {
                Some(unwinding) => unwinding.collected(it),
                None => items(it),
            };
            self.heap_values(HeapOp::From, vec![items], (output, false), span)?
        // A `Vec`, or a `Box<[T]>`, an array too (ADR 0530).
        } else if self.is_vec_like(output) || output.boxed_ty().is_some_and(|inner| inner.is_slice()) {
            let Some(iter_ty) = iter_ty else { return Ok(it) };
            // Of std's struct, the room its hint asks first, as std's
            // `SpecFromIter` does, and of one whose type has no end, the
            // `capacity overflow` a `TrustedLen`'s none asks (ADR 0940). Not
            // of a `LinkedList`, whose `from_iter` extends a new one, asking
            // none.
            let endless = crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty)
                && self.recognition().endless_by_type(iter_ty);
            let asks_room = !self.is_std_type(output, StdItem::LinkedList);
            let it = match item {
                Some(item)
                    if asks_room
                        && (endless
                            || self.compiled_struct_at(iter_ty, span)
                                && !self.recognition().bounded_by_items(iter_ty)) =>
                {
                    let trusted = crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty);
                    let size = self.room_item_size(item);
                    let mut given = vec![it, size, Expr::bool(trusted)];
                    if endless {
                        given.push(Expr::bool(true));
                    }
                    Expr::call_helper("$roomForHint", given)
                }
                _ => it,
            };
            // Of std's struct that's `TrustedLen`, by its `fold`, as std's
            // `extend_trusted` collects it, a zip's `SpecFold` (#380), and
            // of one stepping a lent inclusive range, which that leaves
            // exhausted at its last (#414).
            let folded = (self.compiled_struct_at(iter_ty, span) || self.steps_lent_inclusive(iter_ty))
                && crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty);
            match self.mapped_range(&it, iter_ty, item) {
                Some(items) => items,
                None if let Some(unwinding) = self.unwinding_collect(&it, output, item, Some(iter_ty), span)? => {
                    unwinding.collected(it)
                }
                None if folded && self.steps_lent_inclusive(iter_ty) => {
                    let args = self.folded_unwinding(it, output, item, iter_ty, span)?;
                    Expr::call_helper("$foldToArray", args)
                }
                None => match self.collected_items(&it, iter_ty) {
                    Some(items) => items,
                    None if folded => {
                        let args = self.folded_unwinding(it, output, item, iter_ty, span)?;
                        Expr::call_helper("$foldToArray", args)
                    }
                    None => Expr::collected(it),
                },
            }
        } else if self.is_lang_adt(output, LangItem::String) {
            let items = match self.unwinding_collect(&it, output, item, iter_ty, span)? {
                Some(unwinding) => unwinding.collected(it),
                None => items(it),
            };
            method("join", items, vec![Expr::str("")])
        } else if self.option_of(output).is_some() || self.recognition().is_result(output) {
            // What's left of what it steps, dropped where it stops: closed.
            let it = match iter_ty {
                Some(iter_ty) => self.closing(it, iter_ty, span)?,
                None => it,
            };
            // What's left first, where std collects it otherwise than in
            // place, of values with a destructor (#423).
            let ty::Adt(_, args) = output.kind() else {
                unreachable!("a `Result` or an `Option`")
            };
            let value = item.and_then(|item| match item.kind() {
                ty::Adt(_, args) => args.types().next(),
                _ => None,
            });
            let rest_first = value.is_some_and(|value| self.rest_dropped_first(args.type_at(0), iter_ty, value));
            self.collect_fallible(it, output, item, rest_first, span)?
        } else if self.is_map(output) {
            let set = self.is_set(output);
            // A panic that may stop it: a B-tree's, which std collects as a
            // `Vec` first, collected so; another's, which extends a new one,
            // `$mapFrom`, what's left then what it holds dropped (#423).
            let unwinding = self.unwinding_collect(&it, output, item, iter_ty, span)?;
            let (it, unwinds) = match unwinding {
                Some(unwinding) if self.is_sorted(output) => (unwinding.collected(it), false),
                unwinding => (it, unwinding.is_some()),
            };
            self.made_of(set, self.tcx.mk_args(&[output.into()]), it, unwinds, span)?
        } else if let Some(made) = self.made_by_own_from_iter(it.clone(), output, item, span)? {
            made
        // Into `()`: each item stepped and dropped, as std's body,
        // `iter.for_each(|()| {})`, steps them.
        } else if output.is_unit() {
            method("forEach", it, vec![Expr::arrow(Vec::new(), Vec::new())])
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
        // Of one that knows its back and its length, stepped from its back,
        // made so as `as_exact_size` makes it, a map made here of an
        // array's items its `$mapEnds`, or refused there.
        if self.array_source(iter_ty).is_none() {
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
        span: Span,
    ) -> R<Expr> {
        let own = arg_tys.last().is_some_and(|ty| self.is_called_as_written(*ty));
        let callback = self.binary_callback(f, own);
        // Of an accumulator with a destructor, where a stage may throw, a
        // panic, what drops the one it strands, as std's adapter's fold
        // closure drops it (#423).
        let acc = match init {
            Some(_) => arg_tys.get(1).copied(),
            None => item,
        };
        let receiver = arg_tys.first().copied();
        // Of a type parameter's too, whose steps are anyone's (#424).
        let may_throw = crate::lower::iterators::chain_may_throw(&it)
            || receiver.is_some_and(|ty| ty.has_param() || self.steps_may_panic(ty));
        let unwind = match acc {
            Some(acc) if may_throw && self.drops(acc) != crate::lower::drops::Drops::Nothing => {
                self.drop_function(acc, span)?
            }
            _ => None,
        };
        let Some(init) = init else {
            // `reduce(f)`: from its first item, `None` of none (ADR 0479).
            let mut args = vec![it, callback];
            let boxed = item.is_some_and(|item| self.boxed_payload(item));
            if boxed || unwind.is_some() {
                args.push(Expr::bool(boxed));
            }
            args.extend(unwind);
            return Ok(Expr::call_helper("$reduce", args));
        };
        let mut args = vec![callback, init];
        crate::lower::calls::given_to_js(&mut args);
        // `$foldDropping(it, init, f, drop)`, which passes it on (#423).
        if let Some(unwind) = unwind {
            args.swap(0, 1);
            return Ok(Expr::call_helper(
                "$foldDropping",
                [vec![it], args, vec![unwind]].concat(),
            ));
        }
        Ok(Expr::call(Expr::member(it, "reduce"), args))
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
        // Into another `Try`, std's generic code's `R` or a `ControlFlow`:
        // by its dictionary, `$tryFoldInto` (ADR 0913).
        let tried = match self.option_of(output) {
            Some(_) => None,
            None if self.recognition().is_result(output) => None,
            None => {
                let try_trait = self.tcx.require_lang_item(LangItem::Try, span);
                Some(self.dictionary(ty::TraitRef::new(self.tcx, try_trait, [output]), span)?)
            }
        };
        let helper = match (self.option_of(output), &tried) {
            (Some(_), _) => "$tryFoldSome",
            (None, None) => "$tryFoldOk",
            (None, Some(_)) => "$tryFoldInto",
        };
        // `try_for_each(f)`, std's `try_fold((), |(), x| f(x))` (ADR 0539).
        let (init, f) = match method {
            IterMethod::TryForEach => (Expr::undefined(), self.unary_folded(arg())),
            _ => (arg(), arg()),
        };
        let mut args = vec![it, init, self.binary_callback(f, own)];
        crate::lower::calls::given_to_js(&mut args);
        args.extend(tried);
        Ok(Expr::call_helper(helper, args))
    }
}

/// How a collect a panic may stop drops what it holds: what it collected,
/// by `drop`, and what's left of what it steps, closed, that first where
/// `rest_first` (#422, #423).
struct UnwindingCollect {
    drop: Option<Expr>,
    rest_first: bool,
}

impl UnwindingCollect {
    /// `it` collected so, an array: `$collectDropping(it, drop)`.
    fn collected(self, it: Expr) -> Expr {
        let mut args = vec![it];
        args.extend(self.drop);
        if self.rest_first {
            args.push(Expr::bool(true));
        }
        Expr::call_helper("$collectDropping", args)
    }
}

//! What ends a chain from MIR by its items other than a collection made:
//! a sum or a product, a position found, a fold or a `try_fold`, a
//! `for_each`, a sort, an extreme by a key, and what each takes and drops of
//! what it consumes. A collection made is `collect.rs`'.

use super::*;
use crate::lower::recognition::{OwnStep, Reach};

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

    /// Whether std's `method` of an iterator, giving `output`, takes its
    /// items by `fold`, `for_each` or `reduce`: what calls the crate's own
    /// `fold`, which takes the iterator and drops it (#162).
    pub(in crate::lower) fn consumes_by_fold(&self, method: DefId, output: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        recognition.folds_what_it_consumes(method)
            || recognition.rfolds_what_it_consumes(method)
            || recognition.iterator_method(method) == Some(IterMethod::Collect) && self.collects_by_fold(output)
    }

    /// Whether a keyed extreme, `max_by_key`'s or `min_by_key`'s, of the
    /// crate's iterator with a destructor, `arg_tys[0]`, closes it, as
    /// `consumer_closes` (closing.rs) says of what it answers: of a key moved into its
    /// fold with a destructor (`consumed_function`), std's `Map` the key's
    /// `reduce` folds drops the iterator, then the key, where its first step
    /// ends, after the fold its key drops before what's left, so its drop is
    /// the consumer's, there (#789).
    pub(in crate::lower::mir) fn keyed_consumer_closes(
        &self,
        method: Option<IterMethod>,
        arg_tys: &[Ty<'tcx>],
    ) -> bool {
        matches!(method, Some(IterMethod::MaxByKey(_)))
            && arg_tys
                .first()
                .is_some_and(|&iter| self.is_user_iterator(iter) && self.closes_to_drop(iter))
            && self.consumed_function(method, arg_tys)
    }

    /// Whether std's consumer `method` of an `iter_ty` moves the function
    /// it's given last, of type `f`, into the fold it runs, which drops it
    /// before what's left of the iterator, as `fold(self, init, f)` drops
    /// `f` first, where that's seen (#734): `fold`'s, `rfold`'s, `for_each`'s
    /// and `reduce`'s, of a
    /// function that drops, over an iterator that drops something after it,
    /// its rest or itself, or whose fold moves a function an adapter holds
    /// (`folds_held_function`), the user's decision; and in std's compiled
    /// body, of the `F` it was given or a closure that carries its drop
    /// (#710); where the crate's own `fold` is reached too, which is given
    /// `ends` as its `F`'s drop (`fold_entry`, #734). Its drop is the fold's
    /// `ends`, `$consume`'s, not `dropped_given`'s after the call: the one
    /// answer of both.
    ///
    /// And of `fold`'s, `rfold`'s, `try_fold`'s and `try_rfold`'s function
    /// that drops, whose accumulator has a destructor a stage's panic may
    /// strand (`stranded_drop`'s): which of the two drops first as it
    /// unwinds is the fold's, an adapter's whose frame holds both, a
    /// `skip_while`'s first step, `f` first, where std's own fold holds the
    /// accumulator in its body, it first (#768). `arg_tys` are the call's,
    /// the iterator's first, `f`'s last, the accumulator's second.
    pub(in crate::lower) fn consumed_function(&self, method: Option<IterMethod>, arg_tys: &[Ty<'tcx>]) -> bool {
        let (Some(&iter_ty), Some(&f)) = (arg_tys.first(), arg_tys.last()) else {
            return false;
        };
        let by_value = matches!(
            method,
            Some(
                IterMethod::Fold
                    | IterMethod::RFold
                    | IterMethod::ForEach
                    | IterMethod::Reduce
                    | IterMethod::IsSortedBy
                    | IterMethod::MaxBy(_)
                    | IterMethod::MaxByKey(_)
            )
        );
        let tried = matches!(method, Some(IterMethod::TryFold | IterMethod::TryRFold));
        // `try_fold(&mut self, ..)`'s receiver is a `&mut` of the iterator.
        let iterator = if tried { iter_ty.peel_refs() } else { iter_ty };
        if !(by_value || tried) || iterator.is_mutable_ptr() {
            return false;
        }
        if crate::lower::recognition::is_compiled_std(self.tcx, self.frame.owner) {
            return self.carries_its_drop(f) || self.given_drop(f).is_some();
        }
        if self.drops(f) == crate::lower::drops::Drops::Nothing {
            return false;
        }
        let stranded = matches!(
            method,
            Some(IterMethod::Fold | IterMethod::RFold | IterMethod::TryFold | IterMethod::TryRFold)
        ) && arg_tys.len() == 3
            && self.steps_may_panic(iterator)
            && self.drops(arg_tys[1]) != crate::lower::drops::Drops::Nothing;
        stranded
            || (by_value
                && (self.drops(iter_ty) != crate::lower::drops::Drops::Nothing
                    || self.folds_held_function(iter_ty)
                    || self.keeps_a_dropping_key(method, arg_tys)))
    }

    /// Whether `max_by_key(f)` or `min_by_key(f)` of `arg_tys` keeps a key
    /// with a destructor, which std drops as `let (_, x) = ..?` takes the
    /// kept pair apart, after the `Map` holding `f` is dropped as `max_by`
    /// returns: so `f`'s drop is the fold's, before the key's (#1231).
    fn keeps_a_dropping_key(&self, method: Option<IterMethod>, arg_tys: &[Ty<'tcx>]) -> bool {
        let (Some(IterMethod::MaxByKey(_)), [iter_ty, f, ..]) = (method, arg_tys) else {
            return false;
        };
        self.iterator_item(*iter_ty).is_some_and(|item| {
            let given = Ty::new_imm_ref(self.tcx, self.tcx.lifetimes.re_erased, item);
            self.drops(self.key_of(*f, given)) != crate::lower::drops::Drops::Nothing
        })
    }

    /// `it`, an iterator of type `iter_ty`, as a consumer std routes through
    /// `fold` takes it: where that reaches the crate's own `fold` (#162), or
    /// may, of a type parameter, which `$folded` asks at run time (#521), or
    /// asks its `nth`, as a `skip`'s and a `step_by`'s does (#182), or steps
    /// a lent inclusive range (#414), or asks an `nth` of items with a
    /// destructor, which a reversed `Vec` drops as a slice, so its
    /// `nth(n - 1)` and a step by `next`'s `nth(n)` drop them in another
    /// order (#494), or likewise an `nth_back`, a `take(n).rev()`'s of a
    /// `Vec`'s (#670), `$folded(it)`, its items as that gives them; else `it`.
    /// `first_by_next`, of a consumer that takes its first item by `next`
    /// and folds the rest, a `String` collected of `String`s, `$folded(it,
    /// true)` (#494).
    pub(in crate::lower) fn by_its_fold(&self, it: Expr, iter_ty: Ty<'tcx>, first_by_next: bool) -> Expr {
        let recognition = self.recognition();
        let drops = |source| {
            self.iterator_item(source)
                .is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing)
        };
        match recognition.reaches_its_own(iter_ty, OwnStep::Fold, Reach::May)
            || recognition.folds_by_its_own_nth(iter_ty)
            || self.steps_lent_inclusive(iter_ty)
            || recognition.nth_folded_sources(iter_ty).into_iter().any(drops)
            || recognition.nth_back_folded_sources(iter_ty).into_iter().any(drops)
        {
            true if first_by_next => Expr::call_helper("$folded", vec![it, Expr::bool(true)]),
            true => Expr::call_helper("$folded", vec![it]),
            false => it,
        }
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

    /// What drops a fold's accumulator, an `acc`, that a stage of an
    /// iterator of type `receiver` strands where it throws, a panic, as std's
    /// adapter's fold closure drops it (#423): its steps may panic, as its
    /// type says (`steps_may_panic`), or of a type parameter's, whose steps
    /// are anyone's (#424), and it has a destructor. `$foldDropping`'s and
    /// `$foldBack`'s `unwind`.
    pub(super) fn stranded_drop(&mut self, receiver: Ty<'tcx>, acc: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let may_throw = self.steps_may_panic(receiver);
        match may_throw && self.drops(acc) != crate::lower::drops::Drops::Nothing {
            true => self.drop_function(acc, span),
            false => Ok(None),
        }
    }

    /// `it.fold(init, f)`: `it.reduce(f, init)`, `f` given after what it
    /// starts from. JS's `reduce` counts its arguments: a start of
    /// `undefined`, a `fold(None, ..)`'s, is given, or it starts from the
    /// first item (ADR 0330).
    pub(super) fn folded(
        &mut self,
        (it, method): (Expr, DefId),
        (init, f): (Option<Expr>, Expr),
        (arg_tys, item): (&[Ty<'tcx>], Option<Ty<'tcx>>),
        span: Span,
    ) -> R<Expr> {
        let own = arg_tys.last().is_some_and(|ty| self.is_called_as_written(*ty));
        let ends = self.consumed_ends(self.recognition().iterator_method(method), arg_tys, &f, span)?;
        let callback = self.binary_callback(f, own);
        let acc = match init {
            Some(_) => arg_tys.get(1).copied(),
            None => item,
        };
        let unwind = match (acc, arg_tys.first()) {
            (Some(acc), Some(&receiver)) => self.stranded_drop(receiver, acc, span)?,
            _ => None,
        };
        let Some(init) = init else {
            // `reduce(f)`: from its first item, `None` of none (ADR 0479).
            let boxed = item.is_some_and(|item| self.boxed_payload(item));
            // Of a function it moves into its fold, `$consume`'s (#734).
            if let Some(ends) = ends {
                let unwind = unwind.unwrap_or_else(Expr::undefined);
                let args = vec![it, Expr::str("reduce"), Expr::bool(boxed), callback, unwind, ends];
                return Ok(Expr::call_helper("$consume", args));
            }
            let mut args = vec![it, callback];
            if boxed || unwind.is_some() {
                args.push(Expr::bool(boxed));
            }
            args.extend(unwind);
            return Ok(Expr::call_helper("$reduce", args));
        };
        let mut args = vec![callback, init];
        crate::lower::calls::given_to_js(&mut args);
        args.swap(0, 1);
        // What drops with `f`, its drop, `$consume(it, init, f, unwind,
        // ends)`, which drops it before what's left (#734).
        if let Some(ends) = ends {
            let unwind = unwind.unwrap_or_else(Expr::undefined);
            let step = vec![it, Expr::str("fold")];
            return Ok(Expr::call_helper("$consume", [step, args, vec![unwind, ends]].concat()));
        }
        // `$foldDropping(it, init, f, drop)`, which passes it on (#423).
        if let Some(unwind) = unwind {
            return Ok(Expr::call_helper(
                "$foldDropping",
                [vec![it], args, vec![unwind]].concat(),
            ));
        }
        args.swap(0, 1);
        Ok(Expr::call(Expr::member(it, "reduce"), args))
    }

    /// `it.for_each(f)`, `it.forEach(f)`; of a function it moves into its
    /// fold, `$consume`'s, a fold of `f` of each, which drops it before
    /// what's left (#734).
    pub(super) fn for_each_called(
        &mut self,
        it: Expr,
        (f, own): (Expr, bool),
        arg_tys: &[Ty<'tcx>],
        span: Span,
    ) -> R<Expr> {
        let Some(ends) = self.consumed_ends(Some(IterMethod::ForEach), arg_tys, &f, span)? else {
            let f = self.unary_callback(f, own);
            return Ok(Expr::call(Expr::member(it, "forEach"), vec![f]));
        };
        let each = Expr::arrow(
            vec!["_".into(), "x".into()],
            vec![StmtKind::Expr(Expr::call(f, vec![Expr::var("x")])).at(js::Span::NONE)],
        );
        let args = vec![it, Expr::str("fold"), Expr::undefined(), each, Expr::undefined(), ends];
        Ok(Expr::call_helper("$consume", args))
    }

    /// `it.is_sorted()`, `is_sorted_call`'s; `it.is_sorted_by(f)`,
    /// `$isSortedBy(it, f)`, given what drops with `f`, which std's `all`
    /// drops before what's left (#734); `it.is_sorted_by_key(f)`, std's
    /// `self.map(f).is_sorted()`, `$isSortedBy(it.map(f), ..)` of each
    /// key, the type `f` gives, by its `PartialOrd` (#763), given its drop
    /// where it has a destructor (#776).
    pub(super) fn sorted_called(
        &mut self,
        it: Expr,
        (method, arg, own): (IterMethod, &mut dyn FnMut() -> Expr, bool),
        item: Ty<'tcx>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
    ) -> R<Expr> {
        match method {
            IterMethod::IsSortedBy => {
                let f = arg();
                let ends = self.consumed_ends(Some(method), arg_tys, &f, span)?;
                let mut args = vec![it, self.binary_callback(f, own)];
                args.extend(ends);
                Ok(Expr::call_helper("$isSortedBy", args))
            }
            IterMethod::IsSortedByKey => {
                let key = self.key_of(arg_tys[1], item);
                // A key with a destructor, given its drop: each one
                // replaced dropped, and where two are out of order the later,
                // then the last, as std's `is_sorted_by` drops them (#776).
                let key_drop = self.std_drop_function(key, span)?;
                let keys = Expr::call(Expr::member(it, "map"), vec![self.unary_callback(arg(), own)]);
                self.is_sorted_call(keys, key, key_drop, span)
            }
            _ => self.is_sorted_call(it, item, None, span),
        }
    }

    /// `it.max()` or `it.min()`, `extreme_of`'s; `it.max_by(f)` and its
    /// kind, `method`, of `item`s, `$maxBy`'s, of a function its `reduce`
    /// moves into its fold given what drops with it, which drops before
    /// what's left (#734); of a `_key` form, its key the type `f` gives.
    pub(super) fn extreme_called(
        &mut self,
        it: Expr,
        (method, arg): (IterMethod, &mut dyn FnMut() -> Expr),
        item: Option<Ty<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
    ) -> R<Expr> {
        let tcx = self.tcx;
        let (max, keyed) = match method {
            IterMethod::Max | IterMethod::Min => return self.extreme_of(method == IterMethod::Max, it, item, span),
            IterMethod::MaxBy(max) => (max, false),
            IterMethod::MaxByKey(max) => (max, true),
            _ => unreachable!("only `max()` and its kind"),
        };
        let item = item.ok_or_else(|| self.unsupported(span, "this iterator's items"))?;
        // Of a `_key` form, the key its function gives an item's reference.
        let key = keyed.then(|| self.key_of(arg_tys[1], Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, item)));
        let f = arg();
        let ends = self.consumed_ends(Some(method), arg_tys, &f, span)?;
        // Of a key that drops, over the crate's iterator with a destructor:
        // closed, its drop the consumer's, which std's `Map` the key's
        // `reduce` folds drops before the key where its first step ends,
        // and after it where its fold does (`keyed_consumer_closes`, #789).
        let it = match self.keyed_consumer_closes(Some(method), arg_tys) {
            true => self.closing(it, arg_tys[0], span)?,
            false => it,
        };
        // Given as it is: its helper calls it with Rust's arguments alone,
        // `compare(kept, item)` or `key(item)`, as the recognizers' does.
        self.extreme_by_call((max, key), (it, f), item, ends, span)
    }

    /// What `f`, a key function, gives `given`: `<F as FnOnce<(given,)>>::Output`,
    /// of a closure, a function, a `Box<dyn FnMut>` or a `&mut F` alike, as
    /// std's `max_by_key<B, F: FnMut(&Self::Item) -> B>` names it (#1223).
    fn key_of(&self, f: Ty<'tcx>, given: Ty<'tcx>) -> Ty<'tcx> {
        let tcx = self.tcx;
        let output = tcx.require_lang_item(LangItem::FnOnceOutput, rustc_span::DUMMY_SP);
        let key = Ty::new_projection(tcx, ty::IsRigid::No, output, [f, Ty::new_tup(tcx, &[given])]);
        tcx.normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(key))
    }

    /// The drop of what a by-value consumer `method` of `arg_tys` moves into
    /// its fold, `f`, its last argument, as an arrow, its fold's `ends`,
    /// where `consumed_function` says (#734).
    pub(super) fn consumed_ends(
        &mut self,
        method: Option<IterMethod>,
        arg_tys: &[Ty<'tcx>],
        f: &Expr,
        span: Span,
    ) -> R<Option<Expr>> {
        let Some(&ty) = arg_tys.last() else {
            return Ok(None);
        };
        if !self.consumed_function(method, arg_tys) {
            return Ok(None);
        }
        let mut dropped = Vec::new();
        self.drop_value(f.clone(), ty, span, &mut dropped)?;
        Ok(Some(Expr::arrow(Vec::new(), dropped)))
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
        arg_tys: &[Ty<'tcx>],
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
        // Of an accumulator, a `B`, with a destructor that a stage's panic
        // may strand, its drop, as `fold`'s (#423), into any `Try`, an
        // `Option` or a `Result` too, whose helper drops what its last
        // `Some` or `Ok` held (rd767). Of `try_fold` and `try_rfold`,
        // `(&mut self, B, F)`.
        let unwind = match (arg_tys, method) {
            ([receiver, acc, _], IterMethod::TryFold | IterMethod::TryRFold) => {
                self.stranded_drop(*receiver, *acc, span)?
            }
            _ => None,
        };
        // What drops with `f`, where the fold says which of it and the
        // accumulator drops first, its `ends`, `$endsAfter`'s (#768).
        let ends = self.consumed_ends(Some(method), arg_tys, &f, span)?;
        let mut args = vec![it, init, self.binary_callback(f, own)];
        crate::lower::calls::given_to_js(&mut args);
        args.extend(tried);
        match ends {
            Some(ends) => args.extend([unwind.unwrap_or_else(Expr::undefined), ends]),
            None => args.extend(unwind),
        }
        Ok(Expr::call_helper(helper, args))
    }
}

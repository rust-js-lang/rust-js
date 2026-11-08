//! What ends a chain from MIR by its items: a sum, a collection made, a
//! position found, a fold.

use super::*;
use crate::collections::IndexSet;
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
        // What std steps of it, a type parameter's `IntoIter`, and its items,
        // `Vec::from_iter(iter)`'s: what a panic that stops it asks the
        // in-place facts of, and drops what it collected of (#800).
        let stepped = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, given);
        let item = self.iterator_item(stepped);
        Ok(Some(self.collected_into(it, output, item, Some(stepped), span)?))
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
                && (self.drops(iter_ty) != crate::lower::drops::Drops::Nothing || self.folds_held_function(iter_ty)))
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

    /// Whether std's `collect` into an `output` of `item`s takes its first
    /// item by `next`, then extends with the rest: a `String`'s of `String`s
    /// or of `Cow<str>`s; of `char`s, `&str`s and `Box<str>`s, it extends
    /// with them all.
    fn takes_first_by_next(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> bool {
        self.is_lang_adt(output, LangItem::String)
            && (self.is_lang_adt(item, LangItem::String) || self.recognition().is_cow(item))
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
    /// runs code, held first. Of a count read through a call, or of one
    /// that drops what it holds, the room its hint asks (#1044).
    /// `receiver`, the type `collect` is given, an inclusive range lent,
    /// `&mut r`, is what's collected as that, by its `fold` (#414).
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
        // A count no array bounds: not a `step_by(n)` of an array's items,
        // or a `skip`'s or a `map`'s of them, which no more than the array's
        // length can overflow nothing (#811).
        let of_items = |source: Ty<'tcx>| {
            source
                .walk()
                .any(|part| part.as_type().is_some_and(|t| self.steps_an_array(t)))
                && !source
                    .walk()
                    .any(|part| part.as_type().is_some_and(|t| self.range_kind(t).is_some()))
        };
        let given_count = |ty: Ty<'tcx>| {
            r::is_repeat_n(self.tcx, ty)
                || r::stepped_by_source(self.tcx, ty).is_some_and(|source| !of_items(source))
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
            && !self.compiled_struct(iter_ty, &it);
        let mut held = Vec::new();
        let count = match counted {
            true => Some(self.counted_in_place(&it, iter_ty, &mut held)),
            false => None,
        };
        // The room it asks of a count it reads, none where that surely fits.
        let room = match (&count, item) {
            (Some(Ok(len)), Some(item)) => self.room_size_of(len, item).map(|size| (len.clone(), size)),
            _ => None,
        };
        let drops = self.drops(iter_ty) != crate::lower::drops::Drops::Nothing;
        // Where it asks the room its hint asks instead: of a count it can't
        // read where it's made, one read through a call,
        // `repeat_n(x, *n.as_ref().unwrap())`'s, which reading again would
        // run again; of one it would hold part of `it` to read; and of one
        // that drops what it holds, which a panic asking it closes (#1044);
        // and of one it doesn't count, one MIR held in a local first, of a
        // value made before an `unwrap`'s check or under a guard, whose
        // hint, as each runtime form's is, is std's (#1046).
        let by_hint = match &count {
            Some(Ok(_)) => room.is_some(),
            Some(Err(Uncounted::RunsCode | Uncounted::Unknown)) => true,
            None => false,
        };
        let it = match room {
            // Of a count it reads, of one that drops nothing, as a panic
            // asking it doesn't close it: `$roomFor(n, size)`.
            Some((len, size)) if held.is_empty() && !drops => {
                let it = match it.has_effects() {
                    true => self.spill("it", it, made),
                    false => it,
                };
                self.room_for((None, len), size, span, made);
                it
            }
            // The room its hint asks, `$roomForHint`, as std's
            // `SpecFromIter` asks it, its hint std's, unless what it's
            // collected into asks it already, of one whose hint may run the
            // crate's own, `repeat(x).chain(own).take(n)`'s (`grown_ask`).
            // It's `it` itself, which `$roomForHint` returns, marked as it
            // is, so it's collected as one made in place still.
            _ if by_hint
                && item.is_some()
                && (self.grown_ask(output, item, iter_ty, self.compiled_struct(iter_ty, &it), span)?).is_none() =>
            {
                match self.grown_ask(output, item, iter_ty, true, span)? {
                    Some(grown) => {
                        let marks = crate::lower::iterators::held_marks(it.facts);
                        let mut asked = self.grown_by_hint(it, grown, iter_ty, None, span)?;
                        asked.facts = marks;
                        asked
                    }
                    None => it,
                }
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

    /// Whether std collects an iterator of `iter_ty` into a `Vec` of
    /// `item`s in place, where it's known here (`recognition::in_place_source`,
    /// `fits_in_place`), of a copied default in its impl's terms (ADR 0049):
    /// `None` where it's a caller's, or the adapters don't tell (#423, #672).
    fn collects_in_place(&self, iter_ty: Option<Ty<'tcx>>, item: Option<Ty<'tcx>>) -> Option<bool> {
        use crate::lower::recognition::{InPlaceSource, fits_in_place, in_place_source};
        let (iter_ty, item) = iter_ty.zip(item)?;
        let typing_env = self.typing_env;
        match in_place_source(self.tcx, typing_env, iter_ty)? {
            InPlaceSource::Never => Some(false),
            InPlaceSource::Items(src) => fits_in_place(self.tcx, typing_env, src, item),
            InPlaceSource::Of(_) => None,
        }
    }

    /// The `restFirst` of a collect into `output` of `item`s, of an iterator
    /// of `iter_ty`, where what's left of it is dropped before what it
    /// collected as a panic stops it, `None` where it isn't: of a hash map
    /// or set, or a `LinkedList`, which std extends as each comes, `true`;
    /// of another, which it collects as a `Vec`, where std doesn't collect
    /// it in place (`recognition::in_place_source`), `true` where that's
    /// known here (#423); or where its source is a type parameter's
    /// iterator, its caller's `IInPlace`, `!$collectsInPlace(IInPlace,
    /// size, align)` of the items' layout (#672).
    pub(in crate::lower) fn rest_dropped_first(
        &mut self,
        output: Ty<'tcx>,
        iter_ty: Option<Ty<'tcx>>,
        item: Ty<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        let first = Some(Expr::bool(true));
        if (self.is_map(output) && !self.is_sorted(output)) || self.is_std_type(output, StdItem::LinkedList) {
            return Ok(first);
        }
        let Some(iter_ty) = iter_ty else {
            return Ok(None);
        };
        match self.collects_in_place(Some(iter_ty), Some(item)) {
            Some(true) => return Ok(None),
            Some(false) => return Ok(first),
            None => {}
        }
        let in_place = self.caller_in_place(iter_ty, item, span)?;
        Ok(Some(Expr::unary(crate::js::UnaryOp::Not, in_place)))
    }

    /// Whether std collects an iterator of `iter_ty` into a `Vec` of
    /// `item`s in place where it's its caller's answer (`collects_in_place`'s
    /// `None`): `$collectsInPlace(IInPlace, size, align)`, its fact of what
    /// the adapters keep, or of a source's items' layout, `[TSize, TAlign]`,
    /// against the items' (#672). Refused where the adapters don't tell.
    fn caller_in_place(&mut self, iter_ty: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Expr> {
        use crate::lower::recognition::{InPlaceSource, TypeFact, in_place_source};
        let typing_env = self.typing_env;
        let source = match in_place_source(self.tcx, typing_env, iter_ty) {
            Some(InPlaceSource::Of(_) | InPlaceSource::Items(_)) => {
                self.type_fact_value(iter_ty, TypeFact::InPlace, span)?
            }
            _ => {
                return Err(self.unsupported(
                    span,
                    &format!("collecting a `{iter_ty}`, whose adapters don't say whether std collects it in place"),
                ));
            }
        };
        let size = self.type_fact_value(item, TypeFact::Size, span)?;
        let align = self.type_fact_value(item, TypeFact::Align, span)?;
        Ok(Expr::call_helper("$collectsInPlace", vec![source, size, align]))
    }

    /// Of a collect into `output` of `item`s a panic may stop, of an
    /// iterator of type `iter_ty` whose steps may panic, that holds what has
    /// a destructor or gives items with one: how what it holds is dropped
    /// as std's unwinds (#422, #423). `None` where it holds nothing a panic
    /// would leave.
    fn unwinding_collect(
        &mut self,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Option<UnwindingCollect>> {
        // Of an iterator whose steps may panic, as its type says: a stage of
        // its, a variable's too, or an iterator it holds whose steps no JS
        // shows, the crate's own or a `dyn`'s (#423, rd763, rd765); never
        // read from its JS (#653).
        if !iter_ty.is_some_and(|ty| self.steps_may_panic(ty)) {
            return Ok(None);
        }
        let drop = match item {
            Some(item) if self.drops(item) != crate::lower::drops::Drops::Nothing => self.drop_function(item, span)?,
            _ => None,
        };
        // Or of an iterator that holds what has a destructor, its type's
        // answer: its closing drops it as the panic leaves it, a
        // `$closing`'s or std's compiled struct's (#653).
        let holds = iter_ty.is_some_and(|ty| self.drops(ty) != crate::lower::drops::Drops::Nothing);
        if drop.is_none() && !holds {
            return Ok(None);
        }
        let rest_first = match (&drop, item) {
            (Some(_), Some(item)) => self.rest_dropped_first(output, iter_ty, item, span)?,
            _ => None,
        };
        // Into a `Vec` std collects in place: by its fold, whose sink is
        // what it collected, the accumulator `$kept` leaks as std's does
        // (#681); where that's its caller's answer, as it says (#672).
        let in_place = match (&drop, iter_ty, item) {
            (Some(_), Some(iter_ty), Some(item)) if self.is_std_type(output, StdItem::Vec) => {
                match self.collects_in_place(Some(iter_ty), Some(item)) {
                    Some(true) => InPlace::Yes,
                    Some(false) => InPlace::No,
                    None => InPlace::IfCaller(self.caller_in_place(iter_ty, item, span)?),
                }
            }
            _ => InPlace::No,
        };
        Ok(Some(UnwindingCollect {
            drop,
            rest_first,
            in_place,
        }))
    }

    /// Whether stepping an iterator of `iter_ty` may panic by what it runs,
    /// as its type shows where its JS is still a variable: a closure or a
    /// function of its type's, each but one of the crate's functions that
    /// never unwinds (ADR 0902) or a constructor (#423); or an iterator its
    /// type holds whose steps no JS shows, `unseen_steps_may_panic`'s; or
    /// what a std iterator does itself as it's stepped, a drop or a clone
    /// that may panic, `std_work_may_panic`'s.
    pub(in crate::lower) fn steps_may_panic(&self, iter_ty: Ty<'tcx>) -> bool {
        self.panicking_steps(iter_ty) > 0 || self.unseen_steps_may_panic(iter_ty) || self.std_work_may_panic(iter_ty)
    }

    /// Whether an iterator of std's that `iter_ty` holds, its revealed
    /// type's, does what may panic itself as it's stepped, as recognition's
    /// `stepping_work` catalogs it: drops items, of a `skip()`, a `filter()`
    /// or a `zip()`, or a part it has exhausted, a `chain()`'s side or a
    /// `flat_map()`'s inner iterator, whose drop may panic
    /// (`drop_may_panic`); or clones values, of a `cloned()` or a
    /// `cycle()`, whose clone may (`clone_may_panic`) (#707–#711).
    fn std_work_may_panic(&self, iter_ty: Ty<'tcx>) -> bool {
        use crate::lower::recognition::SteppingWork;
        (self.reveal(iter_ty).walk().filter_map(|part| part.as_type()))
            .flat_map(|part| self.recognition().stepping_work(part))
            .any(|work| match work {
                SteppingWork::DropsItemsOf(source) => {
                    self.iterator_item(source).is_some_and(|item| self.drop_may_panic(item))
                }
                SteppingWork::DropsExhausted(part) => self.drop_may_panic(part),
                SteppingWork::Clones(value) => self.clone_may_panic(value),
            })
    }

    /// Whether an iterator `iter_ty` holds, itself or a source std's
    /// adapters step, its revealed type's, may panic as it's stepped where
    /// no JS of its shows it: the crate's own, or a library's, whose impl
    /// of `Iterator` or `DoubleEndedIterator` writes a method that may
    /// unwind, any but the crate's that never do (ADR 0902), a `fuse()`'s
    /// `next` say, which runs its `next`; or a `dyn` one's or a type
    /// parameter's, anyone's (rd763, rd765, rd768).
    fn unseen_steps_may_panic(&self, iter_ty: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        let traits = [
            Some(std_item(self.tcx, StdItem::Iterator)),
            recognition.double_ended_iterator(),
        ];
        let may_unwind = |ty: Ty<'tcx>| {
            (traits.iter().flatten())
                .filter_map(|&tr| recognition.user_impl(ty::TraitRef::new(self.tcx, tr, [ty])))
                .any(|imp| self.impl_may_unwind(imp))
        };
        (self.reveal(iter_ty).walk().filter_map(|part| part.as_type())).any(|part| match part.kind() {
            // A type parameter's, whose steps are anyone's, as a `dyn`'s are.
            ty::Dynamic(..) | ty::Param(_) => true,
            ty::Adt(..) => recognition.is_user_iterator(part) && may_unwind(part),
            _ => false,
        })
    }

    /// Whether a method the impl `imp` writes may unwind: any but one of
    /// the crate's that never does (ADR 0902).
    pub(in crate::lower) fn impl_may_unwind(&self, imp: DefId) -> bool {
        self.tcx.associated_item_def_ids(imp).iter().any(|&method| {
            matches!(self.tcx.def_kind(method), rustc_hir::def::DefKind::AssocFn)
                && !self.krate.never_unwinding.contains(&method)
        })
    }

    /// How many of the closures and functions of `iter_ty`'s, its stages',
    /// may panic: each but one of the crate's that never unwinds (ADR 0902)
    /// or a constructor. Of its revealed type, so a stage an `impl FnMut`
    /// is, a closure a function made, is one too (rd767).
    fn panicking_steps(&self, iter_ty: Ty<'tcx>) -> usize {
        let mut seen = IndexSet::default();
        (self.reveal(iter_ty).walk().filter_map(|part| part.as_type()))
            .filter(|part| match *part.kind() {
                ty::Closure(def_id, _) | ty::FnDef(def_id, _) => seen.insert(def_id) && self.call_may_unwind(*part),
                _ => false,
            })
            .count()
    }

    /// Whether a call of `f`, a closure's or a function's type, may panic:
    /// any but one of the crate's that never unwinds (ADR 0902) or a
    /// constructor; of another, a `dyn Fn`'s or a type parameter's,
    /// anyone's. The one answer what a call holds across asks (#903).
    pub(in crate::lower) fn call_may_unwind(&self, f: Ty<'tcx>) -> bool {
        match *self.reveal(f).kind() {
            ty::Closure(def_id, _) | ty::FnDef(def_id, _) => {
                !self.krate.never_unwinding.contains(&def_id)
                    && !matches!(self.tcx.def_kind(def_id), rustc_hir::def::DefKind::Ctor(..))
            }
            _ => true,
        }
    }

    /// `$foldToArray`'s arguments, `it` folded into a `Vec` of `item`s:
    /// of items with a destructor, from a type whose steps may panic, as
    /// `steps_may_panic` says, their drop, which what it collected is
    /// dropped by as std's unwinds, and whether what's left is first (#423,
    /// #707).
    fn folded_unwinding(
        &mut self,
        it: Expr,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Vec<Expr>> {
        let drop = match item {
            Some(item) if self.steps_may_panic(iter_ty) && self.drops(item) != crate::lower::drops::Drops::Nothing => {
                self.drop_function(item, span)?
            }
            _ => None,
        };
        let mut args = vec![it];
        match (drop, item) {
            (Some(drop), Some(item)) => {
                args.push(drop);
                args.extend(self.rest_dropped_first(output, Some(iter_ty), item, span)?);
            }
            // Of items with nothing to drop, collected in place through a
            // function its fold would move, said so: std's `try_fold` by
            // `&mut`, which keeps it to drop after what's left (#710).
            _ if self.folds_held_function(iter_ty) && self.collects_in_place(Some(iter_ty), item) == Some(true) => {
                args.extend([Expr::undefined(), Expr::bool(false), Expr::bool(true)]);
            }
            _ => {}
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
        // What it's given, whose maker's marks say whether it's std's struct
        // (`compiled_struct`), before it's collected by its fold.
        let given = it.clone();
        let it = match iter_ty {
            Some(iter_ty) if self.collects_by_fold(output) => {
                let first_by_next = item.is_some_and(|item| self.takes_first_by_next(output, item));
                self.by_its_fold(it, iter_ty, first_by_next)
            }
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
            let from_iterator = std_item(self.tcx, StdItem::FromIterator);
            let from_iter = crate::lower::bindings::trait_method_name(self.tcx, from_iterator, "from_iter");
            // After the items, what its `from_iter::<I>` is given of its own
            // `I`, the iterator collected: the facts its declaration carries,
            // its `IntoIter`'s `InPlace` (`std_method_facts`, #794), then the
            // drops (ADR 0163), as a call through any dictionary gives them.
            let mut args = vec![it];
            if let Some(iter_ty) = iter_ty {
                let method = crate::lower::recognition::trait_method(self.tcx, from_iterator, "from_iter");
                let generic_args = self.tcx.mk_args(&[output.into(), item.into(), iter_ty.into()]);
                args.extend(self.own_evidence(method, generic_args, None, span)?);
            }
            return Ok(Expr::call(Expr::member(dictionary, from_iter), args));
        }
        let heap = self.is_std_type(output, StdItem::BinaryHeap);
        let made_of_vec = (heap || self.is_sorted(output)) && self.recognition().grows_as_vec(output);
        Ok(if made_of_vec {
            // A heap or a B-tree, which std's `from_iter` collects into a
            // `Vec` first, then makes of it, `BinaryHeap::from(vec)` or a
            // sorted `bulk_build`: that `Vec`'s collect, which asks its hint
            // as it grows, collects in place and unwinds as any does (#797).
            let grown = item.map_or(output, |item| self.vec_grown_for(output, item));
            let items = match iter_ty {
                Some(iter_ty) => self.collected_as_vec((it, &given), grown, item, iter_ty, !heap, span)?,
                None => it,
            };
            match heap {
                true => self.heap_values(HeapOp::From, vec![items], (output, false), span)?,
                false => self.made_of(
                    self.is_set(output),
                    self.tcx.mk_args(&[output.into()]),
                    items,
                    false,
                    span,
                )?,
            }
        // A `Vec`, or a `Box<[T]>`, an array too (ADR 0530).
        } else if self.is_vec_like(output) || output.boxed_ty().is_some_and(|inner| inner.is_slice()) {
            let Some(iter_ty) = iter_ty else { return Ok(it) };
            self.collected_as_vec((it, &given), output, item, iter_ty, false, span)?
        } else if self.is_lang_adt(output, LangItem::String) {
            // Of `char`s, its hint asked first, as std's `Extend<char>`
            // reserves by it (string.rs:2507); of strings, none.
            let chars = item.is_some_and(|item| item.peel_refs().is_char());
            let it = match chars {
                true => self.reserved_by_hint(it, iter_ty, crate::lower::iterators::Room::Bytes, None),
                false => it,
            };
            let items = match self.unwinding_collect(output, item, iter_ty, span)? {
                // By its fold, as std's `String::extend` steps it,
                // `for_each`'s: where a panic stops it, each adapter's fold
                // unwinds as std's does, a flatten's inner iterator dropped
                // before its source (#644).
                Some(unwinding) => unwinding.folded(it),
                None => items(it),
            };
            let items = match item {
                Some(item) => self.cow_texts(items, item),
                None => items,
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
            self.collect_fallible(it, output, item, iter_ty, span)?
        } else if self.is_map(output) {
            let set = self.is_set(output);
            // A panic that may stop it: serde_json's sorted `Map`'s,
            // collected as an array first; another's, which extends a new
            // one, `$mapFrom`, what's left then what it holds dropped
            // (#423). Of a hash table's, its hint asked first, as
            // hashbrown's `extend` reserves by it. A std B-tree is made of
            // the `Vec` it grows, above (#797).
            let it = match self.is_sorted(output) {
                true => it,
                false => self.reserved_by_hint(
                    it,
                    iter_ty,
                    item.map_or(
                        crate::lower::iterators::Room::Bytes,
                        crate::lower::iterators::Room::Entries,
                    ),
                    None,
                ),
            };
            let unwinding = self.unwinding_collect(output, item, iter_ty, span)?;
            let (it, unwinds) = match unwinding {
                Some(unwinding) if self.is_sorted(output) => (unwinding.collected(it, None), false),
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

    /// `collect()` of `it`, an iterator of type `iter_ty`, into a JS array
    /// of `item`s, as std grows a `Vec`, a `VecDeque` or a `Box<[T]>`, or
    /// what `output`, a `Vec` a heap or a B-tree is made of, is (#797); a
    /// `LinkedList`'s too, which asks no room. Where what it's made into
    /// takes any `iterable`, a B-tree's, `it` itself where nothing else is
    /// asked of it. `given` is what it was given, before it's collected by
    /// its fold, whose maker's marks say whether it's std's struct
    /// (`compiled_struct`).
    fn collected_as_vec(
        &mut self,
        (it, given): (Expr, &Expr),
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Ty<'tcx>,
        iterable: bool,
        span: Span,
    ) -> R<Expr> {
        // What it asks of its hint as std grows a `Vec` of it (`grown_ask`).
        let trusted = crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty);
        let grown = self.grown_ask(output, item, iter_ty, self.compiled_struct(iter_ty, given), span)?;
        // Whether it's `TrustedLen` where that's its caller's, a type
        // parameter's that folds by its `nth_back` (#670): as it says,
        // `ITrustedLen`, so its first item is taken by `next` before its
        // hint is asked only where it isn't (#724).
        // Of std's struct, or of the adapters made in its place where what
        // that `nth_back` passes has a destructor, `$rev($taking(..))`,
        // whose fold asks it as std's does: what it passes drops from the
        // back, where its `next`'s `nth_back` drops it as a `Vec`'s slice,
        // front to back (#826). Of numbers, the two are one.
        let folds_by_dropping_nth_back =
            (self.recognition().nth_back_folded_sources(iter_ty).into_iter()).any(|source| {
                self.iterator_item(source)
                    .is_some_and(|item| self.drops(item) != crate::lower::drops::Drops::Nothing)
            });
        let callers_trusted = (!trusted
            && iter_ty.has_param()
            && !self.recognition().nth_back_folded_sources(iter_ty).is_empty()
            && (self.compiled_struct(iter_ty, given) || folds_by_dropping_nth_back))
            .then(|| self.type_fact_value(iter_ty, crate::lower::recognition::TypeFact::TrustedLen, span))
            .transpose()?;
        let unroomed = it.clone();
        let it = match grown {
            Some(grown) => self.grown_by_hint(it, grown, iter_ty, callers_trusted.clone(), span)?,
            None => it,
        };
        // Of std's struct that's `TrustedLen`, by its `fold`, as std's
        // `extend_trusted` collects it, a zip's `SpecFold` (#380), and
        // of one stepping a lent inclusive range, which that leaves
        // exhausted at its last (#414).
        // And of one whose fold moves a function an adapter holds with
        // a destructor, which drops before what's left, where std doesn't
        // collect it in place, by `try_fold`, which doesn't move it (#710).
        // And of one whose fold asks an `nth_back` its `next` asks
        // otherwise, of items with a destructor, a `take(n).rev()`'s made
        // in place (#826).
        let folded = (self.compiled_struct(iter_ty, given)
            || folds_by_dropping_nth_back
            || self.steps_lent_inclusive(iter_ty)
            || self.folds_held_function(iter_ty) && self.collects_in_place(Some(iter_ty), item) == Some(false))
            && crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, iter_ty);
        // Of std's struct that folds by a type parameter's `nth_back`, a
        // `take(n).rev()`'s of one, which asks another than its `next`
        // does: whether it's `TrustedLen` is its caller's, `ITrustedLen`,
        // by its fold where it is, as `extend_trusted` collects it, else
        // stepped by `next`, as std's other `SpecFromIter` steps it (#670).
        if !folded && let Some(trusted) = callers_trusted {
            let args = self.folded_unwinding(it.clone(), output, item, iter_ty, span)?;
            let by_fold = Expr::call_helper("$foldToArray", args);
            let by_next = match self.unwinding_collect(output, item, Some(iter_ty), span)? {
                Some(unwinding) => unwinding.collected(it, Some(unroomed)),
                None => Expr::collected(it),
            };
            return Ok(Expr::cond(trusted, by_fold, by_next));
        }
        // By its fold, which drops what it holds as it unwinds itself
        // (`folded_unwinding`), first.
        // A range's `map`, an array's `map` (ADR 0063), only where a panic
        // that stops it has nothing to drop: one of items with a
        // destructor drops what it made as it unwinds (#800).
        let mapped = match self.mapped_range(&it, iter_ty, item) {
            Some(items) if self.unwinding_collect(output, item, Some(iter_ty), span)?.is_none() => Some(items),
            _ => None,
        };
        Ok(match mapped {
            Some(items) => items,
            None if folded && self.steps_lent_inclusive(iter_ty) => {
                let args = self.folded_unwinding(it, output, item, iter_ty, span)?;
                Expr::call_helper("$foldToArray", args)
            }
            None if folded && self.collected_items(&it).is_none() => {
                let args = self.folded_unwinding(it, output, item, iter_ty, span)?;
                Expr::call_helper("$foldToArray", args)
            }
            None if let Some(unwinding) = self.unwinding_collect(output, item, Some(iter_ty), span)? => {
                unwinding.collected(it, Some(unroomed))
            }
            None => match self.collected_items(&it) {
                Some(items) => items,
                None if iterable => it,
                None => Expr::collected(it),
            },
        })
    }

    /// The `Vec` of `item`s std's `from_iter` of `output`, a heap or a
    /// B-tree, collects first, in its allocator, `output`'s last argument.
    fn vec_grown_for(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> Ty<'tcx> {
        let ty::Adt(_, args) = output.kind() else { return output };
        let Some(alloc) = args.types().last() else {
            return output;
        };
        let vec = self.tcx.adt_def(std_item(self.tcx, StdItem::Vec));
        Ty::new_adt(self.tcx, vec, self.tcx.mk_args(&[item.into(), alloc.into()]))
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
                let tcx = self.tcx;
                let sig = match *arg_tys[1].kind() {
                    ty::Closure(_, args) => args.as_closure().sig(),
                    _ => arg_tys[1].fn_sig(tcx),
                };
                let key = tcx.instantiate_bound_regions_with_erased(sig).output();
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
        let (max, key) = match method {
            IterMethod::Max | IterMethod::Min => return self.extreme_of(method == IterMethod::Max, it, item, span),
            IterMethod::MaxBy(max) => (max, None),
            IterMethod::MaxByKey(max) => {
                let sig = match *arg_tys[1].kind() {
                    ty::Closure(_, args) => args.as_closure().sig(),
                    _ => arg_tys[1].fn_sig(tcx),
                };
                (max, Some(tcx.instantiate_bound_regions_with_erased(sig).output()))
            }
            _ => unreachable!("only `max()` and its kind"),
        };
        let item = item.ok_or_else(|| self.unsupported(span, "this iterator's items"))?;
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

/// How a collect a panic may stop drops what it holds: what it collected,
/// by `drop`, and what's left of what it steps, closed, that first where
/// `rest_first` (#422, #423); of one std collects in place, `in_place`,
/// what it collected as the fold's accumulator, which its `unwind` drops,
/// or leaks where std's does (#681).
struct UnwindingCollect {
    drop: Option<Expr>,
    rest_first: Option<Expr>,
    in_place: InPlace,
}

/// Whether std collects a collect into a `Vec` in place: known here, or as
/// its caller's answer, `$collectsInPlace(IInPlace, size, align)`, says
/// (#681, #672).
enum InPlace {
    No,
    Yes,
    IfCaller(Expr),
}

impl UnwindingCollect {
    /// `it` collected so, an array: `$collectDropping(it, drop)`; in place,
    /// `$foldToArray(it, drop)`, by its fold, as std's in-place collect is
    /// its `try_fold` into the source's buffer; where that's its caller's
    /// answer, as it says, `$collectsInPlace(..) ? $foldToArray(it, drop) :
    /// $collectDropping(it, drop, true)`, as `ITrustedLen` picks a fold
    /// (#672, #670). `$collectDropping` closes `it` at its end and as a
    /// panic unwinds, which it says (`Known::ClosesWhatItSteps`, #660), as
    /// `$roomForHint` closes it where asking its room throws. In place, of
    /// `it` before it's given room, `unroomed`, where it is: std's in-place
    /// collect writes into the source's buffer and asks no room, so what it
    /// gathered is the `InPlaceDrop` its `try_fold` holds, which a later
    /// drop's panic leaks, as rustc leaks what a closure gives back (#770).
    fn collected(self, it: Expr, unroomed: Option<Expr>) -> Expr {
        let mut args = vec![it];
        args.extend(self.drop);
        let in_place = |mut args: Vec<Expr>| {
            if let Some(unroomed) = unroomed {
                args[0] = unroomed;
            }
            Expr::call_helper("$foldToArray", args)
        };
        match self.in_place {
            InPlace::Yes => in_place(args),
            InPlace::IfCaller(collects_in_place) => {
                let by_fold = in_place(args.clone());
                args.push(Expr::bool(true));
                Expr::cond(collects_in_place, by_fold, Expr::call_helper("$collectDropping", args))
            }
            InPlace::No => {
                args.extend(self.rest_first);
                Expr::call_helper("$collectDropping", args).closes_what_it_steps()
            }
        }
    }

    /// `it` collected so by its fold, an array: `$foldToArray(it, drop)`.
    fn folded(self, it: Expr) -> Expr {
        let mut args = vec![it];
        args.extend(self.drop);
        args.extend(self.rest_first);
        Expr::call_helper("$foldToArray", args)
    }
}

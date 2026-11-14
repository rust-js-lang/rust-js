//! An iterator stepped: as a JS iterator or lent, its items given or
//! iterated, and stepped from its back.

use super::*;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Iterator.from(value)`: a JS iterator of an array or of one, which
    /// steps either (ADR 0061); one already, as it is.
    pub(in crate::lower) fn js_iterator(&self, value: Expr) -> Expr {
        if is_lent(&value) || is_js_iterator(&value) {
            return value;
        }
        Expr::call(Expr::member(Expr::global("Iterator"), "from"), vec![value])
    }

    /// `items`, each a `Result` or an `Option` (of `item`), collected into
    /// `target`, a `Result` or an `Option` of a collection: the first `Err`
    /// or `None`, which ends it, or `Ok` of all the values, or the values,
    /// an array, made the collection where it's another, `into` it. Of
    /// what's left of an iterator of `iter_ty` and what it collected, which
    /// a panic drops first, `rest_dropped_first`'s (#423).
    pub(in crate::lower) fn collect_fallible(
        &mut self,
        items: Expr,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        // Of a write's result, `undefined` or its error: std's code, which
        // reads each as a `Result`, given it tagged, `{ TAG }`, and a write's
        // result it collects into given back as one (`Form::Tagged`, #1410).
        let Some(crossing) = item.and_then(|item| self.std_reads_tagged(item)) else {
            return self.collect_fallible_items(items, target, item, iter_ty, span);
        };
        let r = self.fresh(Role::R);
        let tagged = self.form_given(crate::lower::generic_facts::Form::Tagged, crossing, Expr::var(&r), span)?;
        let f = Expr::arrow(vec![r.into()], vec![StmtKind::Return(Some(tagged)).at(js::Span::NONE)]);
        let items = Expr::builtin(items, "map", vec![f]);
        let collected = self.collect_fallible_items(items, target, item, iter_ty, span)?;
        let (std, declared, _) = crossing;
        Ok(self.form_back(
            crate::lower::generic_facts::Form::Tagged,
            (std, declared, target),
            collected,
        ))
    }

    /// `collect_fallible`'s, of items in the form std's code reads.
    fn collect_fallible_items(
        &mut self,
        items: Expr,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let ty::Adt(_, target_args) = target.kind() else {
            unreachable!("a `Result` or an `Option`")
        };
        let collection = target_args.type_at(0);
        // An `Option`'s `Some` of a collection that could look like `None`,
        // a `()`, is boxed (ADR 0051): what's collected made into it (#984).
        let boxed_some = matches!(self.option_of(target), Some(payload) if self.boxed_payload(payload));
        // Into a collection whose `from_iter` runs code, the crate's own or
        // a dictionary's: std's `try_process`, that `from_iter` given a
        // `GenericShunt` of the items, what it made dropped where a `None`
        // or an `Err` ended it (#1588).
        if let Some(iter_ty) = iter_ty
            && let Some((collection, value, shunt)) = self.try_processed_into(target, iter_ty)?
        {
            return self.try_processed(items, (target, collection, value), (item, shunt), span);
        }
        let into = match !boxed_some
            && (self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()))
        {
            true => None,
            false => {
                let values = self.fresh(Role::Values);
                // What each `Ok` or `Some` holds.
                let value = item.and_then(|item| match item.kind() {
                    ty::Adt(_, args) => args.types().next(),
                    _ => None,
                });
                let made = self.collected_into((Expr::var(&values), None), collection, value, None, span)?;
                let made = match boxed_some {
                    true => self.some(made),
                    false => made,
                };
                Some(match made.kind {
                    // A function that only takes them, a `from_iter`: the function.
                    js::ExprKind::Call(f, args) if matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(v) if *v == values)) => {
                        *f
                    }
                    kind => Expr::arrow(
                        vec![values.into()],
                        vec![StmtKind::Return(Some(Expr { kind, ..made })).at(js::Span::NONE)],
                    ),
                })
            }
        };
        // Where it stops, what's collected dropped, each value (ADR 0098).
        let value = item.and_then(|item| match item.kind() {
            ty::Adt(_, args) => args.types().next(),
            _ => None,
        });
        let drop = match value {
            Some(value) => self.std_drop_function(value, span)?,
            None => None,
        };
        let into = match (into, &drop) {
            (None, Some(_)) => Some(Expr::undefined()),
            (into, _) => into,
        };
        // Of values with a destructor alone, what drops them first (#423).
        let rest_first = match (&drop, value) {
            (Some(_), Some(value)) => self.rest_dropped_first(collection, iter_ty, value, span)?,
            _ => None,
        };
        // What it asks of its hint through std's `GenericShunt` (#848):
        // asked once first, or as the `Vec` it collects grows, its values'
        // size given last.
        let (items, size) = match self.shunted_ask(target, item, iter_ty, span)? {
            crate::lower::iterators::Shunted::Never => (items, None),
            crate::lower::iterators::Shunted::Reserved => (
                self.reserved_by_hint(items, iter_ty, crate::lower::iterators::Room::Nothing, None),
                None,
            ),
            crate::lower::iterators::Shunted::Grown(size) => (items, Some(size)),
        };
        // Its arguments past `items`, each before one given given, as
        // `undefined` of an optional one, `false` of a flag.
        let given = |optional: Vec<(Option<Expr>, Expr)>| {
            let last = optional.iter().rposition(|(given, _)| given.is_some());
            (optional.into_iter().enumerate())
                .take_while(|&(at, _)| last.is_some_and(|last| at <= last))
                .map(|(_, (given, unset))| given.unwrap_or(unset))
                .collect::<Vec<_>>()
        };
        Ok(if self.option_of(target).is_some() {
            let boxed = item.is_some_and(|item| self.option_of(item).is_some_and(|inner| self.boxed_payload(inner)));
            let mut list = vec![items];
            list.extend(given(vec![
                (boxed.then(|| Expr::bool(true)), Expr::bool(false)),
                (into, Expr::undefined()),
                (drop, Expr::undefined()),
                (rest_first, Expr::bool(false)),
                (size, Expr::undefined()),
            ]));
            Expr::call(Expr::helper("$collectOptions"), list)
        } else {
            let mut list = vec![items];
            list.extend(given(vec![
                (into, Expr::undefined()),
                (drop, Expr::undefined()),
                (rest_first, Expr::bool(false)),
                (size, Expr::undefined()),
            ]));
            Expr::call(Expr::helper("$collectResults"), list)
        })
    }

    /// `collect_fallible_items`' of a collection whose `from_iter` runs
    /// code, `$tryProcessOptions(items, collect, drop, boxed, someBoxed)` or
    /// `$tryProcessResults(items, collect, drop, dropErr)`: std's
    /// `try_process`, which gives that `from_iter`,
    /// `collect`, its `GenericShunt` of `items`, then drops what it made by
    /// `drop` where a `None` or an `Err` ended it, and the `Err` it ended at
    /// by `dropErr` where `from_iter` panics (#1588).
    fn try_processed(
        &mut self,
        items: Expr,
        (target, collection, value): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        (item, shunt): (Option<Ty<'tcx>>, Ty<'tcx>),
        span: Span,
    ) -> R<Expr> {
        let given = self.fresh(Role::Iter);
        let made = self.collected_into((Expr::var(&given), None), collection, Some(value), Some(shunt), span)?;
        let collect = match made.kind {
            js::ExprKind::Call(f, args) if matches!(args.as_slice(), [arg] if matches!(&arg.kind, js::ExprKind::Var(v) if *v == given)) => {
                *f
            }
            kind => Expr::arrow(
                vec![given.into()],
                vec![StmtKind::Return(Some(Expr { kind, ..made })).at(js::Span::NONE)],
            ),
        };
        let result = self.recognition().is_result(target);
        let drop = self.std_drop_function(collection, span)?;
        let drop_err = match (result, target.kind()) {
            (true, ty::Adt(_, args)) => self.std_drop_function(args.type_at(1), span)?,
            _ => None,
        };
        let boxed = item.is_some_and(|item| self.option_of(item).is_some_and(|inner| self.boxed_payload(inner)));
        let some_boxed = matches!(self.option_of(target), Some(payload) if self.boxed_payload(payload));
        let mut args = vec![items, collect];
        let optional = match result {
            true => vec![(drop, Expr::undefined()), (drop_err, Expr::undefined())],
            false => vec![
                (drop, Expr::undefined()),
                (boxed.then(|| Expr::bool(true)), Expr::bool(false)),
                (some_boxed.then(|| Expr::bool(true)), Expr::bool(false)),
            ],
        };
        let last = optional.iter().rposition(|(given, _)| given.is_some());
        args.extend(
            (optional.into_iter().enumerate())
                .take_while(|&(at, _)| last.is_some_and(|last| at <= last))
                .map(|(_, (given, unset))| given.unwrap_or(unset)),
        );
        Ok(Expr::call_helper(
            match result {
                true => "$tryProcessResults",
                false => "$tryProcessOptions",
            },
            args,
        ))
    }

    /// `$lent(it)`: a generic iterator lent as a `&mut`, which steps `it` and
    /// can't close it, as its lender goes on stepping it (ADR 0071); of one
    /// already lent, `by_ref()`'s, itself, a JS iterator nothing closes.
    pub(in crate::lower) fn lent_iterator(&mut self, it: Expr) -> Expr {
        if is_lent(&it) {
            return it;
        }
        IterValue::js(it).lent().emit()
    }

    /// `$iter(items)`: a JS iterator of an array that knows where it is
    /// (ADR 0071).
    pub(in crate::lower) fn stepped_items(&mut self, items: Expr) -> Expr {
        array_stepped(items)
    }

    /// `$iter(items)` of std's iterator of type `ty` over them; of what std
    /// hints no lower bound of, which JS counts, `$iter(items, true)`, whose
    /// hint is std's, where the crate keeps capacities, which std grows by
    /// it (`hints_no_lower_bound`, #1215).
    pub(in crate::lower) fn stepped_items_of(&mut self, items: Expr, ty: ty::Ty<'tcx>) -> Expr {
        let mut it = array_stepped(items);
        if self.krate.keeps_capacity
            && self.recognition().hints_no_lower_bound(ty)
            && let js::ExprKind::Call(_, args) = &mut it.kind
        {
            args.push(Expr::bool(true));
        }
        it
    }

    /// `$iterator(value, next)`: a JS iterator of `value`, a range, stepped
    /// by `next`, its `$rangeNext`.
    pub(in crate::lower) fn stepped_by(&mut self, value: Expr, next: Expr) -> Expr {
        stepped_range_by(value, next)
    }

    /// What's left of `it`, a `$iter` of std's iterator over an array, of
    /// type `ty`: `it.items.slice(it.at, it.end)`, copies of the items it
    /// `owns` that need one (ADR 0181). Of one just made, `$iter(items)`,
    /// nothing's stepped: `items`, which Rust lends it or it owns.
    pub(in crate::lower) fn stepped_left(&mut self, it: Expr, ty: ty::Ty<'tcx>, owns: bool, span: Span) -> R<Expr> {
        let left = match fresh_iter_source(&it) {
            Some(items) => items.clone(),
            None => Expr::call(
                Expr::member(Expr::member(it.clone(), "items"), "slice"),
                vec![Expr::member(it.clone(), "at"), Expr::member(it, "end")],
            ),
        };
        let item = self.iterator_item(ty).expect("an iterator's item");
        Ok(match owns && self.needs_clone(item) {
            true => {
                let clone = self.clone_fn("item", item, span)?;
                Expr::builtin(left, "map", vec![clone])
            }
            false => left,
        })
    }

    /// Is a `ty` given for `callee`'s type parameter `input` one whose JS
    /// value isn't an iterator, where one goes: one of the crate's own, or a
    /// range, an object (ADR 0129), where `input` is an `Iterator` or an
    /// `IntoIterator`. A range is itself anywhere else.
    pub(in crate::lower) fn given_as_iterator(&self, callee: DefId, input: ty::Ty<'tcx>, ty: ty::Ty<'tcx>) -> bool {
        // As it is, with its dictionary (ADR 0411), or as its trait's impl
        // takes it.
        if let ty::Param(param) = input.kind()
            && (given_as_it_is(self.tcx, callee, *param) || given_by_its_trait(self.tcx, callee, param.index))
        {
            return false;
        }
        // The crate's own, a range, or a parameter of this function's given
        // as it is, made a JS iterator by its dictionary (ADR 0551), where
        // the callee bounds it so; given for one bound by neither, `T:
        // Debug`'s, its own value, which that one's dictionaries read
        // (#991).
        if !self.is_user_iterator(ty)
            && self.range_kind(ty.peel_refs()).is_none()
            && !self.iterator_given_as_it_is(ty.peel_refs())
        {
            return false;
        }
        // Bound so where the callee says, as `declared_bounds` reads it.
        let env = ty::TypingEnv::post_analysis(self.tcx, callee);
        (self.declared_bounds(input, env.param_env).into_iter())
            .any(|id| std_item(self.tcx, StdItem::Iterator) == id || std_item(self.tcx, StdItem::IntoIterator) == id)
    }

    /// What std's `IntoIterator` of `ty`, a `&C` or a `&mut C`, gives of
    /// `value` in JS (ADR 0457): a handle on each item of a `&mut`, as `for x
    /// in &mut v` makes, or std's own of what it's given; `None` of what std
    /// has none of here. The one answer, of a dictionary generic code is
    /// given and of what a caller gives it.
    pub(in crate::lower) fn ref_into_iter(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let ty::Ref(_, pointee, mutability) = *ty.kind() else {
            return Ok(None);
        };
        let into_iter_trait = std_item(self.tcx, StdItem::IntoIterator);
        let args = self.tcx.mk_args(&[ty.into()]);
        let into_iter = crate::lower::recognition::trait_method(self.tcx, into_iter_trait, "into_iter");
        // Of a `&mut` to a sequence, a handle on each item, the type's to
        // say, not the recogniser's: what generic code steps is adapted by
        // what it's given, so a call no recogniser answers still gives the
        // handles (#1477).
        let sequence = |ty: ty::Ty<'tcx>| {
            let ty = ty.boxed_ty().unwrap_or(ty);
            ty.is_array() || ty.is_slice() || self.is_vec_like(ty)
        };
        let known = match self.recognition().classify(into_iter, args) {
            Some(known) => known,
            None if mutability == ty::Mutability::Mut && sequence(pointee) => Std::Same,
            None => return Ok(None),
        };
        let into_iter_ty = crate::lower::recognition::trait_method(self.tcx, into_iter_trait, "IntoIter");
        let output = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter_ty, args);
        let output = (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
            .unwrap_or(output);
        let handles = match mutability {
            ty::Mutability::Mut => self.item_handles_of(known, vec![value.clone()], pointee, span, out)?,
            ty::Mutability::Not => None,
        };
        let items = match handles {
            Some(handles) => handles,
            None => self.std_by_values(
                known,
                into_iter,
                args,
                (&[ty], Default::default()),
                vec![value],
                output,
                span,
                out,
            )?,
        };
        Ok(Some(self.std_iterator(items, output)))
    }

    /// Whether `callee` takes `input`, a type parameter of its own, by an
    /// `IntoIterator` bound it's given no dictionary of, as its `bounds`
    /// say: iterated as JS has it.
    fn given_iterable(&self, callee: DefId, input: ty::Ty<'tcx>) -> bool {
        if !matches!(input.kind(), ty::Param(_)) {
            return false;
        }
        let env = ty::TypingEnv::post_analysis(self.tcx, callee);
        let into_iter = std_item(self.tcx, StdItem::IntoIterator);
        let given_one = crate::lower::traits::bounds(self.tcx, self.krate.foreign, self.krate.given, callee)
            .iter()
            .any(|tr| tr.def_id == into_iter && tr.self_ty() == input);
        (self.declared_bounds(input, env.param_env).into_iter()).any(|id| id == into_iter) && !given_one
    }

    /// The arguments of a call of the crate's `callee`, of types `given`:
    /// each iterator of the crate's, or range, where a generic iterator goes
    /// a JS iterator (ADR 0061), a function's and a trait impl's method's
    /// alike.
    pub(in crate::lower) fn given_iterators(
        &mut self,
        callee: DefId,
        values: &mut [Expr],
        given: &[ty::Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        for ((value, &input), &given) in values.iter_mut().zip(inputs).zip(given) {
            // Through a reference, what it's to; through a `Box<I>`, its `I`,
            // given as the box of it is, which steps it by what std's `Box`
            // passes on (`iterator_held`, #1601).
            let (input, given) = match *input.kind() {
                ty::Ref(_, inner, _) => (inner, given.peel_refs()),
                _ if let Some(inner) = input.boxed_ty() => (inner, given),
                _ => (input, given),
            };
            let made_knowing = self.made_knowing_into_iter(callee, input, given);
            // One that asks its length, a JS iterator that knows it, its back
            // too where it steps it from there (ADR 0456).
            if self.exact_size_param(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                let back = self.double_ended_param(callee, input);
                *value = self.as_exact_size(held, given, back, span)?;
            // One that steps it from its back, a JS iterator that knows it
            // (ADR 0429).
            } else if self.double_ended_param(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_double_ended(held, given, span)?;
            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = match made_knowing {
                    Some((back, len)) => self.as_knowing(held, given, (back, len), span)?,
                    None => self.as_js_iterator(held, given, span)?,
                };
            // One an `IntoIterator` bound takes, given the JS iterator of
            // its items (ADR 0061): a `&mut` collection's handles, an
            // `Option`'s, a B-tree's in order, `iterable_items`'.
            } else if matches!(input.kind(), ty::Param(_)) && self.given_iterable(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = match made_knowing {
                    Some((back, len)) => self.as_knowing(held, given, (back, len), span)?,
                    None => self.iterable_items(held, given, span, out)?,
                };
            }
            // A `Vec`'s, given its steps that drop what they pass as one
            // slice, where generic code can't tell it from a `Drain`'s
            // (`slicing`, #703, #704); one given as it is is told by its
            // dictionary (`iterator_dictionary`).
            if let ty::Param(param) = input.kind()
                && !given_as_it_is(self.tcx, callee, *param)
            {
                let back = self.double_ended_param(callee, input);
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.slicing(held, given, back, span)?;
            }
        }
        Ok(())
    }

    /// Of the crate's iterator `given` for `callee`'s input `input`, a type
    /// parameter `T: IntoIterator` whose `T::IntoIter` a bound says is
    /// stepped from its back or asked its length, as generic code's JS
    /// iterator of it is (ADR 0480): whether it's made knowing its back and
    /// its length, as an `I` so bound is (#1604). Another's items know them
    /// as they're made. What `given_iterators` makes of the value, and what
    /// the call's dictionaries of it are (`made_into_iter_of`).
    fn made_knowing_into_iter(&self, callee: DefId, input: ty::Ty<'tcx>, given: ty::Ty<'tcx>) -> Option<(bool, bool)> {
        // A type parameter's alone: what `callee` declares of a concrete
        // type's `IntoIter` is none (`extension_param`).
        if !self.is_user_iterator(given) {
            return None;
        }
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let into_iter = crate::lower::recognition::trait_method(self.tcx, into_iterator, "IntoIter");
        let made = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter, [input]);
        let (back, len) = (
            self.double_ended_param(callee, made),
            self.exact_size_param(callee, made),
        );
        (back || len).then_some((back, len))
    }

    /// Of a call of `callee` given `args`, the crate's iterator a bound of
    /// `T::IntoIter`, `bounded`, is of, where `given_iterators` made it the
    /// JS iterator generic code holds: `T` an input of `callee`'s, as that
    /// steps them, given the crate's iterator made knowing.
    pub(in crate::lower) fn made_into_iter_of(
        &self,
        callee: DefId,
        args: ty::GenericArgsRef<'tcx>,
        bounded: ty::Ty<'tcx>,
    ) -> Option<ty::Ty<'tcx>> {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let into_iter = crate::lower::recognition::trait_method(self.tcx, into_iterator, "IntoIter");
        let ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Projection { def_id },
                args: of,
                ..
            },
        ) = *bounded.kind()
        else {
            return None;
        };
        let input = (def_id == into_iter).then(|| of.type_at(0))?;
        let inputs = self.tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        let taken = |declared: &ty::Ty<'tcx>| match *declared.kind() {
            ty::Ref(_, inner, _) => inner == input,
            _ => declared.boxed_ty().unwrap_or(*declared) == input,
        };
        if !inputs.iter().any(taken) {
            return None;
        }
        let given = ty::EarlyBinder::bind(self.tcx, input)
            .instantiate(self.tcx, args)
            .skip_normalization();
        let given = self
            .tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(given))
            .unwrap_or(given);
        self.made_knowing_into_iter(callee, input, given).map(|_| given)
    }

    /// `held`, an iterator of type `given`, made a JS iterator that knows its
    /// back, `back`, and its length, `len`, where it's asked them.
    fn as_knowing(&mut self, held: Expr, given: ty::Ty<'tcx>, (back, len): (bool, bool), span: Span) -> R<Expr> {
        match len {
            true => self.as_exact_size(held, given, back, span),
            false => self.as_double_ended(held, given, span),
        }
    }

    /// Whether what `callee` holds for its drop `index`, of the type `ty`
    /// given for it, is a JS iterator its caller made of an iterator of the
    /// crate's, `$iterator(value, next)`, whose drop drops `value`, its `of`
    /// (ADR 0421): its type parameter's, or that one's `IntoIter`, the
    /// iterator itself.
    pub(in crate::lower) fn iterated_slot(
        &self,
        callee: DefId,
        index: u32,
        args: ty::GenericArgsRef<'tcx>,
        ty: ty::Ty<'tcx>,
    ) -> bool {
        let slot = crate::lower::traits::drop_slot(self.tcx, self.slots(), callee, index);
        let param = match *slot.kind() {
            ty::Param(param) => param,
            ty::Alias(_, alias) => match *alias.self_ty().kind() {
                ty::Param(param) => param,
                _ => return false,
            },
            _ => return false,
        };
        let given = args.type_at(param.index as usize);
        // The crate's own, `$iterator`'s, or a type parameter's, `$given`'s
        // (#766).
        // Of the same type, its regions apart: a closure's in it is erased
        // in one and not the other (#1239).
        (self.is_user_iterator(given) || matches!(given.kind(), ty::Param(_)))
            && self.tcx.erase_and_anonymize_regions(ty) == self.tcx.erase_and_anonymize_regions(given)
            && self.given_as_iterator(callee, ty::Ty::new_param(self.tcx, param.index, param.name), given)
    }

    /// The items of `value`, an `IntoIterator` of type `ty` a function takes
    /// as it is, `extend(xs)`'s, `chain(xs)`'s or a `Debug` builder's
    /// `entries(xs)`'s, as JS steps them: an
    /// `Option`'s, `[]` or `[x]` (ADR 0128); a range's, the crate's own
    /// iterator's or a type parameter's given as it is, its JS iterator; a
    /// B-tree's, in order; an array's or a JS iterator's, itself. The one
    /// answer (ADR 0528).
    pub(in crate::lower) fn iterable_items(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // An associated type's, `I::Item`'s, its dictionary's `into_iter`
        // (#278).
        if let Some(items) = self.items_by_dictionary(value.clone(), ty, span)? {
            return Ok(items);
        }
        // A `&mut` range is stepped where it is, as it's taken from.
        if let ty::Ref(_, inner, ty::Mutability::Mut) = ty.kind()
            && self.range_kind(*inner).is_some()
        {
            return self.as_js_iterator(value, ty, span);
        }
        // A `&mut` collection's, a handle on each item, as `for x in &mut v`
        // makes (`ref_into_iter`); a type parameter's, an iterator its
        // dictionary steps, `items_by_dictionary`'s above, or none of
        // `ref_into_iter`'s.
        if let ty::Ref(_, inner, ty::Mutability::Mut) = ty.kind()
            && self.option_of(*inner).is_none()
            && !self.is_sorted(*inner)
            && let Some(items) = self.ref_into_iter(value.clone(), ty, span, out)?
        {
            return Ok(items);
        }
        let by_mut = matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut));
        let ty = ty.peel_refs();
        if let Some(item) = self.option_of(ty) {
            return Ok(self.option_items(value, item, out));
        }
        // A `Result`'s, its `Ok` value, as its `into_iter()` gives it (ADR
        // 0923); of a `&mut` to one, a handle on it, `iter_mut()`'s, the
        // type's to say where no recogniser answered above (#1477).
        if self.recognition().is_std_type(ty, StdItem::Result) {
            let none = ty::List::empty();
            let comb = match by_mut {
                true => crate::lower::combinators::Comb::ResultIterMut,
                false => crate::lower::combinators::Comb::ResultIter,
            };
            return self.comb_values(comb, (vec![value], &[ty]), false, none, span, out);
        }
        if self.range_kind(ty).is_some() || self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty) {
            return self.as_js_iterator(value, ty, span);
        }
        // A B-tree's, in its keys' order: a JS `Set` a caller gives may be in
        // any (ADR 0404, amended).
        if self.is_sorted(ty) {
            return self.in_order_of(value, ty, span);
        }
        Ok(value)
    }

    /// The JS iterator of `value`'s items, of `ty`, an associated type or a
    /// type parameter whose `IntoIterator` this function holds as it is,
    /// `I::Item`'s (ADR 0552, #278), `impl Is<I>`'s `I` (#966): its
    /// dictionary's `into_iter`, stepped by the `Iterator` that carries,
    /// `$given(IItemIntoIterator.intoIter(item), ..)`.
    pub(in crate::lower) fn items_by_dictionary(
        &mut self,
        value: Expr,
        ty: ty::Ty<'tcx>,
        span: Span,
    ) -> R<Option<Expr>> {
        // Or a reference whose `IntoIterator` is a bound of this function's,
        // `&'a I: IntoIterator`, which no caller makes a JS iterator of, as
        // it does of a type parameter it gives: its dictionary's
        // `into_iter`, the JS iterator it makes, where `evidence_for` finds
        // one (strsim's `jaro`, #1264).
        let given = match ty.kind() {
            ty::Alias(..) | ty::Param(_) => self.holds_into_iter(ty),
            ty::Ref(..) => true,
            _ => false,
        };
        if !given {
            return Ok(None);
        }
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let Some(dictionary) = self.evidence_for(ty::TraitRef::new(self.tcx, into_iterator, [ty])) else {
            return Ok(None);
        };
        let into_iter = crate::lower::bindings::trait_method_name(self.tcx, into_iterator, "into_iter");
        let made = Expr::call(Expr::member(dictionary, into_iter), vec![value]);
        let into_iter_ty = crate::lower::recognition::trait_method(self.tcx, into_iterator, "IntoIter");
        let output = ty::Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter_ty, [ty]);
        self.as_js_iterator(made, output, span).map(Some)
    }

    /// The crate's iterator `ty`'s own `fold`, `nth` and `last`, where its
    /// impl writes them: the functions what std routes through them calls,
    /// `$iterator`'s (#162, #182), and its `rfold` and `nth_back` (#220).
    /// Stepped from its back, `back`, as `rev()` steps it, its `rfold` and
    /// its `nth_back`, std's `Rev`'s `fold` and `nth`, and std's `last`, a
    /// fold (#217). Through a `&mut`, std passes `nth` and `nth_back` on,
    /// but its folds are the iterator's `try_fold` and `try_rfold`, and its
    /// `last` its fold's (#216). One that takes evidence of its impl's
    /// parameters besides its drops, the sides' dictionaries of either's
    /// `Either<L, R>`, is a function of what the runtime gives that gives
    /// it those of `ty` (#475); of its own, which no caller would give the
    /// same, refused.
    pub(in crate::lower) fn own_methods(
        &mut self,
        ty: ty::Ty<'tcx>,
        back: bool,
        span: Span,
    ) -> R<crate::lower::iterators::OwnMethods> {
        let folds = !ty.is_ref();
        Ok(match back {
            true => crate::lower::iterators::OwnMethods {
                fold: self.own_fold(ty, ("rfold", folds), span)?,
                nth: self.own_method(ty, ("nth_back", true), span)?,
                ..Default::default()
            },
            // Its back's too, which std's adapters stepped from their back
            // ask (#220).
            false => crate::lower::iterators::OwnMethods {
                fold: self.own_fold(ty, ("fold", folds), span)?,
                nth: self.own_method(ty, ("nth", false), span)?,
                last: match folds {
                    true => self.own_method(ty, ("last", false), span)?,
                    false => None,
                },
                rfold: self.own_fold(ty, ("rfold", folds), span)?,
                nth_back: self.own_method(ty, ("nth_back", true), span)?,
                count: match folds {
                    true => self.own_method(ty, ("count", false), span)?,
                    false => None,
                },
            },
        })
    }

    /// The impl's `name`, of `Iterator`, or of `DoubleEndedIterator`,
    /// `of_back`, where it writes one: `own_methods`'.
    fn own_method_id(&self, ty: ty::Ty<'tcx>, (name, of_back): (&str, bool), span: Span) -> R<Option<DefId>> {
        let written = match of_back {
            true => self.recognition().own_back_method(ty, name),
            false => self.recognition().own_iterator_method(ty, name),
        };
        let Some(written) = written else {
            return Ok(None);
        };
        // Given no drops: what std's consumers fold, through the
        // runtime, is a JS value whose type it can't name (#162).
        if self.takes_of_its_own(written) {
            return Err(self.unsupported(
                span,
                &format!("an iterator's own `{name}` given a dictionary of its own type parameter"),
            ));
        }
        Ok(Some(written))
    }

    /// `own_method_id`'s, the function itself; one that takes its impl's
    /// evidence, a function of the iterator and `n`, `nth`'s and
    /// `nth_back`'s, that gives it those of `ty` (`user_method`, #475).
    fn own_method(&mut self, ty: ty::Ty<'tcx>, (name, of_back): (&str, bool), span: Span) -> R<Option<Expr>> {
        let Some(written) = self.own_method_id(ty, (name, of_back), span)? else {
            return Ok(None);
        };
        if !self.takes_more_than_drops(written) {
            return Ok(Some(self.fn_ref(written)));
        }
        let trait_id = match of_back {
            true => (self.recognition().double_ended_iterator()).expect("a `DoubleEndedIterator`'s method"),
            false => std_item(self.tcx, StdItem::Iterator),
        };
        let params: &[&str] = match name {
            "last" | "count" => &[],
            _ => &["n"],
        };
        Ok(Some(self.user_method(ty, trait_id, name, params, span)?.0))
    }

    /// The crate's iterator `ty`'s own `fold` or `rfold`, `name`, where
    /// `folds`: what it takes after its function is what the runtime's
    /// fold gives it, `unwind`, its accumulator's drop (#423), its `B`'s.
    /// The function itself where that's its first drop; else, its impl's
    /// type parameters' drops before it, or its impl's evidence, its sides'
    /// dictionaries of an `Either<L, R>` (#475), `fold_entry`'s, given those
    /// as `ty` has them.
    fn own_fold(&mut self, ty: ty::Ty<'tcx>, (name, folds): (&str, bool), span: Span) -> R<Option<Expr>> {
        if !folds {
            return Ok(None);
        }
        let Some(written) = self.own_method_id(ty, (name, name == "rfold"), span)? else {
            return Ok(None);
        };
        let acc = self.tcx.generics_of(written).parent_count as u32;
        let before = (self.krate.drop_params.get(&written)).is_some_and(|drops| drops.iter().any(|&index| index < acc));
        let trait_id = self
            .tcx
            .trait_of_assoc(self.tcx.trait_item_of(written).unwrap_or(written));
        let given = self.takes_more_than_drops(written);
        let fold = trait_id.and_then(|trait_id| self.fold_of(ty, (trait_id, name)));
        match (before || given, fold) {
            (true, Some(found)) => self.fold_entry(found, span).map(Some),
            (true, None) if given => {
                let what = format!("an iterator's own `{name}` given a dictionary");
                Err(self.unsupported(span, &what))
            }
            _ => Ok(Some(self.fn_ref(written))),
        }
    }

    /// Whether `ty` is an iterator of the crate's that's double-ended, by
    /// its own `next_back`, which `user_iterator_knowing` gives it.
    pub(in crate::lower) fn user_double_ended(&self, ty: ty::Ty<'tcx>) -> bool {
        self.own_next_back(ty).is_ok_and(|back| back.is_some())
    }

    /// Whether `ty` is an iterator of the crate's that's an
    /// `ExactSizeIterator`, its length its hint's.
    pub(in crate::lower) fn user_exact_size(&self, ty: ty::Ty<'tcx>) -> bool {
        self.is_user_iterator(ty)
            && (self.recognition().exact_size_iterator())
                .is_some_and(|exact| self.has_user_impl(exact, crate::lower::recognition::iterator_held(ty)))
    }

    /// `DoubleEndedIterator`, if `ty` is an iterator of the crate's whose
    /// `next_back` is its own: `rev()` of it is a JS iterator of that,
    /// `$iterator(it, spanDoubleEndedIterator_next_back)`, stepped from the
    /// back as Rust's is (ADR 0164).
    pub(in crate::lower) fn own_next_back(&self, ty: ty::Ty<'tcx>) -> R<Option<DefId>> {
        if !self.is_user_iterator(ty) {
            return Ok(None);
        }
        let Some(double_ended) = self.recognition().double_ended_iterator() else {
            return Ok(None);
        };
        let next_back = trait_method(self.tcx, double_ended, "next_back");
        let args = self.args_of(double_ended, crate::lower::recognition::iterator_held(ty));
        let own = self
            .resolve_instance(next_back, args)?
            .is_some_and(|i| self.is_rust_fn(i.def_id()));
        Ok(own.then_some(double_ended))
    }

    /// Whether an iterator of type `ty` is cloned apart (ADR 0525): an
    /// adapter made of two parts alone, a `Map` or a `Filter` of its source
    /// and function, a `Chain` or a `Zip` of its two sources (#181), the
    /// crate clones, made by a function of them that its clone calls again
    /// with a clone of each, `$cloning`'s, as a closure cloned apart is (ADR
    /// 0522). The one answer, of what's made and what's cloned, of one made
    /// in place: a `Map` held is std's struct (ADR 0871), whose clone is
    /// its compiled `Clone` (`clone_value`'s).
    /// Not a copy that's its source, of items whose copy is themselves
    /// (`is_its_source`), numbers, as it's made: a copy cloned apart is one
    /// made of a map of each item's copy, a struct's or generic code's
    /// (#1181).
    pub(in crate::lower) fn iterator_cloned_apart(&self, ty: ty::Ty<'tcx>) -> bool {
        cloned_apart(
            (self.tcx, self.recognition().typing_env),
            self.krate.cloned,
            ty,
            self.frame.owner,
        ) && !self.is_its_source(ty)
    }

    /// `value` of type `ty`, a `step_by(n)` made here, `src.filter((_, i) =>
    /// i % n === 0)` or by its source's `nth`, `$stepBy(src, n)`, a range's
    /// (#231), or `filter` of what drops each it passes over (#718):
    /// `$stepByEnds` of its source made knowing its back and its
    /// length, which it finds its last and counts by, as std's `StepBy`
    /// does, double-ended and exact-sized where its source is exact-sized.
    /// Of a `Vec`'s items with a destructor, `$closing` of their `$iter`,
    /// each it passes over dropped as std's `nth_back` of its source drops
    /// it (#718). `None` of another.
    pub(super) fn step_by_knowing(&mut self, value: &Expr, ty: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some(source) = crate::lower::recognition::stepped_by_source(self.tcx, ty) else {
            return Ok(None);
        };
        // One made so already, itself.
        if crate::lower::iterators::stepped_by_ends_parts(value).is_some() {
            return Ok(Some(value.clone()));
        }
        let Some((source_value, step)) = stepped_by_parts(value) else {
            return Ok(None);
        };
        let source = source.peel_refs();
        let source_value = crate::lower::iterators::stepped_value(&source_value).unwrap_or(source_value);
        let source_value = self.as_exact_size(source_value, source, true, span)?;
        // Of items with a destructor, what drops each it passes over.
        let drop = match self.iterator_item(ty) {
            Some(item) => self.std_drop_function(item, span)?,
            None => None,
        };
        Ok(Some(crate::lower::iterators::stepped_by_ends(source_value, step, drop)))
    }
}

/// What `value`, a `step_by(n)` made here, steps and by how many: of
/// `src.filter((_, i) => i % n === 0)`, of what drops each it passes over
/// too (`every_nth`), `$stepBy(src, n)` by its source's `nth`
/// (`stepped_every_parts`), or one made knowing its back, `$stepByEnds(src,
/// n)` (`stepped_by_ends_parts`), a `rev()`'s source: `src` and `n`. The one
/// reader of each form, `step_by_knowing`'s and `len_in_place`'s (#718,
/// #811).
pub(in crate::lower) fn stepped_by_parts(value: &Expr) -> Option<(Expr, Expr)> {
    if let Some((source_value, step)) = crate::lower::iterators::stepped_by_ends_parts(value) {
        return Some((source_value.clone(), step.clone()));
    }
    match (&value.kind, crate::lower::iterators::stepped_every_parts(value)) {
        (_, Some([source_value, step, ..])) => Some((source_value.clone(), step.clone())),
        (js::ExprKind::Call(callee, args), None)
            if let js::ExprKind::Member(source_value, method) = &callee.kind
                && method == "filter"
                && let [f] = args.as_slice() =>
        {
            crate::lower::std_types::range::every_nth(f).map(|step| ((**source_value).clone(), step))
        }
        _ => None,
    }
}

/// `iterator_cloned_apart`'s, of `ty`, revealed, made in the body whose
/// typing environment `typing_env` is, and the types the crate clones,
/// `cloned`: the one answer, of what the lowering makes, of what the
/// analysis marks made knowing its back by its maker (#1002), and of what
/// a body makes in place or holds to clone (`body_queries`, #1243). A cloned type
/// that generic code's `ty` can't be, its bounds say, isn't one of it: a
/// `Map<I, F>` of an `I: DoubleEndedIterator<Item = u32>` is none of
/// another function's cloned `Map`s of `slice::Iter<'_, i32>` (#955). Of
/// `owner`'s own type parameters, each instance its callers give it, where
/// all are seen (`Cloned::instances_of`, #1119).
pub(in crate::lower) fn cloned_apart<'tcx>(
    (tcx, typing_env): (ty::TyCtxt<'tcx>, ty::TypingEnv<'tcx>),
    cloned: &crate::lower::analysis::Cloned<'tcx>,
    ty: ty::Ty<'tcx>,
    owner: rustc_span::def_id::DefId,
) -> bool {
    // A closure as generic code makes it, which stands for each its
    // callers' instances of it.
    let generic_closures = |ty: ty::Ty<'tcx>| {
        rustc_middle::ty::TypeFoldable::fold_with(
            ty,
            &mut ty::BottomUpFolder {
                tcx,
                ty_op: |t: ty::Ty<'tcx>| match *t.kind() {
                    ty::Closure(id, _) => tcx.type_of(id).instantiate_identity().skip_normalization(),
                    _ => t,
                },
                lt_op: |r| r,
                ct_op: |c| c,
            },
        )
    };
    let adapter = |ty| crate::lower::recognition::cloned_parts(tcx, ty).is_some();
    // And, of one cloned as it is of another function's own type
    // parameters, any type its callers may give them, a `pub` function's:
    // within that function's bounds, by which a `Map<I, F>` of an `I:
    // Iterator<Item = i32>` is no `Map` of another's items (#1318).
    let within_its_owners = |cloned_ty: ty::Ty<'tcx>, ty: ty::Ty<'tcx>| {
        let mut owners = cloned.owners_of(cloned_ty).peekable();
        owners.peek().is_none()
            || owners.any(|owner| {
                crate::lower::copies::instance_within_bounds(
                    tcx,
                    ty::TypingEnv::post_analysis(tcx, owner),
                    generic_closures(ty),
                    generic_closures(cloned_ty),
                )
            })
    };
    let cloned_as = |ty: ty::Ty<'tcx>| {
        cloned.iter().any(|&cloned_ty| {
            adapter(cloned_ty)
                && crate::lower::copies::instance_within_bounds(
                    tcx,
                    typing_env,
                    generic_closures(cloned_ty),
                    generic_closures(ty),
                )
                && within_its_owners(cloned_ty, ty)
        })
    };
    // Of generic code's own type parameters, as each instance its callers
    // give it, so another generic function's adapter of its own
    // parameters, which rustc names alike, isn't taken for it (#1119).
    adapter(ty) && cloned.as_any_instance(tcx, ty, owner, cloned_as)
}

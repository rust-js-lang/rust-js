//! An iterator stepped: as a JS iterator or lent, its items given or
//! iterated, and stepped from its back.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Iterator.from(value)`: a JS iterator of an array or of one, which
    /// steps either (ADR 0061); one already, as it is.
    pub(in crate::lower) fn js_iterator(&self, value: Expr) -> Expr {
        if is_lent(&value) || is_js_iterator(&value) {
            return value;
        }
        Expr::call(Expr::member(Expr::var("Iterator"), "from"), vec![value])
    }

    /// `items`, each a `Result` or an `Option` (of `item`), collected into
    /// `target`, a `Result` or an `Option` of a collection: the first `Err`
    /// or `None`, which ends it, or `Ok` of all the values, or the values,
    /// an array, made the collection where it's another, `into` it.
    pub(in crate::lower) fn collect_fallible(
        &mut self,
        items: Expr,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Expr> {
        let ty::Adt(_, target_args) = target.kind() else {
            unreachable!("a `Result` or an `Option`")
        };
        let collection = target_args.type_at(0);
        let into = match self.is_vec_like(collection) || collection.boxed_ty().is_some_and(|t| t.is_slice()) {
            true => None,
            false => {
                let values = self.fresh("values");
                // What each `Ok` or `Some` holds.
                let value = item.and_then(|item| match item.kind() {
                    ty::Adt(_, args) => args.types().next(),
                    _ => None,
                });
                let made = self.collected_into(Expr::var(&values), collection, value, None, span)?;
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
        Ok(if self.option_of(target).is_some() {
            let mut list = vec![items];
            let boxed = item.is_some_and(|item| self.option_of(item).is_some_and(|inner| self.boxed_payload(inner)));
            if boxed || into.is_some() {
                list.push(Expr::bool(boxed));
            }
            list.extend(into);
            list.extend(drop);
            Expr::call(Expr::helper("$collectOptions"), list)
        } else {
            let mut list = vec![items];
            list.extend(into);
            list.extend(drop);
            Expr::call(Expr::helper("$collectResults"), list)
        })
    }

    /// `$lent(it)`: a generic iterator lent as a `&mut`, which steps `it` and
    /// can't close it, as its lender goes on stepping it (ADR 0071); of one
    /// already lent, `by_ref()`'s, itself, a JS iterator nothing closes.
    pub(in crate::lower) fn lent_iterator(&mut self, it: Expr) -> Expr {
        if is_lent(&it) {
            return it;
        }
        Expr::call(Expr::helper("$lent"), vec![it])
    }

    /// `$iter(items)`: a JS iterator of an array that knows where it is
    /// (ADR 0071).
    pub(in crate::lower) fn stepped_items(&mut self, items: Expr) -> Expr {
        array_stepped(items)
    }

    /// `$iterator(value, next)`: a JS iterator of `value`, stepped by `next`,
    /// a range's by its `$rangeNext`.
    pub(in crate::lower) fn stepped_by(&mut self, value: Expr, next: Expr) -> Expr {
        stepped(value, next, false, None)
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
                Expr::call(Expr::member(left, "map"), vec![clone])
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
        let ty = self.reveal(ty);
        if self.is_user_iterator(ty) {
            return true;
        }
        // A range, or a parameter of this function's given as it is, made a
        // JS iterator by its dictionary (ADR 0551), where the callee bounds
        // it so.
        if self.range_kind(ty.peel_refs()).is_none() && !self.iterator_given_as_it_is(ty.peel_refs()) {
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
        let Some(known) = self.recognition().classify(into_iter, args) else {
            return Ok(None);
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
            None => self.std_by_values(known, into_iter, args, (&[ty], &[]), vec![value], output, span, out)?,
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
            let (input, given) = match *input.kind() {
                ty::Ref(_, inner, _) => (inner, given.peel_refs()),
                _ => (input, given),
            };
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
            // One given as it is to a std body compiled, `rev()`'s `self`,
            // that steps it from its back by its dictionary: of std's
            // adapter, a JS iterator, whose `next_back` is `$nextBack`,
            // made knowing its back where it's given (ADR 0871).
            } else if crate::lower::recognition::is_compiled_std(self.tcx, callee)
                && matches!(input.kind(), ty::Param(param) if crate::lower::iterators::given_as_it_is(self.tcx, callee, *param))
                && self.steps_back_by_dictionary(callee, input)
                && !crate::lower::recognition::known_by_bounds(given)
                && !self.is_user_iterator(given)
                && self.range_kind(given).is_none()
            {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_double_ended(held, given, span)?;
            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_js_iterator(held, given, span)?;
            // One an `IntoIterator` bound takes, given the JS iterator of
            // its items (ADR 0061): a `&mut` collection's handles, an
            // `Option`'s, a B-tree's in order, `iterable_items`'.
            } else if matches!(input.kind(), ty::Param(_)) && self.given_iterable(callee, input) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.iterable_items(held, given, span, out)?;
            }
        }
        Ok(())
    }

    /// Whether `callee` bounds its type parameter `input` by
    /// `DoubleEndedIterator`, which it's given as it is, by its dictionary.
    fn steps_back_by_dictionary(&self, callee: DefId, input: ty::Ty<'tcx>) -> bool {
        let env = ty::TypingEnv::post_analysis(self.tcx, callee);
        (self.declared_bounds(input, env.param_env).into_iter())
            .any(|bound| crate::lower::recognition::is_double_ended(self.tcx, bound))
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
        let slot = crate::lower::traits::drop_slot(self.tcx, self.krate.foreign, callee, index);
        let param = match *slot.kind() {
            ty::Param(param) => param,
            ty::Alias(_, alias) => match *alias.self_ty().kind() {
                ty::Param(param) => param,
                _ => return false,
            },
            _ => return false,
        };
        let given = args.type_at(param.index as usize);
        self.is_user_iterator(self.reveal(given))
            && self.reveal(ty) == self.reveal(given)
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
        let ty = self.reveal(ty);
        // A `&mut` range is stepped where it is, as it's taken from.
        if let ty::Ref(_, inner, ty::Mutability::Mut) = ty.kind()
            && self.range_kind(*inner).is_some()
        {
            return self.as_js_iterator(value, ty, span);
        }
        // A `&mut` collection's, a handle on each item, as `for x in &mut v`
        // makes (`ref_into_iter`); not a type parameter's, an iterator its
        // dictionary steps.
        if let ty::Ref(_, inner, ty::Mutability::Mut) = ty.kind()
            && !matches!(inner.kind(), ty::Param(_))
            && self.option_of(*inner).is_none()
            && !self.is_sorted(*inner)
            && let Some(items) = self.ref_into_iter(value.clone(), ty, span, out)?
        {
            return Ok(items);
        }
        let ty = ty.peel_refs();
        if let Some(item) = self.option_of(ty) {
            return Ok(self.option_items(value, item, out));
        }
        if self.range_kind(ty).is_some() || self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty) {
            return self.as_js_iterator(value, ty, span);
        }
        // A B-tree's, in its keys' order, which its JS map isn't (ADR 0059).
        if self.is_sorted(ty) {
            return self.in_order_of(value, ty, span);
        }
        Ok(value)
    }

    /// The crate's iterator `ty`'s own `fold`, `nth` and `last`, where its
    /// impl writes them: the functions what std routes through them calls,
    /// `$iterator`'s (#162, #182), and its `rfold` and `nth_back` (#220).
    /// Stepped from its back, `back`, as `rev()` steps it, its `rfold` and
    /// its `nth_back`, std's `Rev`'s `fold` and `nth`, and std's `last`, a
    /// fold (#217). Through a `&mut`, std passes `nth` and `nth_back` on,
    /// but its folds are the iterator's `try_fold` and `try_rfold`, and its
    /// `last` its fold's (#216). One that takes evidence besides its
    /// drops, a bound's dictionary, can't be given what each caller would:
    /// refused.
    pub(in crate::lower) fn own_methods(
        &self,
        ty: ty::Ty<'tcx>,
        back: bool,
        span: Span,
    ) -> R<crate::lower::iterators::OwnMethods> {
        let recognition = self.recognition();
        // The impl's `name`, of `Iterator`, or of `DoubleEndedIterator`,
        // `of_back`, where it writes one.
        let own = |name: &str, of_back: bool| -> R<Option<Expr>> {
            let written = match of_back {
                true => recognition.own_back_method(ty, name),
                false => recognition.own_iterator_method(ty, name),
            };
            let Some(written) = written else {
                return Ok(None);
            };
            // Given no drops: what std's consumers fold, through the
            // runtime, is a JS value whose type it can't name (#162).
            if self.takes_more_than_drops(written) {
                return Err(self.unsupported(span, &format!("an iterator's own `{name}` given a dictionary")));
            }
            Ok(Some(self.fn_ref(written)))
        };
        let folds = !ty.is_ref();
        let folded = |name: &str, of_back: bool| if folds { own(name, of_back) } else { Ok(None) };
        Ok(match back {
            true => crate::lower::iterators::OwnMethods {
                fold: folded("rfold", true)?,
                nth: own("nth_back", true)?,
                ..Default::default()
            },
            // Its back's too, which std's adapters stepped from their back
            // ask (#220).
            false => crate::lower::iterators::OwnMethods {
                fold: folded("fold", false)?,
                nth: own("nth", false)?,
                last: folded("last", false)?,
                rfold: folded("rfold", true)?,
                nth_back: own("nth_back", true)?,
            },
        })
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
            && (self.recognition().exact_size_iterator()).is_some_and(|exact| self.has_user_impl(exact, ty.peel_refs()))
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
        let args = self.args_of(double_ended, ty.peel_refs());
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
    /// 0522). The one answer, of what's made and what's cloned.
    pub(in crate::lower) fn iterator_cloned_apart(&self, ty: ty::Ty<'tcx>) -> bool {
        // What an `impl Iterator` hides, a function's result.
        let ty = self.reveal(ty);
        // A closure as generic code makes it, which stands for each its
        // callers' instances of it.
        let tcx = self.tcx;
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
        adapter(ty)
            && (self.krate.cloned.iter())
                .any(|&cloned| adapter(cloned) && self.instance_of(generic_closures(cloned), generic_closures(ty)))
    }
}

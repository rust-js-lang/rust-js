//! An iterator stepped: as a JS iterator or lent, its items given or
//! iterated, and stepped from its back.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `Iterator.from(value)`: a JS iterator of an array or of one, which
    /// steps either (ADR 0061).
    pub(in crate::lower) fn js_iterator(&self, value: Expr) -> Expr {
        if is_lent(&value) {
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
            } else if matches!(input.kind(), ty::Param(_)) && self.given_as_iterator(callee, input, given) {
                let held = std::mem::replace(value, Expr::undefined());
                *value = self.as_js_iterator(held, given, span)?;
            }
        }
        Ok(())
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
        // A `&mut` range is stepped through as it's taken from.
        if let ty::Ref(_, inner, _) = ty.kind()
            && self.range_kind(*inner).is_some()
        {
            return Err(self.unsupported(span, &format!("iterating over a `{ty}`, which steps it")));
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
    /// adapter that holds its source and a function alone, a `Map` or a
    /// `Filter`, the crate clones, made by a function of them that its clone
    /// calls again with a clone of each, `$cloning`'s, as a closure cloned
    /// apart is (ADR 0522). The one answer, of what's made and what's cloned.
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
        let adapter = |ty| crate::lower::recognition::source_and_function(tcx, ty).is_some();
        adapter(ty)
            && (self.krate.cloned.iter())
                .any(|&cloned| adapter(cloned) && self.instance_of(generic_closures(cloned), generic_closures(ty)))
    }
}

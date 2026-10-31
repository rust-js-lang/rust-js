//! An impl's dictionary lowered: its entries, its supertraits' and its
//! bounds', and a trait's default body copied into it.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A dictionary's `method`, which a std default given as another entry
    /// is, a writer's `write_char` its `write_str` or an error's `cause` its
    /// `source`, named once where it's more than a name: `{ write_str,
    /// write_char: write_str, .. }`.
    fn shared_entry(&mut self, props: &mut [Prop], shared: &mut Vec<js::Stmt>, method: &str, span: Span) -> R<Expr> {
        let value = props.iter().find_map(|prop| match prop {
            Prop::Field(name, value) if name == method => Some(value.clone()),
            _ => None,
        });
        let value = value.ok_or_else(|| self.unsupported(span, &format!("a dictionary without its `{method}`")))?;
        if value.is_named() {
            return Ok(value);
        }
        let name = self.fresh(method);
        shared.push(StmtKind::Const(name.clone(), value).at(js::Span::NONE));
        for prop in props.iter_mut() {
            if let Prop::Field(field, value) = prop
                && field == method
            {
                *value = Expr::var(&name);
            }
        }
        Ok(Expr::var(&name))
    }

    /// What recognition knows of a call of the trait method `def_id`, of
    /// `generic_args`: nothing of `into_iter` of a `&C` or a `&mut C`, which
    /// its bound's dictionary gives (ADR 0457), in its impl's terms where
    /// it's a copied default. Nothing but a bound makes one `IntoIterator`:
    /// a `&mut I` of an `I: Iterator` is an iterator, stepped as it is.
    pub(in crate::lower) fn known_trait_call(
        &self,
        def_id: DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Std> {
        // Nor of one whose `IntoIter` is held as it is, its dictionary's
        // (ADR 0552).
        if let Some(trait_id) = self.tcx.trait_of_assoc(def_id)
            && let self_ty = self.in_impl_terms(generic_args).type_at(0)
            && (into_iter_by_dictionary(self.tcx, trait_id, self_ty)
                || (is_std_def(self.tcx, trait_id, StdItem::IntoIterator) && self.holds_into_iter(self_ty)))
        {
            return None;
        }
        self.recognition().classify(def_id, generic_args)
    }

    /// Whether generic code holds a value of `self_ty` in a box: a `&mut` to
    /// an object, given for a type parameter (ADR 0454), `for x in c` of a `c:
    /// &mut C`'s (ADR 0457). A `&C` is the value, and a `&mut` to a number
    /// or text its cell.
    pub(super) fn held_in_box(&self, self_ty: Ty<'tcx>) -> bool {
        matches!(*self_ty.kind(), ty::Ref(_, pointee, Mutability::Mut) if !self.is_cell_pointee(pointee))
    }

    /// The entries of `tr`'s dictionary that give its supertraits', of those
    /// that have one, each made where it's asked: `Display: () =>
    /// innerDisplay()` of an `Error`'s.
    pub(super) fn supertrait_entries(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let found = held_supertraits(self.tcx, self.krate.foreign, tr.def_id, tr.args);
        self.entries_of(found, span)
    }

    /// The entries of `tr`'s dictionary that give the bounds its trait
    /// declares on an associated type, the impl's: `LabelDisplay` of `type
    /// Label = u32` is `u32`'s `Display` (ADR 0106).
    pub(super) fn item_bound_entries(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        self.entries_of(item_bounds(self.tcx, tr.def_id, tr.args), span)
    }

    /// An entry giving each of `bounds`' dictionaries, of those that have one.
    fn entries_of(&mut self, bounds: Vec<(String, ty::TraitRef<'tcx>)>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (name, bound) in bounds {
            if operational(self.tcx, self.krate.foreign, bound.def_id) {
                let dictionary = self.dictionary(bound, span)?;
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
                ));
            }
        }
        Ok(props)
    }

    /// An impl's dictionary: given nothing, the function whose one `return`
    /// is its module's `const` (ADR 0592); given its type parameters' and
    /// consts', a function of them, and its cache's value, made where its
    /// module is: a `WeakMap` of what it's given, or a `Map` of numbers.
    pub(in crate::lower) fn lower_dictionary(&mut self, id: DefId, cache: &str) -> R<(js::Function, Option<Expr>)> {
        let span = self.tcx.def_span(id);
        // An impl of a generic trait, `Convert<f64>`, or of a std one,
        // `PartialEq<Rhs>`, is for its arguments: a dictionary of its own.
        let tr = self.tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        let params = self.evidence_params(id);
        // What the dictionary's entries share, named once before it's made.
        let mut shared = Vec::new();
        let mut props = self.supertrait_entries(tr, span)?;
        props.extend(self.item_bound_entries(tr, span)?);
        for item in self.tcx.associated_items(tr.def_id).in_definition_order() {
            // A constant, the impl's or the trait's default, as rustc computed
            // it: read on each use where it's of a type changed in place, so
            // each is a value of its own, as ADR 0031's are.
            if matches!(self.tcx.def_kind(item.def_id), DefKind::AssocConst { .. }) {
                // Only one generic code reads, here or where a library's consumers
                // may (ADR 0100).
                if !self.krate.library && !self.krate.generic_consts.contains(&item.def_id) {
                    continue;
                }
                let own = self.tcx.impl_item_implementor_ids(id).get(&item.def_id).copied();
                // A library's of a trait's default isn't computed: a consumer
                // may never read it, and rustc reports one it can't compute,
                // `360 / Self::SIDES` of 0, only where it's read (ADR 0176).
                let evaluated = match (self.krate.library, own) {
                    (true, None) => None,
                    _ => eval_const(self.tcx, self.typing_env, item.def_id, tr.args, span),
                };
                let Some(value) = evaluated.and_then(|value| const_js(self.tcx, value)) else {
                    // Of its parameters, `Wrapping(T::ZERO)`: its initializer, read
                    // each time, as a constant is, of the impl's evidence (ADR 0176);
                    // a trait's default's, of the dictionary's.
                    let getter = match own {
                        // A function's, a constant of its module (ADR 0410).
                        Some(own) if crate::lower::analysis::impl_const(self.tcx, own) => Expr::arrow(
                            Vec::new(),
                            vec![StmtKind::Return(Some(self.fn_ref(own))).at(js::Span::NONE)],
                        ),
                        Some(own) => self.impl_const_getter(own, span)?,
                        // The trait's default, copied into the impl as a default
                        // method is, `Self` the impl's type (ADR 0049).
                        None => self.default_method(item.def_id, tr.args)?,
                    };
                    let name = bindings::fn_name(self.tcx, item.def_id);
                    // A literal, `"shape"`, is the value itself: nothing to compute.
                    let literal = match &getter.kind {
                        js::ExprKind::Arrow(params, body) if params.is_empty() => match body.as_slice() {
                            [
                                js::Stmt {
                                    kind: StmtKind::Return(Some(value)),
                                    ..
                                },
                            ] if value.is_constant() => Some(value.clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    props.push(match literal {
                        Some(value) => Prop::Field(name, value),
                        None => Prop::Getter(name, getter),
                    });
                    continue;
                };
                let ty = self
                    .tcx
                    .type_of(item.def_id)
                    .instantiate(self.tcx, tr.args)
                    .skip_normalization();
                let ty = self
                    .tcx
                    .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty));
                let name = bindings::fn_name(self.tcx, item.def_id);
                props.push(match self.contains_mutated(ty) {
                    true => Prop::Getter(
                        name,
                        Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(value)).at(js::Span::NONE)]),
                    ),
                    false => Prop::Field(name, value),
                });
                continue;
            }
            // A binding, which JS reads of the value itself, isn't in it
            // (ADR 0307).
            if self.tcx.def_kind(item.def_id) != DefKind::AssocFn || bindings::is_binding(self.tcx, item.def_id) {
                continue;
            }
            // A std trait's provided methods, like `Clone::clone_from`,
            // aren't in its dictionary: nothing calls them through it. But a
            // `dyn Error` calls `source` (ADR 0141), generic code writing
            // to any writer `write_char` and `write_fmt` (ADR 0180), and a
            // generic `clone_from_slice` an impl's own `clone_from`, as std's
            // does, which it asks the dictionary for.
            let source = self.is_error_source(item.def_id);
            let own_clone_from = is_clone_from(self.tcx, item.def_id) && self.recognition().has_clone_from(id);
            if !self.is_rust_trait(tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !in_std_dictionary(self.tcx, item.def_id)
                && !own_clone_from
            {
                continue;
            }
            // A `Hasher`'s provided method its impl doesn't write: `$hasher`'s,
            // std's over the dictionary's own (ADR 0463).
            if is_hasher(self.tcx, tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !self.tcx.impl_item_implementor_ids(id).contains_key(&item.def_id)
            {
                continue;
            }
            // The method's own parameters: lifetimes, `fn bar<'b>`, are erased,
            // as they aren't in the JS, but rustc resolves with them; a type,
            // `describe<T>`, is its caller's, the trait method's own (ADR 0106).
            let args = tr.args.extend_to(self.tcx, item.def_id, |param, _| match param.kind {
                ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
                _ => self.tcx.mk_param_from_def(param),
            });
            // One whose `where` clauses this impl's type can't meet, a
            // default's `where Self::Item: Deref` of an `Item` that isn't,
            // nothing can call: it isn't in it, as rustc leaves it out of a
            // vtable.
            if self.tcx.instantiate_and_check_impossible_clauses((item.def_id, args)) {
                continue;
            }
            // rustc won't resolve one of a blanket impl over a `?Sized` type,
            // `impl<Q: ?Sized> Equivalent<K> for Q`, as a `dyn` might be that
            // type too: in the impl's own dictionary it's the impl's item, or
            // the trait's default it keeps, `impl<T: Iterator + ?Sized>
            // Itertools for T {}`'s (ADR 0536).
            let own = || match self.tcx.impl_item_implementor_ids(id).get(&item.def_id) {
                Some(&method) => {
                    let args = ty::GenericArgs::for_item(self.tcx, method, |param, _| match param.kind {
                        ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
                        _ => self.tcx.mk_param_from_def(param),
                    });
                    Some(ty::Instance::new_raw(method, args))
                }
                None => self
                    .tcx
                    .defaultness(item.def_id)
                    .has_value()
                    .then(|| ty::Instance::new_raw(item.def_id, args)),
            };
            let instance = self
                .resolve_instance(item.def_id, args)?
                .or_else(own)
                .ok_or_else(|| self.unsupported(span, "this trait implementation"))?;
            let method = instance.def_id();
            // std's own `source`: `None`.
            if source && !self.krate.fns.contains_key(&method) {
                let none = Expr::arrow(
                    Vec::new(),
                    vec![StmtKind::Return(Some(Expr::undefined())).at(js::Span::NONE)],
                );
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), none));
                continue;
            }
            // std's own `Error::description`, deprecated: its text; and
            // `cause`, its `source` (ADR 0141).
            if let Some(cause) = error_default(self.tcx, item.def_id)
                && !self.krate.fns.contains_key(&method)
            {
                let entry = match cause {
                    true => self.shared_entry(&mut props, &mut shared, "source", span)?,
                    false => Expr::arrow(
                        Vec::new(),
                        vec![
                            StmtKind::Return(Some(Expr::str("description() is deprecated; use Display")))
                                .at(js::Span::NONE),
                        ],
                    ),
                };
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), entry));
                continue;
            }
            // std's own `write_char` and `write_fmt`: its `write_str`, given a
            // `char`'s text or a `write!`'s, as std's give it (ADR 0180).
            if is_writer_default(self.tcx, item.def_id) && !self.krate.fns.contains_key(&method) {
                let write_str = bindings::trait_method_name(self.tcx, tr.def_id, "write_str");
                let write_str = self.shared_entry(&mut props, &mut shared, &write_str, span)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), write_str));
                continue;
            }
            // std's own `write_all` and `write_fmt` of a writer of bytes of the
            // crate's: its `write`'s till all's written, as std's (ADR 0392).
            if let Some(all) = io_write_default(self.tcx, item.def_id)
                && !self.krate.fns.contains_key(&method)
            {
                let write = self.shared_entry(&mut props, &mut shared, "write", span)?;
                let helper = match all {
                    true => "$ioWriteAll",
                    false => "$ioWriteFmt",
                };
                props.push(Prop::Field(
                    bindings::fn_name(self.tcx, item.def_id),
                    Expr::call(Expr::helper(helper), vec![write]),
                ));
                continue;
            }
            // std's own `hash_slice` of a `Hash` of the crate's: each item's
            // `hash`, as std's (ADR 0463).
            if is_std_method(self.tcx, item.def_id, StdItem::Hash, "hash_slice")
                && !self.krate.fns.contains_key(&method)
            {
                let hash = self.shared_entry(&mut props, &mut shared, "hash", span)?;
                props.push(Prop::Field(
                    bindings::fn_name(self.tcx, item.def_id),
                    Expr::call(Expr::helper("$hashSlice"), vec![hash]),
                ));
                continue;
            }
            // A library's trait's default, whose body is the library's (ADR 0100):
            // its function over `Self`, given this impl's dictionary as any of
            // its generic functions is (ADR 0185), the crate's own where the
            // library's JS doesn't export it (ADR 0914, amended).
            let library_default = self.krate.foreign.in_library(method);
            if library_default && !self.is_rust_fn(method) {
                let what = format!(
                    "implementing another crate's trait without its default `{}`: write the method in the impl",
                    self.tcx.def_path_str(method)
                );
                return Err(self.unsupported(span, &what));
            }
            if !library_default && !self.krate.fns.contains_key(&method) {
                return Err(self.unsupported(span, &format!("trait method `{}`", self.tcx.def_path_str(method))));
            }
            if !library_default && self.tcx.trait_of_assoc(method).is_some() {
                let value = self.default_method(method, instance.args)?;
                props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
                continue;
            }
            let callee = self.fn_ref(method);
            // Without a `Formatter`, which isn't a JS parameter (ADR 0054).
            let count = self
                .tcx
                .fn_sig(method)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .inputs()
                .len()
                - usize::from(self.formatter_param(method).is_some())
                // A writer's options, where the crate's take them (ADRs 0058, 0137).
                + usize::from(self.writers_take_options() && self.formatter_param(method).is_some());
            let mut params: Vec<String> = (0..count).map(|i| format!("arg{i}")).collect();
            let mut values: Vec<Expr> = params.iter().map(|name| Expr::var(name)).collect();
            // Where it's called, after its arguments, of one whose entries take
            // it (ADR 0427): its caller's, generic code's or a `dyn`'s where
            // its declaration tracks its caller, and generic code's alone
            // where only an impl does, whose entry's is its own definition's
            // where a `dyn`'s call gives none, as rustc's shim gives it.
            let at_definition = self.location_at(self.tcx.def_span(method));
            let mut defaulted = None;
            let location = match self.takes_caller_location(item.def_id) {
                true => {
                    let name = self.fresh("location");
                    params.push(name.clone());
                    if !tracks_caller(self.tcx, item.def_id) && self.takes_caller_location(method) {
                        defaulted = Some((name.clone(), at_definition));
                    }
                    Expr::var(&name)
                }
                false => at_definition,
            };
            // Its own const parameters' values, given after its arguments
            // (ADR 0135), for the method, as the trait method's are named.
            let mark = self.given.const_params.len();
            for index in own_const_params(self.tcx, item.def_id) {
                if self.given.const_params.iter().any(|&(at, _)| at == index) {
                    return Err(self.unsupported(span, "a trait method's const parameter here"));
                }
                let name = self.fresh(
                    self.tcx
                        .generics_of(item.def_id)
                        .param_at(index as usize, self.tcx)
                        .name
                        .as_str(),
                );
                self.given.const_params.push((index, Expr::var(&name)));
                params.push(name);
            }
            // A generic method's own evidence is its caller's, after the arguments.
            let declared: Vec<_> = own_bounds(self.tcx, self.krate.foreign, self.krate.given, item.def_id)
                .into_iter()
                .map(|bound| {
                    ty::EarlyBinder::bind(self.tcx, bound)
                        .instantiate(self.tcx, args)
                        .skip_normalization()
                })
                .collect();
            let names: Vec<String> = declared
                .iter()
                .map(|&d| {
                    let word = format!("{}{}", evidence_word(self.tcx, d.self_ty()), trait_word(self.tcx, d));
                    self.fresh(&js_word(&word))
                })
                .collect();
            // What its caller through the dictionary gives in a box for each:
            // a generic `&mut Self` (ADR 0099), and a `Self` that's a `&mut` to
            // an object, an impl for `&mut Bag`'s (ADRs 0454, 0457); to a method
            // that takes a cell for each `&mut` it can't hold as the object,
            // `&mut u32`'s or `&mut &mut Money`'s: what's in each box more.
            let self_held = self.held_in_box(tr.self_ty());
            for (i, value) in values.iter_mut().enumerate() {
                let given = usize::from(self.param_is_box(item.def_id, i)) + usize::from(i == 0 && self_held);
                for _ in usize::from(self.param_is_box(method, i))..given {
                    *value = Expr::held(std::mem::replace(value, Expr::undefined()));
                }
            }
            // The drops its caller gives for its own type parameters, which it
            // passes on as the method's, where the method takes them (ADR 0163).
            let foreign = self.krate.foreign;
            let own_drops: Vec<u32> = match self.krate.drop_params.get(&method) {
                Some(taken)
                    if own_drop_params(self.tcx, foreign, method)
                        .iter()
                        .any(|i| taken.contains(i)) =>
                {
                    own_drop_params(self.tcx, foreign, item.def_id)
                }
                _ => Vec::new(),
            };
            let mut drop_names = Vec::new();
            let mut replaced = Vec::new();
            for &index in &own_drops {
                let ty = drop_slot(self.tcx, foreign, item.def_id, index);
                let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
                replaced.push(self.lend_drop_param(index, name.clone(), ty));
                drop_names.push(name);
            }
            // The facts its caller gives of its own type parameters, `KMap`,
            // which it passes on as the method's (ADR 0466), as the trait
            // method's are named.
            let parent = self.tcx.generics_of(item.def_id).parent_count as u32;
            let own_facts: Vec<(u32, TypeFact)> = (self.krate.type_facts.get(&item.def_id).into_iter().flatten())
                .copied()
                .filter(|&(index, _)| index >= parent)
                .collect();
            let fact_mark = self.given.type_facts.len();
            let mut fact_names = Vec::new();
            for (index, fact) in own_facts {
                let of = drop_slot(self.tcx, self.krate.foreign, item.def_id, index);
                if self.given.type_facts.iter().any(|&(at, _, _)| at == of) {
                    return Err(self.unsupported(span, "a trait method's type parameter's fact here"));
                }
                let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
                self.given.type_facts.push((of, fact, Expr::var(&name)));
                fact_names.push(name);
            }
            // Everything the method takes that its caller through the dictionary
            // doesn't give: its dictionaries, then its drops (ADR 0098), which
            // are this impl's own, and its own type parameters' it's given.
            let evidence = match declared.is_empty() {
                true => self.evidence_args_with(method, instance.args, location, span),
                false => self.method_evidence(method, instance.args, &declared, &names, location, span),
            };
            self.given.const_params.truncate(mark);
            self.given.type_facts.truncate(fact_mark);
            for lent in replaced {
                self.return_drop_param(lent);
            }
            let evidence = evidence?;
            params.extend(names);
            params.extend(fact_names);
            params.extend(drop_names);
            values.extend(evidence);
            // What generic code takes for an associated type bound by
            // `Iterator`, `fn items(&self) -> Self::Iter`, is a JS iterator
            // (ADR 0061): a range or the crate's own iterator the impl gives,
            // made one, as one given where a generic iterator goes is.
            let declared_output = (self.tcx.fn_sig(item.def_id).instantiate_identity())
                .skip_normalization()
                .skip_binder()
                .output();
            let output = (self.tcx.fn_sig(method).instantiate(self.tcx, instance.args))
                .skip_normalization()
                .skip_binder()
                .output();
            let iterates = matches!(declared_output.kind(), ty::Alias(..))
                && (self
                    .declared_bounds(declared_output, self.tcx.param_env(item.def_id))
                    .into_iter())
                .any(|id| is_std_def(self.tcx, id, StdItem::Iterator))
                && (self.range_kind(output).is_some() || self.is_user_iterator(output));
            let call = Expr::call(callee.clone(), values.clone());
            let call = match iterates {
                true => Some(self.as_js_iterator(call, output, span)?),
                false => None,
            };
            // A `&mut` generic code has in a box by the trait's signature, `&mut
            // Self::Target` of a `Vec`, which the method gives as the object:
            // a box of it, which a write of the whole changes in place (ADR
            // 0454), as the caller through the dictionary takes it out. Of an
            // impl's own type parameter, the trait's is generic too, and the
            // method's box is it.
            let call = match self.boxed_in_call(item.def_id, declared_output, args) {
                Some(_) => Some(Expr::call(
                    Expr::helper("$cellOf"),
                    vec![call.unwrap_or_else(|| Expr::call(callee.clone(), values.clone()))],
                )),
                None => call,
            };
            // A `fmt::Result` the trait's signature has as a `Result`, `fn
            // write(&mut self) -> Result<(), Self::Error>`'s of an impl whose
            // `Error` is `fmt::Error`: one (ADR 0461).
            let call = match self.generic_fmt_result(declared_output, args) {
                true => {
                    let made = call.unwrap_or_else(|| Expr::call(callee.clone(), values.clone()));
                    Some(Expr::call_helper(
                        "$writeResult",
                        vec![Expr::arrow(
                            Vec::new(),
                            vec![js::StmtKind::Return(Some(made)).at(js::Span::NONE)],
                        )],
                    ))
                }
                false => call,
            };
            let made = call.is_none();
            let patterns = params.into_iter().map(|name| match &defaulted {
                Some((location, at)) if *location == name => js::Pattern::Defaulted(name, at.clone()),
                _ => name.into(),
            });
            let arrow = Expr::arrow(
                patterns.collect(),
                vec![StmtKind::Return(Some(call.unwrap_or_else(|| Expr::call(callee, values)))).at(js::Span::NONE)],
            );
            // One that passes on just what it's given, in order, is the method.
            let value = match made {
                true => passed_on(&arrow, Taker::Made, &[]).unwrap_or(arrow),
                false => arrow,
            };
            props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
        }
        props.extend(self.dictionary_drop(tr.self_ty(), span)?);
        props.extend(self.item_drops(tr, span)?);
        props.extend(self.item_facts(tr, span)?);
        let object = match is_hasher(self.tcx, tr.def_id) {
            true => Expr::call(Expr::helper("$hasher"), vec![Expr::object(props)]),
            false => Expr::object(props),
        };
        let mut body = Vec::new();
        let mut made = None;
        if !self.takes_evidence(id) {
            // Given nothing, it's its module's `const` (ADR 0592): what its
            // entries share is the module's too, named apart by its cache's
            // name, `$smallWrite_writeStr`.
            let renamed: Vec<(String, String)> = (shared.iter())
                .filter_map(|stmt| match &stmt.kind {
                    StmtKind::Const(name, _) => Some((name.clone(), format!("{cache}_{name}"))),
                    _ => None,
                })
                .collect();
            let rename = |v: &str| {
                (renamed.iter())
                    .find(|(from, _)| from == v)
                    .map(|(_, to)| Expr::var(to))
            };
            for stmt in &mut shared {
                if let StmtKind::Const(name, value) = &mut stmt.kind {
                    *value = value.substitute(&rename).unwrap_or(value.clone());
                    *name = format!("{cache}_{name}");
                }
            }
            body.extend(shared);
            body.push(StmtKind::Return(Some(object.substitute(&rename).unwrap_or(object))).at(js::Span::NONE));
        } else {
            // Keyed by a const parameter's value first, or a type's fact, a
            // number, which only a `Map` holds (ADRs 0107, 0145).
            let map = match self.given.const_params.is_empty() && self.given.type_facts.is_empty() {
                true => "WeakMap",
                false => "Map",
            };
            made = Some(Expr::new_(Expr::var(map), Vec::new()));
            // One dictionary for each set of what it's given: its const
            // parameters' values, its dictionaries, what it asks of its type
            // parameters, `TSize` (ADR 0145), and its drops, which may be none.
            let keys = Expr::array(
                self.given
                    .const_params
                    .iter()
                    .map(|(_, value)| value.clone())
                    .chain(self.given.evidence.iter().map(|(_, value)| value.clone()))
                    .chain(self.given.type_facts.iter().map(|(_, _, value)| value.clone()))
                    .chain(self.given_drops().iter().map(|name| Expr::var(name)))
                    .collect(),
            );
            let make = Expr::arrow(
                Vec::new(),
                [shared, vec![StmtKind::Return(Some(object)).at(js::Span::NONE)]].concat(),
            );
            body.push(
                StmtKind::Return(Some(Expr::call(
                    Expr::helper("$traitImpl"),
                    vec![Expr::var(cache), keys, make],
                )))
                .at(js::Span::NONE),
            );
        }
        let function = js::Function {
            name: self.krate.fns[&id].name.clone(),
            params,
            body,
            // Not a library's the crate compiles itself, its own (ADR 0914).
            export: self.tcx.visibility(tr.def_id).is_public() && !self.krate.foreign.is_instance(id),
            is_async: false,
            noinline: false,
            span: self.js_span(span),
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
        };
        Ok((function, made))
    }

    /// Copy the default body into this implementation. Its Rust bindings still
    /// refer to the trait definition; only its evidence is specialized here.
    fn default_method(&mut self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> R<Expr> {
        let span = self.tcx.def_span(id);
        let mut specialized = Vec::new();
        // A generic default's own evidence is its caller's, after its
        // arguments (ADR 0106); the rest is the impl's, made here.
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.given, id);
        let mut own_params = Vec::new();
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.given, id) {
            if own.contains(&bound) {
                let word = format!(
                    "{}{}",
                    evidence_word(self.tcx, bound.self_ty()),
                    trait_word(self.tcx, bound)
                );
                let name = self.fresh(&js_word(&word));
                specialized.push((bound, Expr::var(&name)));
                // And as the body asks for it, in the impl's terms: a method's
                // `where Self: PartialEq` of a `Wrapping<T>`'s.
                let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                    .instantiate(self.tcx, args)
                    .skip_normalization();
                if concrete != bound {
                    specialized.push((concrete, Expr::var(&name)));
                }
                own_params.push((bound, name));
                continue;
            }
            let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                .instantiate(self.tcx, args)
                .skip_normalization();
            specialized.push((bound, self.dictionary(concrete, span)?));
        }
        // And the impl's own, `SClone` of `impl<S: Clone> Movable<S> for
        // Point<S>`: a call on the impl's types, which the body's resolve
        // to, is given them.
        specialized.extend(self.given.evidence.iter().cloned());
        // Its trait's type parameters, `Self` among them, drop as the impl's
        // arguments for them do, with the impl's drops (ADR 0098): each is
        // made here, and the body is given it by name.
        let mut made = Vec::new();
        let mut drops = HashMap::new();
        // One rust-js can't make is an error only if the body drops one.
        let mut unsupported = HashMap::new();
        let generics = self.tcx.generics_of(id);
        for (index, arg) in args.iter().enumerate() {
            let Some(ty) = arg.as_type() else {
                continue;
            };
            match self.drops(ty) {
                Drops::Nothing => continue,
                Drops::Unsupported(t, what) => {
                    unsupported.insert(index as u32, (t, what));
                    continue;
                }
                Drops::Runs => {}
            }
            let Some(drop) = self.drop_function(ty, span)? else {
                continue;
            };
            let name = match &drop.kind {
                js::ExprKind::Var(name) => name.clone(),
                _ => {
                    let name = self.fresh(&format!("drop{}", param_word(generics.param_at(index, self.tcx).name)));
                    made.push((index as u32, StmtKind::Const(name.clone(), drop).at(js::Span::NONE)));
                    name
                }
            };
            drops.insert(index as u32, name);
        }
        // And each drop of a std associated type the impl's given, `dropTItem`
        // of `T::Item`, as the body asks in the impl's terms (ADR 0421), and
        // each it could be given (ADR 0489).
        let mut items = HashMap::new();
        for (at, (alias, name)) in self.given_item_drops().into_iter().enumerate() {
            let index = (generics.count() + at) as u32;
            drops.insert(index, name);
            items.insert(alias, index);
        }
        let slots: HashSet<ty::AliasTy<'tcx>> = (drop_items(self.tcx, self.krate.foreign, id).into_iter())
            .filter_map(|own| {
                match *ty::EarlyBinder::bind(self.tcx, own)
                    .instantiate(self.tcx, args)
                    .skip_normalization()
                    .kind()
                {
                    ty::Alias(_, alias) => Some(alias),
                    _ => None,
                }
            })
            .collect();
        // Its trait's const parameters are the impl's arguments for them, `10`
        // of `Scaled<10>` or the impl's own `K`, and its own are given after
        // its arguments (ADR 0135).
        let mut consts = Vec::new();
        let generics = self.tcx.generics_of(id);
        for index in 0..generics.parent_count {
            let param = generics.param_at(index, self.tcx);
            if matches!(param.kind, ty::GenericParamDefKind::Const { .. }) {
                consts.push((param.index, self.const_arg(args.const_at(index), span)?));
            }
        }
        // A C-variadic default's own, which can't come after what it's given
        // past the rest (ADR 0428).
        if crate::lower::recognition::is_c_variadic(self.tcx, id)
            && (!own_params.is_empty() || !own_const_params(self.tcx, id).is_empty())
        {
            return Err(self.unsupported(span, "a C-variadic default given a dictionary or a const"));
        }
        let mut const_names = Vec::new();
        for index in own_const_params(self.tcx, id) {
            let name = self.fresh(generics.param_at(index as usize, self.tcx).name.as_str());
            consts.push((index, Expr::var(&name)));
            const_names.push((index, name));
        }
        // And the impl's, which its type, `Self`, names: `CAP` of an
        // `ArrayVec<T, CAP>`. One numbered as one of the trait's is would
        // be either: refused, as a method's own is (above).
        let names_const = |index: u32| {
            args.iter().any(|arg| {
                arg.walk()
                    .any(|part| matches!(part.as_const().map(|c| c.kind()), Some(ty::ConstKind::Param(p)) if p.index == index))
            })
        };
        for (index, value) in self.given.const_params.clone() {
            match consts.iter().any(|&(at, _)| at == index) {
                true if names_const(index) => {
                    return Err(self.unsupported(span, "a trait default's const parameter numbered as its impl's"));
                }
                true => {}
                false => consts.push((index, value)),
            }
        }
        // The facts its caller gives of its own type parameters, `KMap`,
        // after its dictionaries (ADR 0466).
        let mut facts: Vec<(Ty<'tcx>, TypeFact, Expr)> = Vec::new();
        let mut fact_names = Vec::new();
        for &(index, fact) in self.krate.type_facts.get(&id).into_iter().flatten() {
            if index < generics.parent_count as u32 {
                continue;
            }
            let of = drop_slot(self.tcx, self.krate.foreign, id, index);
            if self.given.type_facts.iter().chain(&facts).any(|&(at, _, _)| at == of) {
                return Err(self.unsupported(span, "a trait default's type parameter's fact here"));
            }
            let name = self.fresh(&format!("{}{}", drop_word(self.tcx, of), fact.word()));
            facts.push((of, fact, Expr::var(&name)));
            // And in the impl's terms, as the body asks: `T::Item` of the
            // trait's `Self::Item`.
            let concrete = ty::EarlyBinder::bind(self.tcx, of)
                .instantiate(self.tcx, args)
                .skip_normalization();
            if concrete != of {
                facts.push((concrete, fact, Expr::var(&name)));
            }
            fact_names.push((index, name));
        }
        // A method's, or a constant's initializer, a trait's default (ADR 0176).
        let body = match self.krate.bodies.get(&id) {
            Some(body) => *body,
            None => (self.krate.closures.get(&id).copied())
                .ok_or_else(|| self.unsupported(span, "a trait's default of another crate"))?,
        };
        // Where it's called, after its arguments, of one that takes it, which
        // `Location::caller()` in it reads (ADR 0427).
        let location = self.takes_caller_location(id).then(|| self.fresh("location"));
        let nested = crate::lower::Nested::Default {
            given: Box::new(super::DefaultGiven {
                evidence: specialized,
                self_args: args,
                const_params: consts,
                caller_location: location.as_ref().map(|name| (id, Expr::var(name))),
                type_facts: facts,
            }),
            typing_env: ty::TypingEnv::post_analysis(self.tcx, id),
            drops,
            unsupported,
            items,
            slots,
        };
        let mut rest = Vec::new();
        // From its MIR (ADR 0364), a constant's initializer of nothing, its
        // value given back. From its MIR, as the trait has it, `Self` the
        // impl's type where it's asked (ADR 0364). Only the drops it uses:
        // most defaults drop nothing of their `Self`.
        // A trait of classes' default, its class's prototype's method, as
        // the class has it: `Self` the class, and each method its impl
        // gives a field by that field, read in place (rd283).
        let class_default = (body.mir.as_ref())
            .filter(|_| crate::lower::classes::is_class_trait(self.tcx, self.tcx.parent(id)))
            .map(|mir| {
                let tcx = self.tcx;
                let bodies = &self.krate.bodies;
                mir.instantiated(tcx, args).fields_read(tcx, &|callee, generic_args| {
                    let instance =
                        ty::Instance::try_resolve(tcx, ty::TypingEnv::fully_monomorphized(), callee, generic_args)
                            .ok()??;
                    crate::lower::classes::trait_member_class(tcx, instance.def_id())?;
                    bodies.get(&instance.def_id())?.mir.as_ref()?.field_given()
                })
            });
        let ((signature, used), hoisted) = self.lowered_in(body, id, nested, |cx| {
            let signature = match class_default.as_ref().or(body.mir.as_ref()) {
                Some(mir) => cx.mir_signature(body, mir, &mut rest)?,
                None => return Err(cx.unsupported(span, "this default, without its MIR")),
            };
            Ok((signature, cx.used_drops()))
        })?;
        // After its arguments, what its caller gives of its own, in
        // `evidence_slots`' order, as `own_evidence` gives them: no drops,
        // which its caller gives the impl's, made here.
        let (mut params, is_async) = signature;
        let named = |names: &[(u32, String)], at: u32| names.iter().find(|&&(i, _)| i == at).map(|(_, n)| n.clone());
        for slot in self.evidence_slots(id, Scope::Own) {
            let name = match slot {
                Slot::Location => location.clone(),
                Slot::Const(index) => named(&const_names, index),
                Slot::Bound(bound) => own_params.iter().find(|&&(b, _)| b == bound).map(|(_, n)| n.clone()),
                Slot::Fact(index, _) => named(&fact_names, index),
                Slot::Drop(_) => continue,
            };
            params.push(name.expect("each of a default's own slots named").into());
        }
        rest.splice(0..0, hoisted);
        let mut out: Vec<_> = made
            .into_iter()
            .filter(|(index, _)| used.contains(index))
            .map(|(_, stmt)| stmt)
            .collect();
        out.extend(rest);
        Ok(if is_async {
            Expr::async_arrow(params, out)
        } else {
            Expr::arrow(params, out)
        })
    }

    /// Of `method`, a trait's, for its `args`, given `count` values: where it
    /// resolves to a library's impl's method that its JS reaches only
    /// through the impl's dictionary, the method resolved and that entry,
    /// `counterClone.clone`, where it's called as the method is: one of no
    /// type parameters of its own, given what it takes as it takes it; else
    /// the crate compiles the impl itself, asked for, and its entry is a
    /// placeholder (ADR 0914, amended). `None` of another.
    pub(in crate::lower) fn dictionary_entry(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        count: usize,
        span: Span,
    ) -> R<Option<(ty::Instance<'tcx>, Expr)>> {
        let Some(instance) = self.resolve_instance(method, args)? else {
            return Ok(None);
        };
        let Some(imp) = self.krate.foreign.through_dictionary(instance.def_id()) else {
            return Ok(None);
        };
        let tr = ty::TraitRef::from_assoc(self.tcx, self.tcx.parent(method), args);
        let as_entry = self.tcx.generics_of(method).own_params.is_empty()
            && !self.held_in_box(tr.self_ty())
            && (0..count).all(|i| self.param_is_box(method, i) == self.param_is_box(instance.def_id(), i));
        if !as_entry {
            self.krate.asked.borrow_mut().push((imp, self.module));
            return Ok(Some((instance, Expr::undefined())));
        }
        let dictionary = self.dictionary(tr, span)?;
        Ok(Some((
            instance,
            Expr::member(dictionary, bindings::fn_name(self.tcx, method)),
        )))
    }

    /// `method` of the hand-written impl for the trait's `args`, called
    /// directly: `counterClone_clone(c)`, or through a library's dictionary
    /// (`dictionary_entry`), `counterClone.clone(c)`.
    pub(in crate::lower) fn impl_call(
        &mut self,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
        mut values: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        let args = self.tcx.erase_and_anonymize_regions(args);
        if let Some((instance, entry)) = self.dictionary_entry(method, args, values.len(), span)? {
            let called = Expr::call(entry, values);
            return Ok(self.fmt_result_value(instance.def_id(), instance.args, called));
        }
        let instance = self
            .resolve_instance(method, args)?
            .filter(|i| self.is_rust_fn(i.def_id()))
            .ok_or_else(|| self.unsupported(span, "this implementation"))?;
        values.extend(self.evidence_args(instance.def_id(), instance.args, span)?);
        let called = Expr::call(self.fn_ref(instance.def_id()), values);
        Ok(self.fmt_result_value(instance.def_id(), instance.args, called))
    }
}

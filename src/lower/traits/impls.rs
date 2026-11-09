//! An impl's dictionary lowered: its entries, its supertraits' and its
//! bounds', and a trait's default body copied into it.

use super::*;
use crate::lower::mut_refs::{Within, adapted_part};

/// What an impl's entry makes of what its caller through the dictionary
/// gives its method and of what the method gives back (`entry_forms`).
struct EntryForms<'tcx> {
    /// Of each parameter, how many boxes' `value` it's taken out of.
    unboxed: Vec<usize>,
    /// Of each parameter, what's within it past its own `&mut` (#686).
    within: Vec<Option<Within<'tcx>>>,
    /// The result's own `&mut` is made generic code's box, an in-place
    /// cell (ADR 0454).
    result_cell: bool,
    /// What's within the result past its own `&mut` (#686).
    result_within: Option<Within<'tcx>>,
}

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

    /// Whether generic code holds a value of `self_ty` in a box: a `&mut` to
    /// an object, given for a type parameter (ADR 0454), `for x in c` of a `c:
    /// &mut C`'s (ADR 0457). A `&C` is the value, and a `&mut` to a number
    /// or text its cell.
    pub(super) fn held_in_box(&self, self_ty: Ty<'tcx>) -> bool {
        matches!(*self_ty.kind(), ty::Ref(_, pointee, Mutability::Mut) if !self.mut_ref_form(pointee).is_cell())
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
    /// Of a type generic code has as the JS iterator the dictionary made of
    /// it (`item_made_js_iterator`), its `Iterator`'s, its back's and its
    /// length's are that JS iterator's (#972).
    pub(super) fn item_bound_entries(&mut self, tr: ty::TraitRef<'tcx>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (name, bound) in item_bounds(self.tcx, tr.def_id, tr.args) {
            if !super::has_item_bound_entry(self.tcx, self.krate.foreign, bound.def_id) {
                continue;
            }
            let made = match *bound.self_ty().kind() {
                ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Projection { def_id: item },
                        ..
                    },
                ) => {
                    let ty = self
                        .tcx
                        .normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(bound.self_ty()));
                    (self.item_made_js_iterator(item, ty))
                        .then(|| self.made_iterator_dictionary(bound.def_id, ty))
                        .flatten()
                }
                _ => None,
            };
            let dictionary = match made {
                Some(made) => made,
                None => self.dictionary(bound, span)?,
            };
            props.push(Prop::Field(
                name,
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
            ));
        }
        Ok(props)
    }

    /// Whether generic code has values of the associated type `item` an
    /// impl gives as `ty` as JS iterators its dictionary made
    /// (`made_js_iterator`), as the trait declares the type.
    pub(super) fn item_made_js_iterator(&self, item: DefId, ty: Ty<'tcx>) -> bool {
        let identity = ty::GenericArgs::identity_for_item(self.tcx, item);
        let alias = Ty::new_projection(self.tcx, ty::IsRigid::No, item, identity);
        self.made_js_iterator(alias, self.tcx.param_env(item), ty)
    }

    /// The dictionary of `trait_id`, `Iterator` or std's extension of it, of
    /// a JS iterator the dictionary made of a `ty` (`made_js_iterator`):
    /// `{ next: $next }`, `{ next_back: $nextBack }`, `{ len }` its own
    /// `len()`, each giving a `Some` that looks like `None` boxed (ADR 0051).
    fn made_iterator_dictionary(&self, trait_id: DefId, ty: Ty<'tcx>) -> Option<Expr> {
        let boxed = self.iterator_item(ty).is_none_or(|item| self.boxed_payload(item));
        let (entry, value) = if is_std_def(self.tcx, trait_id, StdItem::Iterator) {
            (
                "next".to_string(),
                Expr::helper(if boxed { "$nextSome" } else { "$next" }),
            )
        } else if crate::lower::recognition::is_double_ended(self.tcx, trait_id) {
            let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
            (
                name.to_string(),
                Expr::helper(if boxed { "$nextBackSome" } else { "$nextBack" }),
            )
        } else if self.recognition().exact_size_iterator() == Some(trait_id) {
            let name = crate::lower::recognition::extension_entry(self.tcx, trait_id);
            let len = Expr::call(Expr::member(Expr::var("iterator"), "len"), Vec::new());
            let len = Expr::arrow(
                vec!["iterator".into()],
                vec![StmtKind::Return(Some(len)).at(js::Span::NONE)],
            );
            (name.to_string(), len)
        } else {
            return None;
        };
        Some(Expr::object(vec![Prop::Field(entry, value)]))
    }

    /// What a JS iterator the dictionary makes of an associated type's
    /// value, `alias` as the trait declares it under `env`, knows: its back
    /// and its length, where the trait bounds it so, as generic code steps
    /// it (ADRs 0429, 0456; #972).
    fn made_iterator_ends(&self, alias: Ty<'tcx>, env: ty::ParamEnv<'tcx>) -> (bool, bool) {
        let bounds = self.declared_bounds(alias, env);
        let back = bounds
            .iter()
            .any(|&id| crate::lower::recognition::is_double_ended(self.tcx, id));
        let len = bounds
            .iter()
            .any(|&id| self.recognition().exact_size_iterator() == Some(id));
        (back, len)
    }

    /// An entry giving each of `bounds`' dictionaries, each one that has
    /// one, as `held_supertraits` keeps them: a marker, `Send`, none.
    fn entries_of(&mut self, bounds: Vec<(String, ty::TraitRef<'tcx>)>, span: Span) -> R<Vec<Prop>> {
        let mut props = Vec::new();
        for (name, bound) in bounds {
            let dictionary = self.dictionary(bound, span)?;
            props.push(Prop::Field(
                name,
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(dictionary)).at(js::Span::NONE)]),
            ));
        }
        Ok(props)
    }

    /// Whether generic code has a value of an associated type, `alias` as
    /// the trait declares it under `env`, of the type `ty` an impl gives it,
    /// as a JS iterator the impl's dictionary made of it (ADR 0061): one
    /// bound by `Iterator`, a range, the crate's own iterator, or the impl's
    /// type parameter given as it is, by its dictionaries (#972). The one
    /// answer, of what its methods give and what its drop drops (#978).
    pub(super) fn made_js_iterator(&self, alias: Ty<'tcx>, env: ty::ParamEnv<'tcx>, ty: Ty<'tcx>) -> bool {
        matches!(alias.kind(), ty::Alias(..))
            && (self.declared_bounds(alias, env).into_iter()).any(|id| is_std_def(self.tcx, id, StdItem::Iterator))
            && (self.range_kind(ty).is_some() || self.is_user_iterator(ty) || self.iterator_given_as_it_is(ty))
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
                let Some(value) = evaluated.and_then(|value| const_js(self.tcx, &self.krate.flattened.keyed, value))
                else {
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
                        None => self.default_method(id, item.def_id, tr.args)?,
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
            // does, which it asks the dictionary for; and a `PartialOrd`'s
            // own `lt`, `le`, `gt` and `ge`, a `PartialEq`'s own `ne` and an
            // `Ord`'s own `max`, `min` and `clamp`, which generic code and
            // std's helpers call through it, `$partialOrd`, `$partialEq` and
            // `$ord` giving std's where it writes none (#937, #944, #962).
            let source = self.is_error_source(item.def_id);
            let own_clone_from = is_clone_from(self.tcx, item.def_id) && self.recognition().has_clone_from(id);
            let own_operator = crate::lower::ordering::held_by_comparison_dictionary(self.tcx, item.def_id)
                && self.tcx.impl_item_implementor_ids(id).contains_key(&item.def_id);
            if !self.is_rust_trait(tr.def_id)
                && self.tcx.defaultness(item.def_id).has_value()
                && !in_std_dictionary(self.tcx, item.def_id)
                && !own_clone_from
                && !own_operator
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
                let value = self.default_method(id, method, instance.args)?;
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
            // What it takes and gives as its caller through the dictionary
            // has them, made its own (`entry_forms`).
            let forms = self.entry_forms(item.def_id, (method, instance.args), args, count, None);
            for (i, value) in values.iter_mut().enumerate() {
                for _ in 0..forms.unboxed[i] {
                    *value = Expr::held(std::mem::replace(value, Expr::undefined()));
                }
                if let Some(made) = (forms.within[i].as_ref()).and_then(|within| within.adapted(value, true)) {
                    *value = made;
                }
            }
            // The drops its caller gives for its own type parameters, which it
            // passes on as the method's, where the method takes them (ADR 0163).
            let own_drops = self.own_drops_taken(method, item.def_id);
            let mut drop_names = Vec::new();
            let mut replaced = Vec::new();
            for &index in &own_drops {
                let ty = drop_slot(self.tcx, self.slots(), item.def_id, index);
                let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
                replaced.push(self.lend_drop_param(name.clone(), ty));
                drop_names.push(name);
            }
            // The facts its caller gives of its own type parameters, `KMap`,
            // which it passes on as the method's (ADR 0466), as the trait
            // method's are named.
            let parent = self.tcx.generics_of(item.def_id).parent_count as u32;
            // A std trait's required method's, what its declaration says
            // (`std_method_facts`, #794).
            let own_facts: Vec<(u32, TypeFact)> = (self.krate.type_facts.get(&item.def_id).into_iter().flatten())
                .copied()
                .chain(super::std_method_facts(self.tcx, self.krate.foreign, item.def_id))
                .filter(|&(index, _)| index >= parent)
                .collect();
            let fact_mark = self.given.type_facts.len();
            let mut fact_names = Vec::new();
            for (index, fact) in own_facts {
                // Of its type for this impl, `<I as Iterator>::Item` of a
                // `Wrap<I>`'s, as the method's body asks it, named as its
                // bounds' dictionaries are, `WrapItemMap` (#990).
                let declared = drop_slot(self.tcx, self.slots(), item.def_id, index);
                let declared = ty::EarlyBinder::bind(self.tcx, declared)
                    .instantiate(self.tcx, args)
                    .skip_normalization();
                let of = self.drop_slot_of(item.def_id, index, args);
                if self.given.type_facts.iter().any(|&(at, _, _)| at == of) {
                    return Err(self.unsupported(span, "a trait method's type parameter's fact here"));
                }
                let name = self.fresh(&format!("{}{}", drop_word(self.tcx, declared), fact.word()));
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
            let env = self.tcx.param_env(item.def_id);
            let iterates = self.made_js_iterator(declared_output, env, output);
            let call = Expr::call(callee.clone(), values.clone());
            // Knowing its back and its length where the trait bounds it so.
            let call = match (iterates, self.made_iterator_ends(declared_output, env)) {
                (false, _) => None,
                (true, (back, true)) => Some(self.as_exact_size(call, output, back, span)?),
                (true, (true, false)) => Some(self.as_double_ended(call, output, span)?),
                (true, (false, false)) => Some(self.as_js_iterator(call, output, span)?),
            };
            // A `&mut` generic code has in a box by the trait's signature, `&mut
            // Self::Target` of a `Vec`, which the method gives as the object:
            // a box of it, which a write of the whole changes in place (ADR
            // 0454), as the caller through the dictionary takes it out. Of an
            // impl's own type parameter, the trait's is generic too, and the
            // method's box is it.
            let call = match forms.result_cell {
                true => Some(crate::lower::mut_refs::in_place_cell(
                    call.unwrap_or_else(|| Expr::call(callee.clone(), values.clone())),
                )),
                false => call,
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
            // And a `&mut` within what it gives, `Option<&mut Self>`'s, the
            // impl's made generic code's box (#686).
            let given = self.fresh("given");
            let given_back =
                (forms.result_within.as_ref()).and_then(|within| within.adapted(&Expr::var(&given), false));
            let made = call.is_none() && given_back.is_none();
            let call = call.unwrap_or_else(|| Expr::call(callee, values));
            let body = match given_back {
                Some(adapted) => vec![
                    StmtKind::Const(given, call).at(js::Span::NONE),
                    StmtKind::Return(Some(adapted)).at(js::Span::NONE),
                ],
                None => vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
            };
            let patterns = params.into_iter().map(|name| match &defaulted {
                Some((location, at)) if *location == name => js::Pattern::Defaulted(name, at.clone()),
                _ => name.into(),
            });
            let arrow = Expr::arrow(patterns.collect(), body);
            // One that passes on just what it's given, in order, is the method.
            // Or one that passes on its first, to a method that declares just
            // those: what a caller gives past them it never names, a fact the
            // trait carries that the impl doesn't read (#801). Of one whose
            // last gathers the rest, none. As entries are built, a method an
            // entry passes a prefix to declares just that many.
            let first = passed_on_first(&arrow).and_then(|(own, passed)| {
                let declared = self.declared_arity(method)?;
                debug_assert_eq!(
                    declared, passed,
                    "an entry passes on a prefix of what its method declares"
                );
                Some(own)
            });
            let value = match (made, first) {
                (true, Some(method)) => method,
                (true, None) => passed_on(&arrow, Taker::Made, &[]).unwrap_or(arrow),
                (false, _) => arrow,
            };
            props.push(Prop::Field(bindings::fn_name(self.tcx, item.def_id), value));
        }
        props.extend(self.dictionary_drop(tr.self_ty(), span)?);
        props.extend(self.item_drops(tr, span)?);
        props.extend(self.item_facts(tr, span)?);
        let object = match is_hasher(self.tcx, tr.def_id) {
            true => Expr::call(Expr::helper("$hasher"), vec![Expr::object(props)]),
            false => match crate::lower::ordering::comparison_maker(self.tcx, tr.def_id) {
                Some(maker) => Expr::call(Expr::helper(maker), vec![Expr::object(props)]),
                None => Expr::object(props),
            },
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
            // Keyed by what it's given first: a const parameter's value, a
            // number, which only a `Map` holds (ADR 0107), else a dictionary,
            // held weakly, a type's fact after it in a cache of its own,
            // `$traitImpl`'s; a fact first, of an impl given no dictionary,
            // a `Map` (ADR 0145), whose keys are a type's few facts.
            let weak = self.given.const_params.is_empty()
                && (!self.given.evidence.is_empty() || self.given.type_facts.is_empty());
            let map = match weak {
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
            set_apart: false,
        };
        Ok((function, made))
    }

    /// Whether a call of `item`, a trait's method, with `args`, through the
    /// dictionary of the impl its `Self` selects, takes its parameter `i`'s
    /// box apart where it's called: the entry reads the box's `value` once
    /// and gives the method, the impl's or a default copied into it, the
    /// object it holds, which it changes in place, so nothing writes the
    /// box whole, and a copy back would copy the object onto itself (#831,
    /// `entry_forms`'s `unboxed`). Not of a `Self` no impl is selected for
    /// here, generic code's, nor of a blanket impl's generic `Self`, whose
    /// method writes the box; nor of another crate's impl, whose entry is
    /// its own, nor of a default the dictionary doesn't copy into the impl,
    /// a library's, compiled once over a generic `Self` (ADR 0100), or
    /// std's.
    pub(in crate::lower) fn entry_takes_box_apart(
        &self,
        item: DefId,
        args: ty::GenericArgsRef<'tcx>,
        i: usize,
    ) -> bool {
        self.entry_unboxes(item, args, i) == Some(true)
    }

    /// Whether the crate's entry of `item` gives its parameter `i`'s box to
    /// its method as it is, a blanket impl's generic `Self`'s, whose method
    /// may write the box whole, `*self = v` (#968): the impl's own method
    /// or a default copied into it, of an impl `entry_takes_box_apart`
    /// says doesn't take it apart. Not where the crate has no entry of it.
    pub(in crate::lower) fn entry_gives_box(&self, item: DefId, args: ty::GenericArgsRef<'tcx>, i: usize) -> bool {
        self.entry_unboxes(item, args, i) == Some(false)
    }

    /// Whether the crate's entry of `item` takes its parameter `i`'s box
    /// apart, `entry_takes_box_apart`'s and `entry_gives_box`'s one answer;
    /// `None` where the crate has no entry of it.
    fn entry_unboxes(&self, item: DefId, args: ty::GenericArgsRef<'tcx>, i: usize) -> Option<bool> {
        let tr = ty::TraitRef::from_assoc(self.tcx, self.tcx.parent(item), args);
        // The crate's, or a library's, whose entry its library made of the
        // same signatures (#1032).
        let (imp, _) = super::evidence_impl(self.tcx, self.typing_env, tr)
            .filter(|(imp, _)| imp.is_local() || self.krate.foreign.in_library(*imp))?;
        // As the impl's dictionary makes its entry (`lower_dictionary`): in
        // the impl's own terms, the impl's method or the default it keeps.
        let impl_tr = self.tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization();
        let own_args = impl_tr.args.extend_to(self.tcx, item, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
            _ => self.tcx.mk_param_from_def(param),
        });
        let count = (self.tcx.fn_sig(item).instantiate_identity())
            .skip_normalization()
            .skip_binder()
            .inputs()
            .len();
        let forms = match self.tcx.impl_item_implementor_ids(imp).get(&item) {
            Some(&method) => {
                let method_args = ty::GenericArgs::for_item(self.tcx, method, |param, _| match param.kind {
                    ty::GenericParamDefKind::Lifetime => self.tcx.lifetimes.re_erased.into(),
                    _ => self.tcx.mk_param_from_def(param),
                });
                self.entry_forms(item, (method, method_args), own_args, count, None)
            }
            // A default the dictionary copies in (`lower_dictionary`): the
            // crate's trait's, not a library's, whose body is its own.
            None if self.tcx.defaultness(item).has_value()
                && self.is_rust_trait(tr.def_id)
                && !self.krate.foreign.in_library(item)
                && self.krate.fns.contains_key(&item) =>
            {
                self.entry_forms(item, (item, own_args), own_args, count, Some(imp))
            }
            None => return None,
        };
        Some(forms.unboxed.get(i).is_some_and(|&unboxed| unboxed > 0))
    }

    /// Copy the default body into this implementation. Its Rust bindings still
    /// refer to the trait definition; only its evidence is specialized here.
    /// What an impl's entry of `item`, a trait's method, makes of what its
    /// caller through the dictionary gives `method`, the impl's, given
    /// `instance_args`, for the trait's `args`, of its first `count` values,
    /// and of what `method` gives back (ADRs 0099, 0454, 1086): the one
    /// answer of what the entry adapts. Of a default copied into
    /// `copied_into`, what its body takes is in that impl's terms
    /// (`default_args`, #803).
    fn entry_forms(
        &self,
        item: DefId,
        (method, instance_args): (DefId, ty::GenericArgsRef<'tcx>),
        args: ty::GenericArgsRef<'tcx>,
        count: usize,
        copied_into: Option<DefId>,
    ) -> EntryForms<'tcx> {
        // What its caller gives in a box for each: a generic `&mut Self` (ADR
        // 0099), and a `Self` that's a `&mut` to an object, an impl for `&mut
        // Bag`'s (ADRs 0454, 0457); to a method that takes a cell for each
        // `&mut` it can't hold as the object, `&mut u32`'s or `&mut &mut
        // Money`'s: what's in each box more.
        let tr = ty::TraitRef::from_assoc(self.tcx, self.tcx.parent(item), args);
        let self_held = self.held_in_box(tr.self_ty());
        // What the method takes, as it's lowered: a copied default's in its
        // impl's terms, where its body is (`default_args`).
        let takes_box = |i: usize| match copied_into {
            Some(imp) => {
                let env = default_typing_env(self.tcx, imp, item, instance_args);
                self.param_form_of(item, instance_args, env.param_env, i)
                    .is_some_and(|form| form.is_cell())
            }
            None => self.param_is_box(method, i),
        };
        let unboxed = (0..count)
            .map(|i| {
                let given = usize::from(self.param_is_box(item, i)) + usize::from(i == 0 && self_held);
                given.saturating_sub(usize::from(takes_box(i)))
            })
            .collect();
        // And a `&mut` within what it takes past its own, `Option<&mut
        // Self>`'s or `(&mut Self, u32)`'s, generic code's box, made the
        // impl's own the same way, `other?.value`, and within what it gives,
        // made generic code's box (#686). Not of a writer's, whose
        // `Formatter` isn't a JS parameter (ADR 0054).
        let trait_env = self.tcx.param_env(item);
        let declared_sig = (self.tcx.fn_sig(item).instantiate_identity())
            .skip_normalization()
            .skip_binder();
        let impl_sig = (self.tcx.fn_sig(method).instantiate(self.tcx, instance_args))
            .skip_normalization()
            .skip_binder();
        let within = |declared: Option<&Ty<'tcx>>, actual: Option<&Ty<'tcx>>| {
            let (declared, actual) = (adapted_part(*declared?)?, adapted_part(*actual?)?);
            Some(self.within(declared, trait_env, actual))
        };
        let plain = self.formatter_param(method).is_none();
        let within_params = (0..count)
            .map(|i| match plain {
                true => within(declared_sig.inputs().get(i), impl_sig.inputs().get(i)),
                false => None,
            })
            .collect();
        // A `&mut` generic code has in a box by the trait's signature, `&mut
        // Self::Target` of a `Vec`, which the method gives as the object.
        let result_cell = self.boxed_in_call(item, declared_sig.output(), args).is_some();
        EntryForms {
            unboxed,
            within: within_params,
            result_cell,
            result_within: within(Some(&declared_sig.output()), Some(&impl_sig.output())),
        }
    }

    fn default_method(&mut self, imp: DefId, id: DefId, args: ty::GenericArgsRef<'tcx>) -> R<Expr> {
        let span = self.tcx.def_span(id);
        let args = default_args(self.tcx, imp, id, args);
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
        // Its body is in the impl's terms (`default_args`), so it drops as
        // the impl does, with the impl's drops (ADR 0098); and its own type
        // parameters with those its caller gives after its dictionaries, as
        // the trait declares them (ADR 0163, #802), each by its type in those
        // terms, which no drop of the impl's is (#804, #887).
        let (mut drops, mut slots) = self.impl_drops_for_default();
        let mut own_drops = Vec::new();
        for index in self.own_drops_taken(id, id) {
            let ty = drop_slot(self.tcx, self.slots(), id, index);
            let name = self.fresh(&format!("drop{}", drop_word(self.tcx, ty)));
            let in_impl = ty::EarlyBinder::bind(self.tcx, ty)
                .instantiate(self.tcx, args)
                .skip_normalization();
            if !crate::lower::recognition::known_by_bounds(in_impl) {
                continue;
            }
            drops.insert(in_impl, crate::lower::drops::GivenDrop::lent(name.clone()));
            own_drops.push((index, name, in_impl));
        }
        // And each it could be given, of its own where clause too (ADR 0489).
        slots.extend(
            (drop_items(self.tcx, self.krate.foreign, id).into_iter()).filter_map(|own| match *ty::EarlyBinder::bind(
                self.tcx, own,
            )
            .instantiate(self.tcx, args)
            .skip_normalization()
            .kind()
            {
                ty::Alias(_, alias) => Some(alias),
                _ => None,
            }),
        );
        // Its body in the impl's terms names the impl's const parameters,
        // `CAP` of an `ArrayVec<T, CAP>`, the impl's, and its own, given after
        // its arguments, numbered after the impl's (ADR 0135, `default_args`).
        let mut consts = self.impl_const_params();
        let generics = self.tcx.generics_of(id);
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
            // As the impl's terms number it (`default_args`).
            if let Some(there) = args.get(index as usize).and_then(|arg| arg.as_const())
                && let ty::ConstKind::Param(there) = there.kind()
            {
                consts.push((there.index, Expr::var(&name)));
            }
            const_names.push((index, name));
        }
        // The facts its caller gives of its own type parameters, `KMap`,
        // after its dictionaries (ADR 0466).
        let mut facts: Vec<(Ty<'tcx>, TypeFact, Expr)> = Vec::new();
        let mut fact_names = Vec::new();
        for &(index, fact) in self.krate.type_facts.get(&id).into_iter().flatten() {
            if index < generics.parent_count as u32 {
                continue;
            }
            let of = drop_slot(self.tcx, self.slots(), id, index);
            if self
                .given
                .type_facts
                .iter()
                .chain(&facts)
                .any(|&(at, given, _)| at == of && given == fact)
            {
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
            fact_names.push(((index, fact), name));
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
            typing_env: default_typing_env(self.tcx, imp, id, args),
            drops,
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
        let class_default = (body.mir.as_ref()).map(|mir| {
            let mut mir = default_mir(self.tcx, mir, args, default_typing_env(self.tcx, imp, id, args));
            if crate::lower::classes::is_class_trait(self.tcx, self.tcx.parent(id)) {
                mir.read_fields(self.tcx, ty::TypingEnv::fully_monomorphized());
            }
            mir
        });
        // What its caller through the dictionary gives, in its trait's forms,
        // made what its body, in the impl's terms, takes (`entry_forms`):
        // `self.value` of a `&mut Self` generic code has in a box (#803).
        // Of a method: a constant's initializer takes nothing.
        let forms = match self.tcx.def_kind(id) {
            DefKind::AssocFn => {
                let count = (self.tcx.fn_sig(id).instantiate_identity())
                    .skip_normalization()
                    .skip_binder()
                    .inputs()
                    .len();
                Some(self.entry_forms(id, (id, args), args, count, Some(imp)))
            }
            _ => None,
        };
        if (forms.as_ref())
            .is_some_and(|forms| forms.result_cell || forms.result_within.as_ref().is_some_and(Within::changes))
        {
            return Err(self.unsupported(span, "a copied default giving back a `&mut` its impl holds otherwise"));
        }
        // Each parameter it makes another's: named in its body's names, before
        // its body is, so no name of its body's is one of them, a box of a
        // `self` as the naming owner names one (`self_box_name`).
        let count = forms.as_ref().map_or(0, |forms| forms.unboxed.len());
        let words: Vec<Option<&str>> = (0..count)
            .map(|i| {
                let forms = forms.as_ref()?;
                let made = forms.unboxed[i] > 0
                    || (forms.within[i].as_ref())
                        .is_some_and(|within| within.adapted(&Expr::undefined(), true).is_some());
                let receiver = || {
                    let sig = self.tcx.fn_sig(id).instantiate(self.tcx, args).skip_normalization();
                    sig.skip_binder().inputs()[0]
                };
                made.then(|| match i {
                    0 => crate::lower::analysis::self_box_name(self.tcx, receiver()),
                    _ => "arg",
                })
            })
            .collect();
        let ((signature, used, given_names), hoisted) = self.lowered_in(body, id, nested, |cx| {
            let given_names: Vec<Option<String>> = (words.iter()).map(|word| word.map(|word| cx.fresh(word))).collect();
            let signature = match class_default.as_ref().or(body.mir.as_ref()) {
                Some(mir) => cx.mir_signature(body, mir, &mut rest)?,
                None => return Err(cx.unsupported(span, "this default, without its MIR")),
            };
            Ok((signature, cx.used_drops(), given_names))
        })?;
        // After its arguments, what its caller gives of its own, in
        // `evidence_slots`' order, as `own_evidence` gives them: of its
        // trait's parameters no drops, which its impl's are, made here.
        let (mut params, is_async) = signature;
        let mut prologue = Vec::new();
        for (i, param) in params.iter_mut().enumerate().take(count) {
            let (Some(forms), Some(given)) = (forms.as_ref(), given_names[i].clone()) else {
                continue;
            };
            let js::Pattern::Name(name) = param else {
                continue;
            };
            let mut value = Expr::var(&given);
            for _ in 0..forms.unboxed[i] {
                value = Expr::held(value);
            }
            if let Some(made) = (forms.within[i].as_ref()).and_then(|within| within.adapted(&value, true)) {
                value = made;
            }
            if value != Expr::var(&given) {
                // A `const` but of a parameter its body assigns again, `mut`.
                let mutable = (class_default.as_ref()).is_some_and(|mir| {
                    mir.body.local_decls[rustc_middle::mir::Local::from_usize(i + 1)].mutability
                        == rustc_hir::Mutability::Mut
                });
                prologue.push(
                    match mutable {
                        true => StmtKind::Let(name.clone(), Some(value)),
                        false => StmtKind::Const(name.clone(), value),
                    }
                    .at(js::Span::NONE),
                );
                *param = given.into();
            }
        }
        let named = |names: &[(u32, String)], at: u32| names.iter().find(|&&(i, _)| i == at).map(|(_, n)| n.clone());
        // Its own drops last, those after the last it uses left out, as its
        // caller's `undefined`s at the end are.
        let own_used = |key: Ty<'tcx>| used.contains(&key);
        let mut unused = 0;
        for slot in self.evidence_slots(id, Scope::Own) {
            let name = match slot {
                Slot::Location => location.clone(),
                Slot::Const(index) => named(&const_names, index),
                Slot::Bound(bound) => own_params.iter().find(|&&(b, _)| b == bound).map(|(_, n)| n.clone()),
                // By its fact too: a parameter's `Size` and `Align` are two.
                Slot::Fact(index, fact) => (fact_names.iter())
                    .find(|&&(at, _)| at == (index, fact))
                    .map(|(_, name)| name.clone()),
                Slot::Drop(index) => {
                    // None, of a crate whose values drop nothing.
                    let Some((_, name, key)) = own_drops.iter().find(|(at, _, _)| *at == index) else {
                        continue;
                    };
                    unused = match own_used(*key) {
                        true => 0,
                        false => unused + 1,
                    };
                    Some(name.clone())
                }
            };
            params.push(name.expect("each of a default's own slots named").into());
        }
        params.truncate(params.len() - unused);
        rest.splice(0..0, hoisted);
        let mut out = prologue;
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
        // Nor one whose trait's signature names an associated type of
        // `Self` bound by `Iterator`, `fn numbers(&self) -> Self::Iter`,
        // whose entry gives generic code its form of it, a JS iterator of a
        // `Range` (#1032).
        let tcx = self.tcx;
        let iterator = |item: DefId| {
            (tcx.item_bounds(item).instantiate_identity().skip_normalization().iter())
                .filter_map(|clause| clause.as_trait_clause())
                .any(|p| {
                    crate::lower::recognition::is_std_def(tcx, p.def_id(), crate::lower::recognition::StdItem::Iterator)
                        || crate::lower::recognition::is_iterator_extension(tcx, p.def_id())
                })
        };
        let names_own_type = tcx.def_kind(method) == DefKind::AssocFn
            && (tcx.fn_sig(method).instantiate_identity().skip_normalization().skip_binder().inputs_and_output)
                .iter()
                .any(|ty| {
                    ty.walk().any(|part| {
                        matches!(part.as_type().map(|t| *t.kind()), Some(ty::Alias(_, alias))
                            if matches!(alias.kind, ty::Projection { def_id } if iterator(def_id)) && alias.self_ty().is_param(0))
                    })
                });
        let as_entry = self.tcx.generics_of(method).own_params.is_empty()
            && !names_own_type
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

//! The dictionary of an iterator JS holds as no object, given generic code
//! that holds it in a place of its own, `{ value: x }` (`Form::Placed`,
//! #2255). Split from dictionaries.rs by concept, as a file is at most 1500
//! lines.

use super::super::*;
use crate::generic_facts::Placed;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The dictionary of `bound`, generic code `def_id`'s `declared` bound
    /// instantiated, where generic code holds what it's of, or what it makes,
    /// in a place of its own (`Form::Placed`, its one answer of each type an
    /// entry takes and gives): its own, each entry given what each place
    /// holds where generic code gives one, and giving back in a place what
    /// generic code holds so, `$throughPlace(triClone, { clone: [1] },
    /// ["clone"])`, `$throughBox`'s `1` of each place; its supertraits' and
    /// its associated types' bounds' entries so too, `IntoIter`'s `Iterator`
    /// of an `IntoIterator`. `None` where no entry crosses so. An entry that
    /// takes or gives one inside another value is refused; of an iterator's
    /// trait, whose dictionary has none of the trait's methods that give an
    /// adapter of it, those left out.
    pub(crate) fn placed_dictionary(
        &mut self,
        def_id: DefId,
        (declared, bound): (ty::TraitRef<'tcx>, ty::TraitRef<'tcx>),
        span: Span,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let holds_placed = |cx: &Self, i: Ty<'tcx>| {
            i.walk()
                .filter_map(|part| part.as_type())
                .any(|t| cx.placed_iterator(t))
        };
        if !bound.args.types().any(|ty| holds_placed(self, ty)) {
            return Ok(None);
        }
        let what = format!(
            "`{}` of a `{}` generic code holds in a place of its own (#2255)",
            tcx.def_path_str(bound.def_id),
            bound.self_ty()
        );
        let iterator_trait = crate::recognition::is_iterator_trait(tcx, bound.def_id);
        let there = ty::TypingEnv::new(tcx.param_env(def_id), ty::TypingMode::non_body_analysis());
        let normalized = |env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>| {
            let ty = tcx.erase_and_anonymize_regions(ty);
            tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let mut dictionary: Option<Expr> = None;
        let (mut given, mut made) = (Vec::new(), Vec::new());
        let (mut into, mut back) = (Vec::new(), Vec::new());
        for (method, shapes) in self.placed_entries(def_id, (declared, bound))? {
            let Some(EntryShapes { holder, entry }) = shapes else {
                match iterator_trait {
                    true => continue,
                    false => return Err(self.unsupported(span, &what)),
                }
            };
            let name = bindings::trait_method_name(tcx, bound.def_id, tcx.item_name(method).as_str());
            let dictionary = match &dictionary {
                Some(made) => made,
                None => dictionary.insert(self.dictionary(bound, span)?),
            };
            // Of one written whole here, `{ next, sizeHint }`, its own
            // entries alone; of another, every method's, which
            // `$throughPlace` skips where it has none.
            if let js::ExprKind::Object(props, ..) = &dictionary.kind
                && !(props.iter()).any(|prop| matches!(prop, Prop::Field(field, _) if *field == name))
            {
                continue;
            }
            shaped_entry(&name, holder, (&mut given, &mut made));
            shaped_entry(&name, entry, (&mut into, &mut back));
        }
        // Its supertraits', and its associated types' bounds', each placed
        // where it crosses so.
        let mut props = Vec::new();
        let declared_parts = (held_supertraits(tcx, self.krate.foreign, declared.def_id, declared.args).into_iter())
            .chain(item_bounds(tcx, declared.def_id, declared.args));
        let parts = (held_supertraits(tcx, self.krate.foreign, bound.def_id, bound.args).into_iter())
            .chain(item_bounds(tcx, bound.def_id, bound.args));
        let of = |tr: ty::TraitRef<'tcx>, env: ty::TypingEnv<'tcx>| {
            let self_ty: ty::GenericArg<'tcx> = normalized(env, tr.self_ty()).into();
            ty::TraitRef::new(tcx, tr.def_id, std::iter::once(self_ty).chain(tr.args.iter().skip(1)))
        };
        for ((name, theirs), (_, ours)) in declared_parts.zip(parts) {
            if let Some(part) = self.placed_dictionary(def_id, (of(theirs, there), of(ours, self.typing_env)), span)? {
                props.push(Prop::Field(
                    name,
                    Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(part)).at(js::Span::NONE)]),
                ));
            }
        }
        if [&given, &made, &into, &back, &props]
            .iter()
            .all(|props| props.is_empty())
        {
            return Ok(None);
        }
        let dictionary = match dictionary {
            Some(made) => made,
            None => self.dictionary(bound, span)?,
        };
        // Its entries given in the place their own generic code holds,
        // `$intoPlace`'s, inside what reads generic code's, `$throughPlace`'s.
        let wrapped = |dictionary: Expr, helper: &'static str, given: Vec<Prop>, made: Vec<Prop>| match given.is_empty()
            && made.is_empty()
        {
            true => dictionary,
            false => {
                let mut args = vec![dictionary, Expr::object(given)];
                args.extend((!made.is_empty()).then(|| Expr::object(made)));
                Expr::call_helper(helper, args)
            }
        };
        let placed = wrapped(
            wrapped(dictionary, "$intoPlace", into, back),
            "$throughPlace",
            given,
            made,
        );
        if props.is_empty() {
            return Ok(Some(placed));
        }
        props.insert(0, Prop::Spread(placed));
        Ok(Some(Expr::object(props)))
    }
    /// Each method of `bound`, generic code `def_id`'s `declared` bound
    /// instantiated, whose entry crosses where generic code holds a value in
    /// a place of its own (`Form::Placed`): what of each argument it takes
    /// and of its value it gives is so, as `def_id` holds them, and as the
    /// method's own generic code does, where they differ (`EntryShapes`);
    /// or `None` of one that takes or gives one inside another value
    /// (`placed_within`).
    fn placed_entries(
        &self,
        def_id: DefId,
        (declared, bound): (ty::TraitRef<'tcx>, ty::TraitRef<'tcx>),
    ) -> R<Vec<(DefId, Option<EntryShapes>)>> {
        let tcx = self.tcx;
        let there = ty::TypingEnv::new(tcx.param_env(def_id), ty::TypingMode::non_body_analysis());
        let normalized = |env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>| {
            let ty = tcx.erase_and_anonymize_regions(ty);
            tcx.try_normalize_erasing_regions(env, ty::Unnormalized::new_wip(ty))
                .unwrap_or(ty)
        };
        let methods: Vec<DefId> = (tcx.associated_items(bound.def_id).in_definition_order())
            .filter(|item| tcx.def_kind(item.def_id) == DefKind::AssocFn)
            .map(|item| item.def_id)
            .collect();
        let mut entries = Vec::new();
        for method in methods {
            let args_of = |tr: ty::TraitRef<'tcx>| {
                ty::GenericArgs::identity_for_item(tcx, method).rebase_onto(tcx, tr.def_id, tr.args)
            };
            let sig_of = |tr: ty::TraitRef<'tcx>, env: ty::TypingEnv<'tcx>| -> Vec<Ty<'tcx>> {
                let sig = tcx.instantiate_bound_regions_with_erased(
                    tcx.fn_sig(method).instantiate(tcx, args_of(tr)).skip_normalization(),
                );
                (sig.inputs_and_output.iter()).map(|ty| normalized(env, ty)).collect()
            };
            let (theirs, ours) = (sig_of(declared, there), sig_of(bound, self.typing_env));
            let identity = erased_sig(tcx, method);
            // What the method called is, as a call of it here resolves it,
            // whose generic code takes what it's given as a call of it here
            // gives it (`call_callee`, `boxed_args`): an impl's own, or the
            // trait's provided one. Not a conversion's, which takes it as
            // its caller holds it.
            let callee = match self.is_conversion(bound) {
                true => None,
                false => self.call_callee(method, args_of(bound))?,
            };
            let callee_sig = callee.map(|(id, _)| erased_sig(tcx, id));
            let mut holder = Vec::new();
            let mut entry = Vec::new();
            let mut nested = false;
            for (at, ((&d, &i), trait_ty)) in theirs
                .iter()
                .zip(&ours)
                .zip(identity.inputs_and_output.iter())
                .enumerate()
            {
                let held_mut = matches!((*d.kind(), *i.kind()), (ty::Ref(_, d, Mutability::Mut), ty::Ref(_, i, Mutability::Mut))
                    if self.holds_in_place((def_id, d, i)));
                let boxed = held_mut
                    && matches!(*trait_ty.kind(), ty::Ref(_, pointee, Mutability::Mut)
                        if self.mut_ref_form_in(pointee, tcx.param_env(method)).is_generic_box());
                holder.push(match boxed {
                    true => Some(Placed::Itself),
                    false => self.placed_shape((def_id, d, i)),
                });
                nested |= self.placed_within(def_id, d, i).is_some();
                let callee_crossing = callee.zip(callee_sig.as_ref()).and_then(|((id, args), sig)| {
                    let d = *sig.inputs_and_output.get(at)?;
                    Some((id, d, self.instantiated(d, args)))
                });
                entry.push(callee_crossing.and_then(|crossing| {
                    nested |= self.placed_within(crossing.0, crossing.1, crossing.2).is_some();
                    self.placed_shape(crossing)
                }));
            }
            if nested {
                entries.push((method, None));
                continue;
            }
            // Where both hold one alike, given as it's held.
            for (holder, entry) in holder.iter_mut().zip(&mut entry) {
                if holder == entry {
                    (*holder, *entry) = (None, None);
                }
            }
            if holder.iter().chain(&entry).any(Option::is_some) {
                let split = |mut shapes: Vec<Option<Placed>>| {
                    let gives = shapes.pop().expect("a signature's output");
                    (shapes, gives)
                };
                entries.push((
                    method,
                    Some(EntryShapes {
                        holder: split(holder),
                        entry: split(entry),
                    }),
                ));
            }
        }
        Ok(entries)
    }
}

/// What of each argument an entry takes, and of its value it gives, is in a
/// place of its own where it differs (`placed_entries`): as generic code
/// that holds the dictionary holds it, which `$throughPlace` reads, and as
/// the method's own generic code does, which `$intoPlace` makes, as a call
/// of it gives it (`boxed_args`), where a caller holds it as it is.
struct EntryShapes {
    holder: Shapes,
    entry: Shapes,
}

/// What of each argument it takes, and of its value it gives, is in a place.
type Shapes = (Vec<Option<Placed>>, Option<Placed>);

/// An entry `name`'s shapes, what it `takes` to the last one in a place, as
/// the rest are given as they are, and what it `gives`, in `given`'s and
/// `made`'s objects (`$throughPlace`'s, `$intoPlace`'s).
fn shaped_entry(name: &str, (takes, gives): Shapes, (given, made): (&mut Vec<Prop>, &mut Vec<Prop>)) {
    if let Some(last) = takes.iter().rposition(Option::is_some) {
        let codes = (takes[..=last].iter())
            .map(|shape| shape.as_ref().map_or_else(|| Expr::int(0), Placed::code))
            .collect();
        given.push(Prop::Field(name.into(), Expr::array(codes)));
    }
    if let Some(gives) = gives {
        made.push(Prop::Field(name.into(), gives.code()));
    }
}

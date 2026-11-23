//! What a function takes after its own parameters, and in which order: the
//! one list its body names (`evidence_params`) and each call gives
//! (`evidence_args_with`, `method_evidence`, `own_evidence`), each slot of
//! it given by that consumer's own rule of its value.

use super::*;
use crate::temporary_names::{Base, Role};
use rustc_span::def_id::DefId;

/// One of what a function takes after its own parameters.
#[derive(Clone, Copy)]
pub(crate) enum Slot<'tcx> {
    /// Where it's called, of one that tracks its caller (ADR 0427).
    Location,
    /// A const parameter's value, `N`, by its index (ADR 0107).
    Const(u32),
    /// A bound's dictionary, `IIterator`, of the function's own terms (ADR
    /// 0049).
    Bound(ty::TraitRef<'tcx>),
    /// A fact it asks of a type parameter, `TSize`, by its slot (ADR 0145).
    Fact(u32, TypeFact),
    /// A drop function of a type parameter or an associated type, `dropT`,
    /// by its slot (ADR 0098): given last, so one with nothing to drop is
    /// left out at the end.
    Drop(u32),
}

/// `{ entry: helper }`, a default dictionary (`default_dictionary`).
pub(crate) fn plain_dictionary((entry, helper): (&str, &'static str)) -> Expr {
    Expr::object(vec![js::Prop::Field(entry.to_string(), Expr::helper(helper))])
}

/// Whether `e` is `plain_dictionary(default)` as written.
pub(crate) fn is_plain_dictionary(e: &Expr, (entry, helper): (&str, &str)) -> bool {
    matches!(&e.kind, js::ExprKind::Object(props, _)
        if matches!(props.as_slice(), [js::Prop::Field(name, value)] if name == entry && value.helper_name() == Some(helper)))
}

/// Whose evidence: the whole function's, or a trait's generic method's own,
/// given through a dictionary, whose impl's and trait's the dictionary has
/// (ADR 0106).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Scope {
    Whole,
    Own,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The dictionary `id`, a std body compiled into `@rust-js/std`, takes
    /// for its bound `tr` where it's given none: a JS iterator's own, `{
    /// next: $next }` of an `Iterator` bound, `{ next_back: $nextBack }` of a
    /// `DoubleEndedIterator` one, as its entry and its helper. A call whose
    /// last argument is it leaves it out (ADR 0995): the one rule, of its
    /// parameter and of its calls.
    pub(crate) fn default_dictionary(&self, id: DefId, tr: DefId) -> Option<(&'static str, &'static str)> {
        if !crate::recognition::is_compiled_std(self.tcx, id) {
            return None;
        }
        if is_std_def(self.tcx, tr, StdItem::Iterator) {
            Some(("next", "$next"))
        } else if self.recognition().double_ended_iterator() == Some(tr) {
            Some(("next_back", "$nextBack"))
        } else {
            None
        }
    }

    /// The entry of `ty`'s own of `id`, a provided method of a trait with no
    /// parameters but `Self`, `Iterator::partition`, which a dictionary or a
    /// value carries for generic code's `$provided` (#2414, #2421): a
    /// function of what the call gives, its values, then the trait method's
    /// own evidence (`own_slots_through`, `own_evidence`'s list), which gives
    /// the impl's function what it takes after its values, `ty`'s impl's
    /// own evidence, `LIterator` of a `W<L>`'s, then those of the given it
    /// asks, by `evidence_args_given`: so an impl with type parameters of
    /// its own, or whose method asks fewer bounds than the trait's, is
    /// given what it takes. `counterIterator_partition` itself where that's
    /// what it's given. A const's or a fact's slot, which none of
    /// `Iterator`'s methods has, refused.
    pub(crate) fn provided_entry(&mut self, id: DefId, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        let trait_parent = tcx.generics_of(id).parent_count as u32;
        if trait_parent != 1 {
            return Err(self.unsupported(
                span,
                "an own provided method of a trait with parameters of its own (#2421)",
            ));
        }
        let args = ty::GenericArgs::for_item(tcx, id, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            _ if param.index == 0 => ty.into(),
            _ => tcx.mk_param_from_def(param),
        });
        let instance =
            (self.resolve_instance(id, args)?).ok_or_else(|| self.unsupported(span, "this implementation"))?;
        let method = instance.def_id();
        let impl_parent = tcx.generics_of(method).parent_count as u32;
        let names = crate::signatures::taken_parameter_names(tcx, id, None, false);
        let mut params: Vec<js::Name> = (names.into_iter().enumerate())
            .map(|(at, name)| match (at, name) {
                (0, _) | (_, None) => self.fresh(Base::Named(if at == 0 { "iterator" } else { "arg" })),
                (_, Some(name)) => self.fresh(Base::Named(&name)),
            })
            .collect();
        let mut values: Vec<Expr> = params.iter().map(Expr::var).collect();
        let (mut bounds, mut drops, mut location) = (Vec::new(), Vec::new(), None);
        for slot in self.own_slots_through(id, args) {
            let name = match slot {
                Slot::Location => {
                    let name = self.fresh(Role::Location);
                    location = Some(Expr::var(&name));
                    name
                }
                Slot::Bound(declared) => {
                    // Of `ty`'s terms, `B: Extend<u32>`, as the impl's
                    // method names it, not `Extend<Self::Item>`.
                    let tr = ty::EarlyBinder::bind(tcx, declared)
                        .instantiate(tcx, args)
                        .skip_normalization();
                    let tr = (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(tr)))
                        .unwrap_or(tr);
                    let name = self.fresh(Base::Named(&bound_word(tcx, tr)));
                    bounds.push((tr, Expr::var(&name)));
                    name
                }
                // A drop of a type parameter of the method's own, which the
                // impl's method names after its impl's own.
                Slot::Drop(index) => {
                    match crate::generic_facts::type_parameter_of(self.drop_slot_of(id, index, args)) {
                        Some(of) if of >= trait_parent => {
                            let dropped = self.drop_slot_of(id, index, args);
                            let name = self.fresh(Base::Prefixed(Role::Drop, &drop_word(tcx, dropped)));
                            drops.push((impl_parent + of - trait_parent, Expr::var(&name)));
                            name
                        }
                        _ => {
                            return Err(self.unsupported(
                                span,
                                "an own provided method's drop of what isn't its own type parameter (#2421)",
                            ));
                        }
                    }
                }
                Slot::Const(_) | Slot::Fact(..) => {
                    return Err(self.unsupported(span, "an own provided method given a const or a fact (#2421)"));
                }
            };
            params.push(name);
        }
        let location = location.unwrap_or_else(|| self.location_at(tcx.def_span(method)));
        let given = GivenEvidence {
            bounds: &bounds,
            drops: &drops,
        };
        values.extend(self.evidence_args_given(method, instance.args, location, given, span)?);
        let call = Expr::call(self.fn_ref(method), values);
        let entry = Expr::arrow(
            params.into_iter().map(Into::into).collect(),
            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
        );
        Ok(crate::passed_on::passed_on(&entry, crate::passed_on::Taker::Made, &[]).unwrap_or(entry))
    }

    /// `facts_through_dictionary` of the lowering's.
    pub(crate) fn facts_through_dictionary(&self, id: DefId, base: Vec<(u32, TypeFact)>) -> Vec<(u32, TypeFact)> {
        facts_through_dictionary(self.tcx, self.krate.foreign, id, base)
    }

    /// `evidence_slots`' own of `id`, a trait's method, called with
    /// `generic_args` through a dictionary: of a library's impl whose
    /// dictionary its JS exports, the facts its JS of the method takes, as
    /// the library's contract lists them, never ones this crate's analysis
    /// makes (#1612); of another's, `facts_through_dictionary`'s.
    pub(crate) fn own_slots_through(&self, id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Vec<Slot<'tcx>> {
        let slots = self.evidence_slots(id, Scope::Own);
        let tr = ty::TraitRef::from_assoc(self.tcx, self.tcx.parent(id), generic_args);
        let parent = self.tcx.generics_of(id).parent_count as u32;
        let facts = match (self.exported_impl(tr)).and_then(|_| self.krate.foreign.trait_method_facts(id)) {
            Some(facts) => facts.into_iter().filter(|&(index, _)| index >= parent).collect(),
            None => {
                let base = (slots.iter())
                    .filter_map(|slot| match *slot {
                        Slot::Fact(index, fact) => Some((index, fact)),
                        _ => None,
                    })
                    .collect();
                self.facts_through_dictionary(id, base)
            }
        };
        let at = (slots.iter())
            .position(|slot| matches!(slot, Slot::Fact(..) | Slot::Drop(_)))
            .unwrap_or(slots.len());
        let mut own: Vec<Slot<'tcx>> = slots
            .into_iter()
            .filter(|slot| !matches!(slot, Slot::Fact(..)))
            .collect();
        own.splice(at..at, facts.into_iter().map(|(index, fact)| Slot::Fact(index, fact)));
        own
    }

    /// Whether `id` is given a dictionary of its `Self`'s `Iterator`, as a
    /// compiled std adapter is, `take`'s `SelfIterator`, which it steps its
    /// `Self` by. One that isn't, a compiled consumer, `partition`'s, steps
    /// it as a JS iterator, as generic code holds one, so it's given one
    /// (`trait_call`, ADR 0061, #2387).
    pub(crate) fn given_self_iterator(&self, id: DefId) -> bool {
        let iterator = crate::recognition::std_item(self.tcx, crate::recognition::StdItem::Iterator);
        (self.evidence_slots(id, Scope::Whole).iter()).any(|slot| {
            matches!(slot, Slot::Bound(bound) if bound.def_id == iterator && bound.self_ty() == self.tcx.types.self_param)
        })
    }

    /// What `id` takes after its own parameters, of `scope`, in order: its
    /// caller's location, its const parameters, its bounds' dictionaries,
    /// the facts it asks, then its drops. The one order.
    pub(crate) fn evidence_slots(&self, id: DefId, scope: Scope) -> Vec<Slot<'tcx>> {
        let tcx = self.tcx;
        let mut slots = Vec::new();
        if self.takes_caller_location(id) {
            slots.push(Slot::Location);
        }
        let consts = match scope {
            Scope::Whole => const_params(tcx, id).into_iter().map(|param| param.index).collect(),
            Scope::Own => own_const_params(tcx, id),
        };
        slots.extend(consts.into_iter().map(Slot::Const));
        let bounds = match scope {
            Scope::Whole => bounds(tcx, self.krate.foreign, self.krate.given, id),
            Scope::Own => own_bounds(tcx, self.krate.foreign, self.krate.given, id),
        };
        slots.extend(bounds.into_iter().map(Slot::Bound));
        // Of a method's own, what it asks of its own type parameters (ADR
        // 0466).
        let parent = tcx.generics_of(id).parent_count as u32;
        let facts = self.facts_asked(id).into_iter();
        slots.extend(
            facts
                .filter(|&(index, _)| scope == Scope::Whole || index >= parent)
                .map(|(i, f)| Slot::Fact(i, f)),
        );
        // Of a method's own, a drop of each of its own type parameters the
        // trait declares one for (ADR 0163).
        let drops = match scope {
            Scope::Whole => self.drops_taken(id),
            Scope::Own => own_drop_params(tcx, self.krate.foreign, id),
        };
        slots.extend(drops.into_iter().map(Slot::Drop));
        slots
    }
}

/// The facts `id`, a trait's method, takes through a dictionary of the
/// trait that isn't a library's exported one: `base`, what this crate's
/// analysis says, and of a library's trait each fact of the method's own
/// type parameters its contract lists, in its place by index, as the
/// library's generic code gives them through any dictionary of the trait, a
/// consumer's copy of the library's impl (ADR 0914) or a consumer's own
/// (#2285). The one answer a call through such a dictionary, the
/// dictionary's entry and the analysis that relays a call's asks to its
/// caller all ask.
pub(crate) fn facts_through_dictionary(
    tcx: TyCtxt<'_>,
    foreign: &crate::library::Foreign<'_, '_>,
    id: DefId,
    base: Vec<(u32, TypeFact)>,
) -> Vec<(u32, TypeFact)> {
    let Some(contract) = foreign.trait_method_facts(id) else {
        return base;
    };
    let generics = tcx.generics_of(id);
    let own = generics.parent_count as u32..generics.count() as u32;
    let mut facts = base;
    for (index, fact) in contract.into_iter().filter(|(index, _)| own.contains(index)) {
        if facts.contains(&(index, fact)) {
            continue;
        }
        let at = (facts.iter()).position(|&(at, _)| at > index).unwrap_or(facts.len());
        facts.insert(at, (index, fact));
    }
    facts
}

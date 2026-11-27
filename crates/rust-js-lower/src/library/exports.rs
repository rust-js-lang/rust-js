//! What a library's manifest says of it (ADR 0100): the items another
//! crate can reach that its JS exports, the impls it calls through them,
//! the bindings its modules hold, and each fact its consumers read.

use super::*;

/// A crate's canonical name, as its manifest says it, `name` (the
/// crate's own of `LOCAL_CRATE`): what a fact naming another crate's item
/// keys it by, the export net's records' `OfferedDef` (ADR 1204).
pub fn canonical_crate_name(tcx: TyCtxt<'_>, krate: rustc_span::def_id::CrateNum) -> String {
    tcx.crate_name(krate).to_string()
}

/// The items another crate can reach that its JS exports, `exported`
/// (`exported_names`), and the impls it calls through them; and the
/// bindings its modules hold, which a consumer naming one through its
/// module reads through it (ADR 0633). What a consumer reaches that it
/// doesn't export, the consumer compiles (ADR 0914, amended).
pub fn exports<'tcx>(
    tcx: TyCtxt<'tcx>,
    (functions, exported, internal): (&IndexMap<DefId, FnInfo>, &Exported, &[super::internal::InternalExport]),
    contracts: &Contracts<'_, 'tcx>,
    (trait_impls, held): (&[DefId], &IndexMap<LocalModId, Vec<HeldBinding>>),
    dependencies: &Dependencies,
) -> Library {
    let Contracts {
        // What each type parameter is asked, `type_params`'s.
        drop_params: _,
        elided,
        shown_params: _,
        dyn_debug_shown,
        written,
        makes_fmt_dyns,
        no_drops: _,
        type_facts: _,
        // `associated`'s.
        carried_super: _,
        built_slots: _,
        given: _,
        failing,
        writes_may_fail,
        caller_locations,
        reads_arguments,
        keeps_capacity,
        writers_take_options,
        debug_hex,
        reads_formatter_options,
        merges_identities,
        untold_locations,
        bound,
        makes_dropping_anys,
        makes_dropping_errors,
        makes_dropping_fn_dyns,
        makes_dropping_iterators,
        drops_any,
        prints,
        program,
        counted,
        changed,
        ref_cells: _,
        fields: _,
        lazy_locals: _,
        at_hand_keys: _,
        variables,
        identities,
        keyed,
        eq_by_address,
        gives_argument,
        displays_value,
        built,
        cloned_closures,
        owned_by_return,
        nulled_params,
        split,
        js_reached,
    } = contracts;
    let crate_name = tcx.crate_name(LOCAL_CRATE);
    // What its module exports it by: its type's object's name, a
    // namespace's method by its property, which is the function it passes
    // its parameters on to where it's that (#596), else its own name.
    let export_of = |id: DefId, info: &FnInfo| match crate::classes::member(tcx, id) {
        Some(crate::classes::Member::Namespace) => crate::classes::property(tcx, id),
        _ => info.owner.clone().unwrap_or_else(|| info.name.clone()),
    };
    let mut items: Vec<_> = functions
        .iter()
        .filter(|&(&id, info)| {
            own(tcx, id)
                && reachable(tcx, id)
                && exported.contains(&(module_path(tcx, info.module), export_of(id, info)))
                // Not a method of a trait of classes that gives its field,
                // which has no JS of its own (rd304, #1032).
                && !crate::classes::gives_its_field(tcx, id)
        })
        .map(|(&id, info)| Item {
            key: key(tcx, id),
            rust_path: format!("{crate_name}::{}", tcx.def_path_str(id)),
            module: module_path(tcx, info.module),
            export: export_of(id, info),
            member: info.owner.as_ref().map(|_| info.name.clone()),
            // As its own crate imports it (ADRs 0596, 0611, 0773).
            exported_as: (info.owner.is_none())
                .then(|| crate::default_exports::exported_as(tcx, id, info.exported()))
                .filter(|exported| *exported != info.name),
            internal: false,
            unchecked: split.unchecked(id).map(str::to_owned),
            checked: id.as_local().is_some_and(|local| js_reached.may_call(tcx, local)),
            calling: rust_js_program::library::Calling {
                params: type_params(tcx, id, contracts),
                facts: [
                    // An impl's dictionary, of a method that may fail, which a
                    // consumer calls through it (`Foreign::through_dictionary`).
                    (
                        ItemFact::Fails,
                        failing.contains(&id)
                            || (matches!(tcx.def_kind(id), rustc_hir::def::DefKind::Impl { of_trait: true })
                                && (tcx.associated_item_def_ids(id).iter()).any(|method| failing.contains(method))),
                    ),
                    (ItemFact::TakesLocation, caller_locations.contains(&id)),
                    (ItemFact::Rest, crate::bindings::takes_rest(tcx, id)),
                    (ItemFact::Arguments, reads_arguments.functions.contains(&id)),
                    (ItemFact::This, reads_arguments.this.contains(&id)),
                    (
                        ItemFact::Lazy,
                        id.as_local()
                            .is_some_and(|local| contracts.lazy_locals.contains(&local)),
                    ),
                    (ItemFact::Variable, variables.contains(&id)),
                    (ItemFact::ReturnsJs, contracts.given.js_given.main(tcx, id).returns_js),
                    (ItemFact::AtHand, contracts.at_hand_keys.contains(&id)),
                ]
                .into_iter()
                .filter_map(|(fact, holds)| holds.then_some(fact))
                .collect(),
                owned_by_return: owned_by_return.of(id),
                quiet_by_return: owned_by_return.quiet_of(id),
                nulled_params: {
                    let mut nulled: Vec<u32> = (nulled_params.iter())
                        .filter(|&&(of, _)| of == id)
                        .map(|&(_, i)| i as u32)
                        .collect();
                    nulled.sort();
                    nulled
                },
                written_arguments: contracts.written.arguments.of_fn(id).collect(),
            },
        })
        .collect();
    // Its private state a consumer's compile of its functions may read,
    // by its internal name (#2017).
    items.extend(
        internal
            .iter()
            .map(|super::internal::InternalExport { id, alias, variable }| Item {
                key: key(tcx, *id),
                rust_path: format!("{crate_name}::{}", tcx.def_path_str(*id)),
                module: module_path(tcx, functions[id].module),
                export: functions[id].name.clone(),
                member: None,
                exported_as: alias.clone(),
                internal: true,
                unchecked: None,
                checked: false,
                calling: rust_js_program::library::Calling {
                    facts: [
                        (
                            ItemFact::Lazy,
                            id.as_local()
                                .is_some_and(|local| contracts.lazy_locals.contains(&local)),
                        ),
                        (ItemFact::Variable, *variable || variables.contains(id)),
                        (ItemFact::AtHand, contracts.at_hand_keys.contains(id)),
                    ]
                    .into_iter()
                    .filter_map(|(fact, holds)| holds.then_some(fact))
                    .collect(),
                    ..Default::default()
                },
            }),
    );
    items.sort_by(|a, b| a.rust_path.cmp(&b.rust_path).then_with(|| a.key.cmp(&b.key)));
    let mut impls: Vec<String> = trait_impls
        .iter()
        .filter(|&&id| own(tcx, id) && reachable(tcx, id))
        .map(|&id| key(tcx, id))
        .collect();
    impls.sort();
    let mut held: Vec<Held> = (held.iter())
        .flat_map(|(&module, bindings)| bindings.iter().map(move |binding| (module, binding)))
        .filter_map(|(module, binding)| {
            Some(Held {
                binding: key(tcx, binding.binding),
                holder: key(tcx, module.to_def_id()),
                rust_name: binding.ident.to_string(),
                module: module_path(tcx, module),
                export: binding.import()?.to_string(),
                name: binding.name.clone(),
            })
        })
        .collect();
    held.sort_by(|a, b| (&a.module, &a.rust_name).cmp(&(&b.module, &b.rust_name)));
    // What its modules' `pub use`s re-export of its others' items (rd704),
    // each its module's file exports by that name; not its root's, which
    // a consumer naming one through it imports by the package's name, or
    // from its home (ADR 1008).
    let mut reexported: Vec<Reexported> = (crate::reexports::reexported_items(tcx, functions).into_iter())
        .filter(|(module, _, _, export)| {
            !module.is_top_level_module() && exported.contains(&(module_path(tcx, *module), export.clone()))
        })
        .map(|(module, name, item, export)| Reexported {
            item: key(tcx, item),
            holder: key(tcx, module.to_def_id()),
            rust_name: name.to_string(),
            module: module_path(tcx, module),
            export,
        })
        .collect();
    reexported.sort_by(|a, b| (&a.module, &a.rust_name).cmp(&(&b.module, &b.rust_name)));
    // Its modules, whose namespaces a consumer's `pub use` of one
    // re-exports (#1420).
    let mut modules: Vec<NamedModule> = (tcx.hir_crate_items(()).definitions())
        .filter(|&id| tcx.def_kind(id) == rustc_hir::def::DefKind::Mod)
        .map(|id| NamedModule {
            key: key(tcx, id.to_def_id()),
            module: module_path(tcx, LocalModId::new_unchecked(id)),
        })
        .collect();
    modules.sort_by(|a, b| a.module.cmp(&b.module));
    Library {
        version: VERSION,
        name: crate_name.to_string(),
        crate_id: crate_id(tcx, LOCAL_CRATE),
        crate_identity: crate::recognition::crate_identity(tcx, LOCAL_CRATE),
        crate_hash: tcx.crate_hash(LOCAL_CRATE).to_string(),
        items,
        impls,
        // Those its consumers must be given too: not `@rust-js/std`, built
        // into the compiler they use (ADR 0561).
        libraries: {
            let mut names: Vec<String> = (dependencies.libraries.iter())
                .filter(|(_, library)| !library.builtin)
                .map(|(name, _)| name.clone())
                .collect();
            names.sort();
            names
        },
        inputs: Vec::new(),
        keeps_capacity: *keeps_capacity,
        writes_may_fail: *writes_may_fail,
        writers_take_options: *writers_take_options,
        debug_hex: *debug_hex,
        dyn_debug_pretty: dyn_debug_shown.pretty,
        dyn_debug_hex: dyn_debug_shown.hex,
        reads_formatter_options: sorted_keys(tcx, reads_formatter_options.iter()),
        untold_locations: {
            let mut untold = untold_locations.to_vec();
            untold.sort();
            untold.dedup();
            untold
        },
        // Its own: what it compiles of another library's is that one's.
        merges_identities: sorted_keys(tcx, merges_identities.iter().filter(|id| id.is_local())),
        bound: bound.to_vec(),
        makes_dropping_anys: *makes_dropping_anys,
        makes_dropping_errors: *makes_dropping_errors,
        makes_dropping_fn_dyns: *makes_dropping_fn_dyns,
        makes_dropping_iterators: *makes_dropping_iterators,
        drops_any: *drops_any,
        printing_types: sorted_keys(tcx, prints.types.iter().filter(|id| id.is_local()))
            .into_iter()
            .collect(),
        stopping_types: sorted_keys(tcx, prints.stopping.iter().filter(|id| id.is_local()))
            .into_iter()
            .collect(),
        stopping_where_caught: sorted_keys(tcx, prints.stopping_where_caught.iter())
            .into_iter()
            .collect(),
        leaves_unended: prints.unended,
        written_into: sorted_keys(tcx, written.local_adts(tcx).iter()),
        written_trait_arguments: {
            let mut of: Vec<(String, Vec<u32>)> = (written.arguments.trait_methods(tcx).into_iter())
                .map(|(id, params)| (key(tcx, id), params))
                .collect();
            of.sort();
            of
        },
        framed_writers: sorted_keys(
            tcx,
            (prints.framed.iter()).filter(|id| id.is_local() && tcx.trait_impl_of_assoc(**id).is_none()),
        ),
        dyn_written_into: written.dyn_written,
        makes_fmt_dyns: *makes_fmt_dyns && !written.dyn_written,
        dictionary_consts: program.as_ref().map(|(consts, _)| sorted_keys(tcx, consts.iter())),
        dyns_drop_nothing: program
            .as_ref()
            .map_or(Vec::new(), |(_, traits)| sorted_keys(tcx, traits.iter())),
        held,
        reexported,
        modules,
        elided: {
            let mut all: Vec<rust_js_program::library::Elided> = (elided.iter())
                .map(|(&id, shapes)| rust_js_program::library::Elided {
                    key: key(tcx, id),
                    shapes: shapes.iter().cloned().collect(),
                })
                .collect();
            all.sort_by(|a, b| a.key.cmp(&b.key));
            all
        },
        counted_rcs: {
            let mut shapes: Vec<RcShape> = match counted.all {
                true => vec![RcShape {
                    of: None,
                    args: Vec::new(),
                }],
                // Of a pointee whose callers are seen, its instances alone,
                // which its consumers count as it does (#1009).
                false => (counted.types.iter())
                    .filter(|ty| !counted.resolved.contains(*ty))
                    .map(|&ty| rc_shape(tcx, ty))
                    .collect(),
            };
            shapes.sort();
            shapes.dedup();
            shapes
        },
        // Its own: a library's whose function it hands on says its own, as
        // its consumers are given every library's contract (`libraries`).
        with_this_arguments: {
            let mut shapes: Vec<RcShape> = (reads_arguments.with_this.iter())
                .map(|&params| rc_shape(tcx, params))
                .collect();
            shapes.sort();
            shapes.dedup();
            shapes
        },
        changed: shapes(tcx, changed.mutated.iter().chain(changed.vecs).copied()),
        ref_cells: contracts.ref_cells.clone(),
        fields: contracts.fields.clone(),
        // What each of its own traits' methods asks of its own slots, as its
        // JS of each takes them (#1612).
        trait_methods: {
            let mut methods: Vec<rust_js_program::library::TraitMethod> = (contracts.type_facts.iter())
                .filter(|&(&id, facts)| {
                    id.is_local() && !facts.is_empty() && tcx.trait_of_assoc(id).is_some_and(|tr| tr.is_local())
                })
                .map(|(&id, facts)| rust_js_program::library::TraitMethod {
                    key: key(tcx, id),
                    facts: facts
                        .iter()
                        .map(|&(index, fact)| (index, fact.key().to_string()))
                        .collect(),
                })
                .collect();
            methods.sort_by(|a, b| a.key.cmp(&b.key));
            methods
        },
        // What its traits' dictionaries carry of their associated types,
        // `type_fact_params`' own entry of each (ADR 0145, amended).
        associated: {
            let mut associated: Vec<rust_js_program::library::Associated> = (contracts.type_facts.iter())
                .filter(|&(&id, _)| id.is_local() && tcx.def_kind(id) == rustc_hir::def::DefKind::AssocTy)
                .map(|(&id, facts)| rust_js_program::library::Associated {
                    key: key(tcx, id),
                    facts: facts.iter().map(|&(_, fact)| fact.key().to_string()).collect(),
                    carrier: None,
                })
                .collect();
            // And what its consumers' impls' carry of a std supertrait's
            // associated types, which its defaults over `Self` read (#990).
            for (&carrier, facts) in contracts.carried_super.iter().filter(|&(&id, _)| id.is_local()) {
                let mut by_assoc: IndexMap<DefId, Vec<String>> = IndexMap::default();
                for &(assoc, fact) in facts {
                    by_assoc.entry(assoc).or_default().push(fact.key().to_string());
                }
                associated.extend(
                    by_assoc
                        .into_iter()
                        .map(|(assoc, facts)| rust_js_program::library::Associated {
                            key: key(tcx, assoc),
                            facts,
                            carrier: Some(key(tcx, carrier)),
                        }),
                );
            }
            associated.sort_by(|a, b| (&a.carrier, &a.key).cmp(&(&b.carrier, &b.key)));
            associated
        },
        keyed: {
            let mut keys: Vec<String> = (keyed.iter())
                .filter(|&&id| id.is_local() && reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        identities: {
            let mut keys: Vec<String> = (identities.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        eq_by_address: {
            let mut keys: Vec<String> = (eq_by_address.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        gives_argument: {
            let mut keys: Vec<String> = (gives_argument.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        // Its impls whose `partial_cmp` is `Some(self.cmp(other))`, read of
        // their HIR, which a consumer has none of (#964).
        partial_cmp_is_cmp: {
            let partial_ord =
                tcx.require_lang_item(rustc_hir::attrs::lang_items::LangItem::PartialOrd, rustc_span::DUMMY_SP);
            let mut keys: Vec<String> = crate::traits::declared_impls_of(tcx, partial_ord)
                .filter(|&imp| crate::ordering::partial_cmp_is_cmp(tcx, imp))
                .map(|imp| key(tcx, imp))
                .collect();
            keys.sort();
            keys
        },
        displays_value: {
            let mut keys: Vec<String> = (displays_value.iter())
                .filter(|&&id| reachable(tcx, id))
                .map(|&id| key(tcx, id))
                .collect();
            keys.sort();
            keys
        },
        built: {
            let mut keys: Vec<String> = built.iter().map(|&id| key(tcx, id)).collect();
            keys.sort();
            keys
        },
        // Another library's among them, which its consumers ask that one's
        // contract of, as each closure is asked of its own crate's, are
        // never read.
        cloned_closures: {
            let mut closures: Vec<rust_js_program::library::ClonedClosure> = (cloned_closures.iter())
                .map(|&(id, apart)| rust_js_program::library::ClonedClosure {
                    key: key(tcx, id),
                    apart,
                })
                .collect();
            closures.sort();
            closures
        },
        types: declared_types(tcx, contracts.built_slots.0),
        // Its package, which artifact planning reads (ADR 1008).
        package: None,
    }
}

/// Each type its modules' declarations export by its name, which a
/// consumer's declarations import (ADR 0210, amended): a public struct, enum
/// or union of a module's own that its declarations declare
/// (`declarations::declares_type`), and a trait whose dictionaries they
/// declare, `interface Middleware<Self>`, which a `dyn` of it is the pair
/// of (#1870); its default too, sorted.
fn declared_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::library::Foreign<'_, 'tcx>,
) -> Vec<rust_js_program::library::DeclaredType> {
    let declared = |id: DefId| match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Trait => crate::declarations::declared_methods(tcx, foreign, id).is_some(),
        _ => crate::declarations::declares_type(tcx, id),
    };
    let mut types: Vec<rust_js_program::library::DeclaredType> = (tcx.hir_crate_items(()).definitions())
        .map(|local| local.to_def_id())
        .filter(|&id| {
            declared(id)
                && matches!(tcx.def_kind(tcx.parent(id)), rustc_hir::def::DefKind::Mod)
                && tcx.visibility(id).is_public()
                && !crate::bindings::is_mark(tcx, id)
        })
        .map(|id| {
            let name = crate::declarations::type_name(tcx, id);
            rust_js_program::library::DeclaredType {
                key: key(tcx, id),
                module: module_path(tcx, tcx.parent_module_from_def_id(id.expect_local())),
                default: crate::default_exports::exported_as(tcx, id, &name) == "default",
                name,
            }
        })
        .collect();
    types.sort();
    types
}

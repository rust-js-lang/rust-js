//! Orchestrate analyzed crate facts, function emission, reachability and symbolic module assembly.

mod built;
mod consts;
mod contexts;
mod copies;
mod driver;
pub(crate) use self::driver::{start_session, tidy_pass_names};
mod crate_imports;
mod field_reads;
mod forwarders;
mod holes;
mod load_cycles;
mod packages;
mod pass;
mod reached;
mod settle;
mod variables;

pub use self::built::Built;
use self::holes::{fill_hole, fill_holes};
use self::load_cycles::{ModuleJs, module_imports};
use self::pass::Pass;
use self::reached::written_where_reached;
use rust_js_lower::analysis::{AnalyzedCrate, analyze_crate, nulled_params::NulledParams};
use rust_js_lower::bindings::Export;
use rust_js_lower::representation::{NonePlace, none_form};
use rust_js_lower::{
    Body, CrateFacts, Instances, LoweredFn, SerdeAttributes, Settled, aggregates, analysis, bindings, body_queries,
    classes, crate_facts, declarations, default_exports, drops, instances, library, located, module_file, path_read,
    patterns, recognition, reexports, refusal, representation, serde, sources, std_types, untagged,
};
use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js::{self, Expr, Prop, StmtKind};
use rust_js_program::program::{LoweredModule, Unlinked, UnlinkedModule};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalModId};

/// What lowering the crate gave: its JS, none where it failed; or the
/// library functions it must compile itself first, which the crate's calls
/// give more than they take (ADR 0914), each with the module it's placed in;
/// or the thread-locals it must make where they're first used, whose maker
/// is of a module that may load after theirs (ADR 0037, amended).
pub enum Lowered {
    Crate(Option<Box<Unlinked>>),
    Instances(Vec<(DefId, LocalModId)>),
    MadeWhereUsed(Vec<rustc_span::def_id::LocalDefId>),
}

/// Lower every function, grouped by module. Reports all unsupported
/// features as rustc errors. `instances` are the library functions the
/// crate compiles itself, whose bodies `all_bodies` has (ADR 0914);
/// `made_where_used`, the thread-locals an earlier lowering found must be
/// made where they're first used (ADR 0037, amended).
pub fn lower_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    (all_bodies, initializers): (&mut [Body<'tcx>], &mut [Body<'tcx>]),
    serde_attrs: &SerdeAttributes,
    (dependencies, instances, made_where_used): (
        &rust_js_program::library::Dependencies,
        &Instances,
        &IndexSet<rustc_span::def_id::LocalDefId>,
    ),
    built: Built<'_>,
) -> Lowered {
    let Built {
        library: export_library,
        program,
        std_bodies,
        std_iterators,
        panics_abort,
        tidy_until,
        js_modules,
    } = built;
    // A program decides by its own code, whatever contract it writes (#597).
    let library = export_library && !program;
    // What no codec of rust-js's reads, refused before anything is lowered.
    if serde::refuse_unread(tcx, serde_attrs) {
        return Lowered::Crate(None);
    }
    // A CSS Module's binding its file doesn't bear out (#1512).
    if bindings::refuse_unverified_css_modules(tcx, js_modules) {
        return Lowered::Crate(None);
    }
    let sources = sources::CapturedSources::new(tcx);
    // What the analysis stands on; then which adapters each body makes in
    // place, decided once the crate's `Recognition` exists, before anything
    // reads it, a collect's hint where the program sees it among what
    // decides it (#1327).
    // A binary that links a library that keeps a tracked value's caller's
    // line, which it would write (#2335).
    std_types::location::refuse_untold_locations(tcx, dependencies);
    let ground = analysis::ground(tcx, all_bodies, dependencies, (library, std_bodies), instances);
    for bodies in [&mut *all_bodies, &mut *initializers] {
        let facts = (&ground.cloned, ground.keeps_capacity());
        body_queries::decide_in_place(ground.recognition(tcx), bodies, facts, std_iterators);
    }
    let (all_bodies, initializers): (&[Body<'tcx>], &[Body<'tcx>]) = (all_bodies, initializers);
    let AnalyzedCrate {
        bound,
        bodies,
        flattened,
        closures,
        trait_impls,
        dictionaries,
        import_names,
        imported,
        required,
        required_uses,
        thread_local_inits,
        consts,
        variants,
        codecs,
        modules,
        taken,
        fns,
        mut failed,
        mut called_from_elsewhere,
        tests,
        paths,
        mutated,
        counted,
        self_handles,
        scopes,
        cell_borrows,
        wholes,
        closure_facts,
        cloned,
        located: located_calls,
        iterators,
        chars_whole,
        searches_whole,
        changed_vecs,
        js_copies,
        stored_places,
        drop_params,
        drops_any,
        bounds,
        js_given,
        type_facts,
        carried_super,
        built_slots,
        failing,
        writes_may_fail,
        dictionaries_fail,
        throwing,
        never_unwinding,
        quiet_fns,
        caller_locations,
        asks_capacity,
        generic_consts,
        given_again,
        writer_facts,
        prints,
        shown_params,
        written_params,
        reads_fn_identity,
        quiet_locals,
        quiet_library_keys,
        lazy_locals,
        eager_locals,
        locals_at_load,
        named_expressions,
        local_functions,
        local_consts,
        held,
        module_paths,
        null_fields,
        named_as_values,
    } = match analyze_crate(
        tcx,
        ground,
        all_bodies,
        initializers,
        library,
        (instances, made_where_used),
    ) {
        Some(analyzed) => analyzed,
        None => return Lowered::Crate(None),
    };
    // Each module's default export: its own, which a module of the crate
    // imports as its default, not by a name of its own (ADR 0251), as every
    // importer does (`default_exports::exported_as`); or another module's
    // function, which that module exports for it (`exported_across_modules`).
    let defaulted: IndexSet<DefId> = (default_exports::defaults(tcx).into_iter())
        .map(|(_, item)| item)
        .collect();
    // A library exports what its program does: what its consumers reach that
    // it doesn't, they compile (ADR 0914, amended). `@rust-js/std`, compiled
    // once into a package, all they may reach (ADR 0561).
    let exports_reachable = std_bodies;
    if exports_reachable {
        called_from_elsewhere.extend(fns.keys().copied().filter(|&id| library::reachable(tcx, id)));
    }
    // A derived serde impl's codec is one rust-js writes of its type's
    // attributes, which no consumer sees to write: a library exports each
    // its consumers can reach (ADRs 0077, 0100).
    if library {
        called_from_elsewhere.extend(codecs.iter().copied().filter(|&id| library::reachable(tcx, id)));
    }
    let foreign = library::Foreign::new(tcx, dependencies, instances, (writer_facts.writes(), reads_fn_identity))
        .asking_capacity(asks_capacity);
    // Which of its libraries' functions may copy what the crate changes in place, which their
    // JS, copying what their own crates change, may share (ADR 0914, amended).
    let changed = rust_js_lower::copies::Changed {
        mutated: &mutated.types,
        vecs: &changed_vecs,
    };
    let copying = instances::Copying::new((&trait_impls, &foreign), changed);
    let given = bounds.given(&js_given);
    // The library functions the crate's calls give more than they take,
    // compiled here first, before anything is lowered (ADR 0914).
    let analyzed = instances::Analyzed {
        trait_impls: &trait_impls,
        drops: (library, &closure_facts, &counted),
        drop_params: &drop_params,
        given,
        failing: analysis::Failing {
            fns: &failing,
            any: writes_may_fail,
            dictionaries: dictionaries_fail,
        },
        asking: iterators.hinted.asking(),
        bodies: bodies.iter().copied().chain(closures.values().copied()).collect(),
    };
    match instances::needed(&foreign, instances, analyzed) {
        _ if tcx.dcx().has_errors().is_some() => return Lowered::Crate(None),
        more if !more.is_empty() => return Lowered::Instances(more),
        _ => {}
    }
    // Each constant and static: its JS constant, or its initializer lowered
    // as code (`consts.rs`).
    let consts::Consts {
        items: mut const_items,
        owned: mut owned_consts,
        local_holes: mut local_const_holes,
        initialized,
        failed: consts_failed,
    } = consts::lower_consts(
        tcx,
        (&consts, initializers),
        (&fns, &foreign),
        (&flattened, &sources),
        (&thread_local_inits, &local_consts),
        &called_from_elsewhere,
    );
    failed |= consts_failed;

    let function_bodies = bodies.iter().map(|body| (body.def_id, *body)).collect();
    // What reads `js::arguments()`, each call of which gives every argument (rd332).
    let with_closures: Vec<&Body<'tcx>> = bodies.iter().copied().chain(closures.values().copied()).collect();
    let reads_arguments = analysis::arguments::reads_arguments(tcx, (&with_closures, initializers), &foreign);
    let (forwarded, prototypes) =
        reached::forwarded(tcx, &bodies, (given, &js_given), (&caller_locations, &fns), &foreign);
    let named_anywhere: Vec<&Body<'tcx>> = with_closures.iter().copied().chain(initializers).collect();
    // A namespace's method that's its function is that function (#601); a
    // nullable thread-local's every read is in the crate's bodies, its
    // initializers' too (#665).
    let recognition = recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    let nulled_params = NulledParams::scan(
        &recognition,
        (&with_closures, &named_anywhere, &forwarded),
        (&named_as_values, &cell_borrows),
    );
    failed |= !classes::exported_by_property(tcx, &fns, &forwarded);
    // A function that only passes its parameters on to a function at one
    // instantiation, which its module re-exports where its JS is that call
    // alone (rd575, ADR 1064).
    let forwarders = forwarders::found(
        tcx,
        (&bodies, &named_anywhere),
        (&foreign, &trait_impls),
        &js_given,
        (&drop_params, &reads_arguments),
    );
    let forwarding_prototypes = reached::forwarding(prototypes, &reads_arguments.functions, &forwarders);
    // A trait impl's dictionary (ADR 0049) has no body of its own, so its
    // function context gets an empty one; a copied default brings its own.
    let no_body = rustc_middle::thir::Thir::new(rustc_middle::thir::BodyTy::Const(tcx.types.unit));
    // Lower each body once. Cross-module references have symbolic names until
    // the link step knows exactly which imports and local names survive.
    let mut lowered_items: Vec<(DefId, LoweredFn)> = Vec::new();
    let mut lowered_copies: Vec<((DefId, usize), LoweredFn)> = Vec::new();
    // Each module's JS, where a JS file of the program's own imports it
    // from, and what it imports for its effects (`module_imports`).
    let places = ModuleJs::of(tcx, &modules, &paths);
    // What each module re-exports, decided once, before any is lowered: what
    // its JS writes, `export { one } from "./z.js"`, and its imports in the
    // load-order graph (`module_imports`, #295).
    let mut reexported: IndexMap<LocalModId, _> = (modules.iter())
        .map(|&module| {
            let module_held = held.modules.get(&module).map_or(&[][..], Vec::as_slice);
            let default = default_exports::module_default(tcx, module, &fns, module_held);
            let from = (default.from, module_held);
            let made = (&fns, &foreign, &forwarded);
            let reexports = reexports::reexports(tcx, module, made, &paths, from);
            (module, (default, reexports))
        })
        .collect();
    // And what each function that may be a re-export is of (rd575).
    let forwarded_from = forwarders::reexported_from(tcx, &modules, &forwarders, (&fns, &foreign, &paths), &sources);
    let reexports_from: Vec<load_cycles::ReexportFrom> = (reexported.iter())
        .flat_map(|(&module, (_, reexports))| reexports.iter().map(move |(_, from, _)| (module, from.clone())))
        .chain(forwarded_from)
        .collect();
    // A `Vec`'s capacity is kept where the crate or a library it uses asks
    // it, as what it gives that library may be asked (ADR 0419); and by
    // `@rust-js/std`'s bodies, which every consumer's `capacity()` may ask
    // of what they grow, as its manifest says (#1339).
    let keeps_capacity = asks_capacity || foreign.keeps_capacity() || std_bodies;
    let takes_used_drops = drops::TakesUsed::new(tcx, &foreign, &drop_params);
    // What each function's lowering found of drops, gathered in the order
    // they're lowered, lowered or not, then resolved (ADRs 0190, 0300).
    let mut drops_observed = drops::DropsObserved::default();
    let ref_cells = crate_facts::Lazy::default();
    let no_callbacks = IndexSet::default();
    // What runs where its module loads, which reads another module's
    // `const`s as their values where that module may load after its own
    // (`load_cycles`). A thread-local made where it's first used runs
    // nothing as its module loads (ADR 0037, amended).
    let loads: IndexSet<DefId> = (thread_local_inits.iter())
        .filter(|(_, key)| !lazy_locals.contains(*key))
        .map(|(init, _)| init.to_def_id())
        .chain(
            (initialized.iter())
                .map(|body| body.def_id)
                .filter(|id| !local_consts.contains_key(id)),
        )
        .chain(
            (bodies.iter())
                .map(|body| body.def_id)
                .filter(|&id| bindings::is_on_load(tcx, id)),
        )
        .collect();
    let struct_literals = aggregates::struct_literals(tcx);
    // The nullable fields each constructor of their struct sets first (rd628).
    let constructed = classes::set_where_constructed(tcx, &named_anywhere);
    // The module's imports a body reads as plain, made once (#2315).
    let plain_imports = rust_js_mir::plain_imports(&import_names);
    let crate_facts = CrateFacts {
        flattened: &flattened,
        foreign: &foreign,
        library,
        compiled_once: exports_reachable,
        copying: &copying,
        panics_abort,
        sources: &sources,
        mutated: &mutated,
        counted: &counted,
        self_handles: &self_handles,
        scopes: &scopes,
        cell_borrows: &cell_borrows,
        wholes: &wholes,
        closure_facts: &closure_facts,
        cloned: &cloned,
        located: &located_calls,
        iterators: &iterators,
        chars_whole,
        searches_whole,
        generic_consts: &generic_consts,
        given_again: &given_again,
        changed_vecs: &changed_vecs,
        js_copies: &js_copies,
        stored_places: &stored_places,
        drop_params: &drop_params,
        given,
        type_facts: &type_facts,
        carried_super: &carried_super,
        built_slots: &built_slots,
        failing: analysis::Failing {
            fns: &failing,
            any: writes_may_fail,
            dictionaries: dictionaries_fail,
        },
        throwing: &throwing,
        never_unwinding: &never_unwinding,
        quiet_fns: &quiet_fns,
        reads_arguments: &reads_arguments,
        caller_locations: &caller_locations,
        keeps_capacity,
        takes_used_drops: &takes_used_drops,
        closures: &closures,
        bodies: &function_bodies,
        fns: &fns,
        imports: &import_names,
        plain_imports: &plain_imports,
        required: &required,
        required_uses: &required_uses,
        held_reads: &held.reads,
        module_paths: &module_paths,
        trait_impls: &trait_impls,
        serde_attrs,
        pretty_debug: writer_facts.pretty_debug,
        format_options: writer_facts.format_options,
        debug_hex: writer_facts.debug_hex,
        shown: &shown_params,
        written: &written_params,
        prints: &prints,
        tidy_until,
        ref_cells: &ref_cells,
        own_callbacks: &no_callbacks,
        quiet_locals: &quiet_locals,
        quiet_library_keys: &quiet_library_keys,
        lazy_locals: &lazy_locals,
        eager_locals: &eager_locals,
        named_expressions: &named_expressions,
        local_functions: &local_functions,
        forwarded: &forwarded,
        local_consts: &local_consts,
        struct_literals: &struct_literals,
        constructed: &constructed,
        nulled_params: &nulled_params,
    };
    // Its own callbacks' parameters, found of the facts above (ADR 1148).
    let own_callbacks = contexts::own_callbacks(tcx, &crate_facts, &no_body);
    let crate_facts = CrateFacts {
        own_callbacks: &own_callbacks,
        ..crate_facts
    };
    // What a library's manifest says of its writers (ADR 0137).
    let writers_take_options = crate_facts.writers_take_options();
    // Each dictionary's cache, named apart in its module.
    let mut cache_names: IndexMap<LocalModId, IndexSet<String>> = IndexMap::default();
    let cache_of: IndexMap<DefId, String> = (dictionaries.iter())
        .map(|&id| {
            let info = &fns[&id];
            let cache =
                rust_js_program::names::dictionary_cache(cache_names.entry(info.module).or_default(), &info.name);
            (id, cache)
        })
        .collect();
    // A generic dictionary's cache's value, its module's `const` (ADR 0592).
    let mut cache_values: IndexMap<DefId, Expr> = IndexMap::default();
    let mut work: Vec<(DefId, Option<&Body<'tcx>>, Option<usize>)> = bodies
        .iter()
        .filter(|b| reached::lowered_here(tcx, (&foreign, &forwarded), exports_reachable, b.def_id))
        // A scope struct's method is lowered in its function (ADR 1043).
        .filter(|b| scopes.of_method(b.def_id).is_none())
        // Each copy its callers need besides, after it (ADR 0051, amended).
        .flat_map(|b| {
            let copies = js_given.copies(b.def_id).map(|(at, _)| Some(at));
            std::iter::once(None)
                .chain(copies)
                .map(|copy| (b.def_id, Some(*b), copy))
        })
        .chain(dictionaries.iter().map(|id| (*id, None, None)))
        .chain(initialized.iter().map(|body| (body.def_id, Some(*body), None)))
        .collect();
    // Each method that reads each variant's field, first, so its callers read it (rd317).
    let field_reads = field_reads::found(tcx, &bodies);
    field_reads::first(&mut work, &field_reads);
    let (mut used, mut queued) = (IndexSet::default(), IndexSet::default());
    let no_facts = body_queries::BodyFacts::default();
    // What the bodies lowered settle, which each lowered after reads: made
    // anew of what's settled for each body, as no body changes it.
    let mut settled = Settled::default();
    // What of its libraries the bodies found the crate compiles itself,
    // which their JS doesn't export, as each gives it back (ADR 1148).
    let mut asked = Vec::new();
    let shared = contexts::Shared {
        tcx,
        krate: &crate_facts,
        no_body: &no_body,
        no_facts: &no_facts,
    };
    let mut next = 0;
    // Once each body queued is lowered, a round more, of the first protocol
    // that asks for one, bounded (`settle`, ADR 1148).
    let mut rounds = settle::Rounds::new(codecs.len());
    loop {
        // Then the codecs something uses, a round at a time, each in the
        // order they're declared. A shared crate derives both for its types,
        // so one that's never used isn't lowered: its type needn't be one
        // rust-js reads or writes. A library's consumers may use one they can
        // reach (ADR 0100).
        if next == work.len() {
            let round: Vec<_> = codecs
                .iter()
                .filter(|&&id| (used.contains(&id) || (library && library::reachable(tcx, id))) && queued.insert(id))
                .map(|&id| (id, None, None))
                .collect();
            let protocol = if !round.is_empty() {
                work.extend(round);
                settle::Protocol::Codecs
            } else if nulled_params.again(&mut settled.nulled, &mut work, |&(at, _, _)| at) {
                // Each with an own nullable parameter each call gives `null`
                // or a value, lowered again with its copies (ADR 0861, amended).
                settle::Protocol::NulledParams
            } else {
                // Each body run where its module loads that read another
                // module's `const` by its name, where that module may load
                // after its own: lowered again, reading its values, once
                // (ADR 0031, amended); a thread-local its module's load
                // can't make first, made where it's first used (ADR 1006).
                let modules = (&places[..], &fns, (&imported, &reexports_from[..]), js_modules);
                let at_load = (&locals_at_load, &loads);
                match load_cycles::again(tcx, (&lowered_items, &settled.loads_in_cycles), modules, at_load) {
                    load_cycles::Again::Settled => break,
                    load_cycles::Again::MadeWhereUsed(cyclic) => return Lowered::MadeWhereUsed(cyclic),
                    load_cycles::Again::Lower(again) => {
                        let bodies: Vec<_> = (again.iter())
                            .filter_map(|&id| work.iter().find(|&&(at, _, copy)| at == id && copy.is_none()).copied())
                            .collect();
                        settled.loads_in_cycles.extend(again);
                        work.extend(bodies);
                        settle::Protocol::LoadCycles
                    }
                }
            };
            if let Err(words) = rounds.another(protocol) {
                tcx.dcx().err(words);
                failed = true;
                break;
            }
        }
        let (def_id, body, copy) = work[next];
        next += 1;
        let module = fns[&def_id].module;
        // Locals must never shadow a function or an import of this file (ADR
        // 0267). Not its recursive types' functions' (`recursive_fns`), made
        // after, nor an earlier `on_load!` body's, at the module's top beside
        // this one's: what one declares at its top is a name not spelled yet,
        // which preparation spells apart from the module's top (#2070), and
        // a local of a function's word that reads it is given apart from it
        // where the module is prepared (ADR 1150).
        let mut names = taken[&module].clone();
        // A function's scope struct's methods, functions declared in it (ADR 1043).
        let scope_functions = scopes.function_names(def_id, |method| fns[&method].name.clone(), &mut names);
        let mut cx = contexts::fn_cx(
            (&shared, &settled),
            (def_id, body, copy),
            (module, &taken[&module]),
            names,
            scope_functions,
        );
        rust_js_mir::MirCx(&mut cx).nest_scope_copies();
        let result = match body {
            // Each body, a function's or an initializer's, from its MIR (ADR 0364).
            Some(body) if let Some(mir) = &body.mir => rust_js_mir::MirCx(&mut cx).lower_fn_mir(body, mir),
            Some(_) => Err(cx.unsupported(tcx.def_span(def_id), "this body, without its MIR")),
            None if codecs.contains(&def_id) => cx.lower_codec(def_id).map(|function| LoweredFn {
                function,
                jsx: cx.jsx,
                dependencies: cx.dependencies.take(),
                entry: None,
            }),
            None => cx
                .lower_dictionary(def_id, &cache_of[&def_id])
                .map(|(function, cache)| {
                    if let Some(cache) = cache {
                        cache_values.insert(def_id, cache);
                    }
                    LoweredFn {
                        function,
                        jsx: cx.jsx,
                        dependencies: cx.dependencies.take(),
                        entry: None,
                    }
                }),
        };
        // Of one that failed too, whose calls are checked as each other's.
        drops_observed.gather(cx.drops_observed.take());
        asked.extend(cx.asked_of_libraries.take());
        let (made, seen) = (cx.module_fns_made.take(), cx.nulled_seen.take());
        drop(cx);
        settled.module_fns.settle(made);
        settled.nulled.settle(seen);
        match result {
            Ok(lowered) => {
                used.extend(lowered.dependencies.uses.iter().map(|&(_, to)| to));
                // A copy its callers need, beside it (ADR 0051, amended).
                if let Some(copy) = copy {
                    lowered_copies.retain(|(at, _)| *at != (def_id, copy));
                    lowered_copies.push(((def_id, copy), lowered));
                    continue;
                }
                if field_reads.contains(&def_id)
                    && let Some(read) = untagged::field_reads::read_of_its_parameter(&lowered.function)
                {
                    settled.field_reads.insert(def_id, read);
                }
                // One lowered again, its values read: in its place.
                match lowered_items.iter_mut().find(|(id, _)| *id == def_id) {
                    Some(earlier) => earlier.1 = lowered,
                    None => lowered_items.push((def_id, lowered)),
                }
            }
            Err(_) => failed = true,
        }
    }
    // A library's contract says of its `RefCell`s, its structs' fields'
    // too, though none of its bodies asked (rd418): found in a context of
    // one of its structs, as any body's finds them, of the crate.
    let a_struct = || (tcx.hir_crate_items(()).definitions()).find(|&id| tcx.def_kind(id) == DefKind::Struct);
    if export_library
        && ref_cells.get().is_none()
        && let Some(id) = a_struct()
    {
        let given = (CRATE_MOD_ID, &taken[&CRATE_MOD_ID]);
        let cx = contexts::item_cx((&shared, &settled), id.to_def_id(), given);
        cx.ref_cells();
        asked.extend(cx.asked_of_libraries.take());
        let made = cx.module_fns_made.take();
        drop(cx);
        settled.module_fns.settle(made);
    }
    // What of its libraries the crate's lowering found it compiles itself,
    // which their JS doesn't export: the crate lowered again with them, as
    // with what its calls give more (ADR 0914, amended).
    // Each module's recursive types' functions (`recursive_fns`).
    let mut module_fns = std::mem::take(&mut settled.module_fns);
    if !asked.is_empty() && tcx.dcx().has_errors().is_none() {
        return Lowered::Instances(instances::asked(tcx, asked));
    }
    // What a function takes no destructor of, each call of it checked (ADR 0190).
    let checks = std::mem::take(&mut drops_observed.checks);
    failed |= drops::check_no_drops(tcx, &mut drops_observed.no_drops, &checks);
    if failed {
        return Lowered::Crate(None);
    }
    let no_drops = std::mem::take(&mut drops_observed.no_drops);
    // A library's functions whose JS may give one function where Rust makes
    // two, as it compares none, and each that names one: what a consumer
    // that compares a function's identity compiles its own of (#1052).
    let merges_identities = library::merging(
        &counted.merges,
        (lowered_items.iter().map(|(id, item)| (*id, item)))
            .chain(lowered_copies.iter().map(|((id, _), item)| (*id, item))),
    );
    // Only the drops each function uses (ADR 0300), of each copy of one its
    // callers need too, and of the module's own functions (#974).
    let lowered = (lowered_items.iter_mut().map(|(id, item)| (*id, item)))
        .chain(lowered_copies.iter_mut().map(|((id, _), item)| (*id, item)));
    let kept_drop_params = drops_observed.keep(&takes_used_drops, &drop_params, lowered, &mut module_fns);
    // The copies made alike another left out, each call of one calling that
    // one (ADR 0051, amended).
    let through = (lowered_items
        .iter()
        .map(|(_, item)| item)
        .chain(lowered_copies.iter().map(|(_, item)| item)))
    .flat_map(|item| item.dependencies.through_copies.iter().copied())
    .collect::<Vec<_>>();
    let mut copies_of = copies::settle(&js_given, &fns, &mut lowered_items, lowered_copies, (tcx, &through));
    // What's written only where it's reached (`reached.rs`), the types equal only to themselves, its
    // contract's (ADR 0864, amended), and the untagged enums each plain `{}` of which is in place (rd730).
    let lowered = lowered_items.iter().map(|&(id, _)| id);
    let ((identities, eq_by_address), derived, displays_value) =
        written_where_reached(tcx, (&trait_impls, &foreign), &function_bodies, lowered);
    // Its functions each call of which is what it's given, which its
    // contract lists for its consumers (ADR 0584, amended).
    let gives_argument: Vec<DefId> = (function_bodies.keys().copied())
        .filter(|&id| patterns::gives_its_argument(tcx, &function_bodies, id))
        .collect();
    let edges = lowered_items
        .iter()
        .flat_map(|(_, item)| item.dependencies.uses.iter().copied());
    // What each module's own items read by name of its own: a `const` the
    // module's default holds, read so, is declared, then exported (ADR 0625,
    // amended).
    let read_at_home: IndexSet<DefId> = (edges.clone())
        .filter(|&(from, to)| {
            from.as_local()
                .zip(to.as_local())
                .is_some_and(|(from, to)| tcx.parent_module_from_def_id(from) == tcx.parent_module_from_def_id(to))
        })
        .map(|(_, to)| to)
        .collect();
    let own_roots = (edges.clone())
        .filter(|(from, _)| !derived.contains(from))
        .map(|(_, to)| to);
    // What the crate's own code reaches: a dictionary its consumers alone
    // read is read once every module of the crate has loaded, so no load
    // cycle of the crate's reaches it (ADR 0592, amended).
    let reached_here = rust_js_program::reachability::reachable(own_roots.clone(), edges.clone());
    let roots = own_roots
        // `@rust-js/std`'s consumers may reach one (ADR 0561); a library's
        // compile their own (ADR 0914, amended).
        .chain(
            derived
                .iter()
                .copied()
                .filter(|&id| exports_reachable && library::reachable(tcx, id)),
        );
    let reached = rust_js_program::reachability::reachable(roots, edges);
    let fmt_into_left = lowered_items.iter().any(|(_, item)| item.dependencies.fmt_into_left);
    let untold_locations: Vec<rust_js_program::library::UntoldLocation> = (lowered_items.iter())
        .flat_map(|(_, item)| item.dependencies.untold_locations.iter().cloned())
        .collect();
    let mut pass = Pass::default();
    for (def_id, mut lowered) in lowered_items
        .into_iter()
        .filter(|(id, _)| !derived.contains(id) || reached.contains(id))
    {
        let module = fns[&def_id].module;
        // A binary's `main`'s start, which its module runs last (#2292).
        if let Some(entry) = lowered.entry.take() {
            pass.entry = Some((module, entry));
        }
        // Each recursive type's function, and std order's dictionary (#944), it
        // reads: its module's, before it where it's the first (`recursive_fns`).
        let first = module_fns.written_before(def_id);
        pass.dictionary_names.extend(first.const_names());
        pass.functions.entry(module).or_default().extend(first.functions);
        pass.local_consts.entry(module).or_default().extend(first.consts);
        let initializer = initialized.iter().any(|body| body.def_id == def_id);
        // Each copy of it its callers need besides, beside it (ADR 0051, amended).
        let mut copies = copies_of.swap_remove(&def_id).unwrap_or_default();
        copies::gather(&mut pass, (module, &mut lowered), &mut copies);
        // A class's trait of classes' impl: its defaults the prototype's (rd283).
        if dictionaries.contains(&def_id)
            && let Some(class) = classes::trait_impl_class(tcx, def_id)
        {
            let defaults = classes::prototype_defaults(tcx, def_id, std::mem::take(&mut lowered.function.body));
            pass.class_defaults
                .extend(defaults.into_iter().map(|function| (class, function)));
            pass.jsx.extend(lowered.jsx.then_some(module));
            continue;
        }
        // A dictionary given nothing is its module's `const`, what its entries
        // share beside it; one given something, its function, its cache a
        // `const` beside it (ADR 0592).
        if dictionaries.contains(&def_id) {
            let span = def_id
                .as_local()
                .map_or(js::Span::NONE, |local| sources.item_span(tcx, local));
            // At the module's top, its names the module's (ADR 1150).
            let constant = |name: String, value: Expr, export: bool| js::Const {
                name: js::Name::module(&name),
                value,
                mutable: false,
                fields: js::Fields::default(),
                export,
                span,
            };
            if reached_here.contains(&def_id) {
                pass.dictionary_consts.push((module, def_id));
            }
            let consts = pass.local_consts.entry(module).or_default();
            let made = consts.len();
            if let Some(cache) = cache_values.swap_remove(&def_id) {
                consts.push(constant(cache_of[&def_id].clone(), cache, false));
            } else if lowered.function.params.is_empty() {
                let export = lowered.function.export || called_from_elsewhere.contains(&def_id);
                let mut body = std::mem::take(&mut lowered.function.body);
                let Some(js::Stmt {
                    kind: StmtKind::Return(Some(object)),
                    ..
                }) = body.pop()
                else {
                    unreachable!("a dictionary given nothing returns its object")
                };
                for stmt in body {
                    if let StmtKind::Const(name, value) = stmt.kind {
                        consts.push(constant(name.into_word(), value, false)); // word: the module's constant, its word its identity
                    }
                }
                consts.push(constant(fns[&def_id].name.clone(), object, export));
                pass.dictionary_names
                    .extend(consts[made..].iter().map(|c| c.name.clone()));
                if lowered.jsx {
                    pass.jsx.insert(module);
                }
                continue;
            }
            pass.dictionary_names
                .extend(consts[made..].iter().map(|c| c.name.clone()));
        }
        pass.prototypes.read.extend(&lowered.dependencies.prototypes_read);
        match lowered {
            lowered
                if let Some(key) = def_id.as_local().and_then(|local| {
                    (thread_local_inits.get(&local).copied()).or_else(|| initializer.then_some(local))
                }) =>
            {
                // `const COUNT = { value: 0 };`: made when the module loads. One
                // whose initializer runs the crate's code, or may panic, made
                // where it's first used, as Rust makes it, and read by calling
                // it, `const START = $local(() => ..)` (ADR 0037, amended).
                let function = lowered.function;
                let fields = classes::namespace_fields(tcx, key.to_def_id());
                let value = match (lazy_locals.contains(&key), function.body.as_slice()) {
                    (true, _) => Expr::call_helper("$local", vec![Expr::arrow(Vec::new(), function.body)]),
                    (
                        false,
                        [
                            js::Stmt {
                                kind: StmtKind::Return(Some(value)),
                                ..
                            },
                        ],
                    ) => value.clone(),
                    (false, body) => classes::taken_apart(&fields, body)
                        .unwrap_or_else(|| Expr::call(Expr::arrow(Vec::new(), function.body), Vec::new())),
                };
                // A nullable thread-local's `None` `null` (ADR 0586, amended).
                let value = match none_form(tcx, NonePlace::Local(key.to_def_id())).stores_null() {
                    true => std_types::cell::nullable_made(value),
                    false => value,
                };
                // A `static mut`'s is its `{ value }` (ADR 0096).
                let value = match tcx.static_mutability(key.to_def_id()) {
                    Some(mutability) if mutability.is_mut() => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),
                    _ => value,
                };
                let info = &fns[&key.to_def_id()];
                let local = local_consts.contains_key(&key.to_def_id());
                let constant = js::Const {
                    name: consts::const_name(&info.name, key, local, info.owner.is_some()),
                    value,
                    mutable: false,
                    // A namespace's, its module's exports (ADR 1037, amended).
                    fields: classes::exported_fields(tcx, key.to_def_id(), fields, |at| sources.span(at)),
                    // A `const { .. }` block has no visibility: it's its function's alone.
                    export: (tcx.def_kind(key) != DefKind::AnonConst && tcx.visibility(key).is_public())
                        || called_from_elsewhere.contains(&key.to_def_id()),
                    span: sources.item_span(tcx, key),
                };
                match local {
                    true => local_const_holes.push((key.to_def_id(), constant)),
                    false if info.owner.is_some() => _ = owned_consts.insert(key.to_def_id(), constant),
                    false => pass.local_consts.entry(info.module).or_default().push(constant),
                }
                if lowered.jsx {
                    pass.jsx.insert(module);
                }
            }
            mut lowered => {
                if lowered.jsx {
                    pass.jsx.insert(module);
                }
                // A class's, its constructor or its prototype's (ADR 0761);
                // an object's method, its module's function (ADR 0943).
                let member = classes::member(tcx, def_id);
                let owner = fns[&def_id].owner.as_deref().unwrap_or_default();
                let mut copied =
                    std::iter::once(&mut lowered.function).chain(copies.iter_mut().map(|c| &mut c.function));
                let mut constructor_field_docs = Vec::new();
                if let Some(what) =
                    copied.find_map(|f| classes::as_member(tcx, def_id, owner, f, &mut constructor_field_docs).err())
                {
                    refusal::refuse(tcx, tcx.def_span(def_id), refusal::Refused::Feature(what));
                    failed = true;
                    continue;
                }
                let copies = copies.into_iter().map(|copy| copy.function);
                lowered.function.export |= called_from_elsewhere.contains(&def_id);
                match &fns[&def_id].owner {
                    Some(owner) => {
                        let namespace = js::Namespace::of(pass.namespaces.entry(module).or_default(), owner);
                        let (export, elsewhere) = (lowered.function.export, called_from_elsewhere.contains(&def_id));
                        namespace.export |= classes::exports_its_type(tcx, def_id, export, elsewhere);
                        if let Some(member) = member {
                            let class = namespace.class.get_or_insert_with(|| js::Class {
                                declared: (classes::class_of(tcx, def_id).and_then(|adt| adt.as_local()))
                                    .map_or(js::Span::NONE, |adt| sources.item_span(tcx, adt)),
                                ..classes::js_class(tcx, def_id, &import_names[&module])
                            });
                            class.constructor_field_docs.extend(constructor_field_docs);
                            let forwarding = forwarding_prototypes.contains(&def_id);
                            classes::place((member, forwarding), class, lowered.function.name.word()); // word: the method's JS name, as the class shows it
                        }
                        namespace.methods.push(lowered.function);
                        namespace.methods.extend(copies);
                    }
                    None if bindings::is_on_load(tcx, def_id) => {
                        // One replacing a class's prototype, set right after
                        // its constructor, alone (#448, ADR 0761 amended).
                        let set = lowered.dependencies.prototype_set;
                        pass.prototypes.set_alone(tcx, module, set, &lowered.function.body);
                        pass.statements.entry(module).or_default().push(lowered.function.body)
                    }
                    // Named in itself alone, as the Rust names it, unless
                    // what it reads is named so.
                    // Its function's, where it's written there (ADR 0308).
                    None if local_functions.contains_key(&def_id) => {
                        let mut function = lowered.function;
                        function.export = false;
                        pass.holes.functions.insert(def_id.index.as_u32(), function);
                    }
                    None if named_expressions.contains(&def_id) => {
                        let mut function = lowered.function;
                        // Spelled by preparation apart from what its body
                        // reads, of the item it's written in, its own by its
                        // index there.
                        function.name = function.name.spelled_later(def_id.index.as_usize());
                        function.export = false;
                        pass.holes.functions.insert(def_id.index.as_u32(), function);
                    }
                    None => {
                        let functions = pass.functions.entry(module).or_default();
                        functions.push(lowered.function);
                        functions.extend(copies);
                    }
                }
            }
        }
    }
    // Each default a class takes of a trait of classes, its prototype's
    // method beside its own (rd283).
    for (class, function) in std::mem::take(&mut pass.class_defaults) {
        let Some(constructor) = classes::constructor_of(tcx, class) else {
            continue;
        };
        let info = &fns[&constructor];
        let namespaces = pass.namespaces.entry(info.module).or_default();
        let owner = info.owner.as_deref().unwrap_or_default();
        if let Err(what) = classes::take_default(namespaces, owner, function) {
            refusal::refuse(tcx, tcx.def_span(class), refusal::Refused::Feature(&what));
            failed = true;
        }
    }
    if failed {
        return Lowered::Crate(None);
    }
    // Each constant placed, those the bodies made too (`consts.rs`).
    consts::place_consts(
        tcx,
        &mut pass,
        (&mut const_items, &mut owned_consts),
        (&consts, &variants),
        &fns,
        (&called_from_elsewhere, &reached),
        &sources,
    );
    // Each function a block makes and gives, where its block was (ADR 0296).
    let holes = &mut pass.holes;
    for constant in const_items.values_mut().flatten() {
        constant.value.each_mut(&mut |e| fill_hole(e, holes));
    }
    // Each `const` written in a function's body, a `const` there, its own
    // and never exported.
    for (def_id, mut constant) in local_const_holes {
        constant.value.each_mut(&mut |e| fill_hole(e, holes));
        constant.export = false;
        holes.consts.insert(def_id.index.as_u32(), constant);
    }
    let methods = pass
        .namespaces
        .values_mut()
        .flatten()
        .flat_map(|n| n.methods.iter_mut());
    for function in pass.functions.values_mut().flatten().chain(methods) {
        fill_holes(&mut function.body, holes);
    }
    for statements in pass.statements.values_mut().flatten() {
        fill_holes(statements, holes);
    }

    pass.export_referenced(&fns, &defaulted);
    // A copy of a function another module calls, exported by its name (ADR
    // 0051, amended).
    copies::export(&js_given, &fns, &pass.copy_references, &mut pass.functions);
    let copy_edges: Vec<(LocalModId, DefId)> = (pass.copy_references.iter()).map(|&(from, id, _)| (from, id)).collect();
    let edges = (
        pass.references.iter().chain(copy_edges.iter()),
        pass.held.iter(),
        pass.package_uses.iter(),
        pass.reexporters.iter(),
    );
    let mut imports = module_imports(&places, &fns, edges, (&imported, &reexports_from), js_modules);
    load_cycles::refuse_dictionaries_in_load_cycles(tcx, &pass, &const_items, (&modules, &imports));
    // What a module names through another of the crate's modules that
    // re-exports it, imported from there where that makes no load cycle
    // (rd650).
    let through = reexports::imported_through(tcx, (&fns, &foreign));
    let imported_through = load_cycles::taken_through(through, &paths, &mut imports);
    let copies = copies::imported(&js_given, &fns, &pass.copy_references);
    let mut targets = crate_imports::of_crate(tcx, (&pass, &fns), copies, &imported_through);
    // What another module than its own calls, which stays a function.
    let called_elsewhere: IndexSet<DefId> = (pass.references.iter())
        .filter(|&&(from, id)| fns[&id].module != from)
        .map(|&(_, id)| id)
        .collect();
    // The thread-locals and `static mut`s their modules made their variables, exported or internal (#2017).
    let mut variables: IndexSet<DefId> = IndexSet::default();
    // A library's private state its consumers' compiles may read, and what each module exports of it (#2017).
    let (internal, mut internal_exports) = (library::internal_state(tcx, library), Vec::new());
    let mut lowered: Vec<UnlinkedModule> = modules
        .into_iter()
        .map(|module| {
            // Where each import's `use` is, which its comments are before.
            let through = |id| imported_through.get(&(module, id)).map(|through| through.module);
            let uses = analysis::UsesWritten::of(tcx, module, (&foreign, &held), &through);
            let written = |at: Option<rustc_span::Span>| at.map_or(js::Span::NONE, |at| sources.span(at));
            // What it writes of no JS where it's written: its bindings, and
            // its types' and traits' declarations (#643).
            let unprinted: Vec<(js::Span, bool)> = (uses.bindings().map(|at| (at, true)))
                .chain(sources::declarations(tcx, module).map(|at| (at, false)))
                .map(|(at, binding)| (written(Some(at)), binding))
                .collect();
            // One `import` per JS module, of what this file uses from it, and
            // of what it imports for what that does when loaded (ADR 0110).
            let used = |export: &Export| pass.package_uses.contains(&(module, export.clone()));
            let (packages, loaded) = packages::of_module(
                tcx,
                module,
                &imported,
                &used,
                &import_names[&module],
                (&uses, &foreign),
                &written,
            );
            // `js::directive!("use client");` and `js::export_default!(page);`,
            // what a Next.js route's module has (ADR 0192).
            let directives = bindings::module_directives(tcx, module);
            // What it holds of a binding as a value of its own (ADRs 0622, 0633).
            let module_held = held.modules.get(&module).map_or(&[][..], Vec::as_slice);
            let (
                default_exports::ModuleDefault {
                    export: default_export,
                    declared: default_declared,
                    ..
                },
                reexports,
            ) = reexported.swap_remove(&module).expect("each module's re-exports");
            let aliased_items = default_exports::aliases(tcx, module, &fns);
            // Each of its functions that only passes its parameters on to a
            // function at one instantiation, whose JS is that call alone:
            // that function, re-exported by its name, no function of its
            // own, its declaration its own still; what only it read not
            // imported (rd575, ADR 1064).
            let forwarders::InPlace {
                reexports,
                declared: declared_reexports,
                reads: forwarded_reads,
            } = forwarders::reexported(
                tcx,
                (module, pass.functions.entry(module).or_default()),
                &forwarders,
                (&fns, &foreign, &paths),
                default_export,
                &aliased_items,
                (reexports, &sources),
            );
            // A binding it holds, its `const` of what the binding reads (ADRs
            // 0622, 0633); not one it re-exports from another that holds it.
            let names = import_names.get(&module);
            const_items.entry(module).or_default().extend(
                (module_held.iter())
                    .filter(|held| held.through.is_none())
                    .map(|held| js::Const {
                        name: js::Name::module(&held.name),
                        value: path_read(&held.path, &required, |export| {
                            names.expect("its imports")[export].clone()
                        }),
                        export: held.exported_by_name(),
                        mutable: false,
                        fields: js::Fields::default(),
                        span: held.span.map_or(js::Span::NONE, |span| sources.span(span)),
                    }),
            );
            // A constant the default is, read by name in its own module, is
            // declared and then exported by default, `const v = ..; export
            // default v;`, as JS has no name of `export default ..;`; still
            // its default alone (ADR 0625, amended). So is one a `pub use`
            // of its module names by another name, `export { VERSION as
            // RELEASE };`, which reads it by its name.
            let aliased = |held: DefId| aliased_items.iter().any(|&(item, _)| item == held);
            let read_default = default_export.filter(|&held| {
                default_declared && tcx.def_kind(held) != DefKind::Fn && (read_at_home.contains(&held) || aliased(held))
            });
            // What either form of `js::export_default!` makes the default, a
            // function or a constant of the module's own, is its default
            // alone (`Pass::default_alone`); a binding's is
            // `HeldBinding::exported_by_name`'s.
            if let Some(own) = default_export.filter(|&own| bindings::held_path(tcx, own).is_none()) {
                pass.default_alone(
                    module,
                    &bindings::fn_name(tcx, own),
                    const_items.entry(module).or_default(),
                );
            }
            let no_ref_cells = drops::RefCells::default();
            let ref_cells = ref_cells.get().unwrap_or(&no_ref_cells);
            let aliases: Vec<(String, String)> = aliased_items.iter().map(|(_, names)| names.clone()).collect();
            let entry = pass.entry.take_if(|(of, _)| *of == module).map(|(_, entry)| entry);
            let mut lowered = LoweredModule {
                path: paths[&module].clone(),
                file: module_file(tcx, module).name.clone().into_local_path(),
                code: sources.module_code(tcx, module),
                unprinted,
                directives,
                packages,
                imports: Vec::new(),
                import_names: js::ImportNames::default(),
                taken_into: Vec::new(),
                reexports,
                aliases,
                // A binary's `main`'s start, after all its module makes and
                // runs as it loads (#2292, #2527).
                items: js::entry_last(
                    js::in_load_order(
                        (pass
                            .namespaces
                            .swap_remove(&module)
                            .into_iter()
                            .flatten()
                            .map(js::Item::Namespace))
                        .chain(
                            const_items
                                .swap_remove(&module)
                                .into_iter()
                                .flatten()
                                .map(js::Item::Const),
                        )
                        .chain(
                            pass.statements
                                .swap_remove(&module)
                                .into_iter()
                                .flatten()
                                .map(js::Item::Statements),
                        )
                        .chain(entry.clone().map(js::Item::Statements))
                        .chain(
                            pass.functions
                                .swap_remove(&module)
                                .into_iter()
                                .flatten()
                                .map(js::Item::Function),
                        )
                        .collect(),
                    ),
                    entry.as_deref(),
                ),
                default_export: default_export.map(|function| bindings::fn_name(tcx, function)),
                default_declared: default_declared && read_default.is_none(),
                declarations: None,
                runtime: Vec::new(),
                jsx: pass.jsx.contains(&module),
                located: located(tcx, module),
                types_only: false,
                reexports_alone: module == CRATE_MOD_ID && reexports::reexports_alone(tcx, module),
            };
            lowered.taken_into =
                reached::taken_into(tcx, (module, &lowered), (&[&forwarded, &forwarders], &fns), &sources);
            // Each re-export among the imports (rd620).
            reexports::place(tcx, &mut lowered.reexports, &uses, (&fns, &foreign, &paths), &written);
            // A class's prototype its load replaces, set right after its
            // constructor (#448, ADR 0761 amended); an ES class's fields (#568).
            let owner_of = |constructor| fns[&constructor].owner.clone();
            classes::place_prototypes(tcx, &mut lowered.items, module, &pass.prototypes, owner_of, &sources);
            // One of types alone has no JS of its own: what it imports is
            // what its items read, of which it has none (the user's
            // decision, ADR 1005). Not the crate's root, its entry, which a
            // host and a package import: a JS file of nothing, as before.
            lowered.types_only = module != CRATE_MOD_ID
                && lowered.items.is_empty()
                && lowered.packages.is_empty()
                && lowered.reexports.is_empty()
                && lowered.default_export.is_none()
                && lowered.aliases.is_empty()
                && lowered.directives.is_empty();
            // What it exports by another name it exports by that alone,
            // `function getCurrentTime() {}`, whoever else reads it (ADR
            // 0596, amended).
            let aliased = |name: &js::Name| lowered.aliases.iter().any(|(local, _)| name.is(local));
            for item in &mut lowered.items {
                match item {
                    js::Item::Function(function) if aliased(&function.name) => function.export = false,
                    js::Item::Const(constant) if aliased(&constant.name) => constant.export = false,
                    _ => {}
                }
            }
            // Its variables, and a library's it reads (`variables.rs`).
            let (module_variables, made) = variables::place(
                tcx,
                module,
                &mut lowered.items,
                (&fns, &consts, &foreign),
                (&quiet_locals, &thread_local_inits, &internal),
                import_names.get(&module),
            );
            variables.extend(module_variables.keys().copied());
            let exports = library::internal_exports(tcx, (module, &mut lowered), (&internal, &fns, &made));
            internal_exports.extend(exports);
            // What a `&mut` and a `dyn` in a signature are, as the item's
            // body has them: asked of a context of the item (#619, #620).
            let context = |owner: DefId| contexts::item_cx((&shared, &settled), owner, (module, &taken[&module]));
            let mut_ref = |owner: DefId, pointee: ty::Ty<'tcx>| context(owner).mut_ref_form(pointee);
            let pair_trait = |owner: DefId, ty: ty::Ty<'tcx>| context(owner).dynamic_trait(ty);
            let signature = |function: DefId| context(function).js_signature(function);
            let boxed = |owner: DefId, payload: ty::Ty<'tcx>| context(owner).boxed_payload(payload);
            let representation = representation::SignatureForms {
                mut_ref: &mut_ref,
                pair_trait: &pair_trait,
                signature: &signature,
                boxed: &boxed,
            };
            lowered.declarations = declarations::module(
                tcx,
                &foreign,
                module,
                (default_export, default_declared),
                &paths,
                (&declared_reexports, &forwarded),
                &aliased_items,
                (ref_cells, &counted, &null_fields, &flattened),
                (&fns, module_held, &module_variables),
                representation,
            );
            // A `Drop` impl's `drop` of this module no other module calls,
            // which may be its body where its one drop is (ADR 0588).
            let drop_trait = tcx.lang_items().drop_trait();
            let drops: IndexSet<String> = (fns.iter())
                .filter(|&(&id, info)| {
                    info.module == module
                        && drop_trait.is_some_and(|t| {
                            tcx.impl_of_assoc(id)
                                .is_some_and(|imp| tcx.impl_opt_trait_id(imp) == Some(t))
                        })
                        && !called_elsewhere.contains(&id)
                })
                .map(|(_, info)| info.name.clone())
                .collect();
            drops::inlined_drops(&mut lowered.items, &drops);
            std_types::map::discarded_calls(&mut lowered.items);
            // Not what nothing reads (ADR 0722), nor what only a function
            // re-exported in its place read (rd575).
            packages::unread(&mut lowered, (&imported, module), &loaded, &forwarded_reads);
            // A library's item it names through another of the crate's
            // modules that re-exports it, imported from there (rd650).
            let reimported = packages::through_reexports(
                &mut lowered.packages,
                &loaded,
                (imported_through.iter())
                    .filter(|((from, _), _)| *from == module)
                    .filter_map(|(&(_, id), through)| Some((foreign.import_key(id, module)?, id, through))),
            );
            let imports = targets.swap_remove(&module).unwrap_or_default();
            let reexporters = (pass.reexporters.iter()).filter_map(|&(from, to)| (from == module).then_some(to));
            let candidates =
                crate_imports::requests(tcx, module, (imports, reimported, reexporters), &paths, &|of, named| {
                    written(uses.module_import_at(tcx, &fns, of, named))
                });
            UnlinkedModule {
                module: lowered,
                imports: candidates,
                reserved_names: (taken[&module].iter().cloned())
                    .chain(module_fns.names(module).map(str::to_owned))
                    .collect(),
            }
        })
        .collect();
    // What each module reads of another's variables (ADR 0270, amended).
    std_types::cell::crate_imported_variables(tcx, &fns, &variables, &mut lowered);
    // A library's trait method without a default given its caller's
    // location through its dictionary (ADR 0427).
    if export_library {
        library::refuse_unsaid_locations(tcx, &caller_locations, &fns);
    }
    // What the library does with its `RefCell`s, which its consumers agree
    // with (ADR 0863).
    let drop_runs = |t| drops::drop_runs(recognition, (library, &closure_facts), &counted, t);
    let ref_cell_facts = || {
        let no_ref_cells = drops::RefCells::default();
        (ref_cells.get().unwrap_or(&no_ref_cells)).facts(&recognition, &cell_borrows, &drop_runs)
    };
    let own_traits = || drops::own_traits_dropping_nothing(tcx, &drop_runs);
    let tested = program.then(|| (&generic_consts, own_traits()));
    let exported = library::exported_names(&lowered);
    Lowered::Crate(tcx.dcx().has_errors().is_none().then(|| {
        Box::new(Unlinked {
            offered_paths: rust_js_lower::offered::offered_paths(tcx, &foreign, &flattened),
            library: export_library.then(|| {
                library::exports(
                    tcx,
                    (&fns, &exported, &internal_exports),
                    &library::Contracts {
                        drop_params: &kept_drop_params,
                        elided: &library::elided_through_calls(tcx, all_bodies),
                        shown_params: &shown_params.params,
                        dyn_debug_shown: shown_params.dyn_debug(tcx),
                        written: &written_params,
                        makes_fmt_dyns: written_params.makes_dyns || fmt_into_left,
                        given,
                        failing: &failing,
                        writes_may_fail: dictionaries_fail,
                        no_drops: &no_drops,
                        type_facts: &type_facts,
                        carried_super: &carried_super,
                        built_slots: (&foreign, &built_slots),
                        caller_locations: &caller_locations,
                        reads_arguments: &reads_arguments,
                        keeps_capacity,
                        writers_take_options,
                        debug_hex: writer_facts.debug_hex,
                        reads_formatter_options: &writer_facts.reads_options,
                        merges_identities: &merges_identities,
                        untold_locations: &untold_locations,
                        bound: &bound,
                        makes_dropping_anys: closure_facts.any_dyns_drop,
                        makes_dropping_errors: closure_facts.error_dyns_drop,
                        makes_dropping_fn_dyns: closure_facts.fn_dyns_drop.any(),
                        makes_dropping_iterators: closure_facts.iterator_dyns_drop,
                        drops_any,
                        prints: &prints,
                        program: tested,
                        counted: &counted,
                        changed,
                        ref_cells: ref_cell_facts(),
                        fields: library::fields(tcx, ref_cells.get(), cell_borrows.pointers()),
                        lazy_locals: &lazy_locals,
                        at_hand_keys: &cell_borrows.at_hand().keys().collect(),
                        variables: &variables,
                        identities: &identities,
                        keyed: &flattened.keyed,
                        eq_by_address: &eq_by_address,
                        gives_argument: &gives_argument,
                        displays_value: &displays_value,
                        built: cell_borrows.pointers().built().collect(),
                        cloned_closures: closure_facts.clones(),
                        owned_by_return: self_handles.owned_by_return(),
                        nulled_params: &nulled_params.found(&settled.nulled),
                    },
                    (&trait_impls, &held.modules),
                    dependencies,
                )
            }),
            sources: sources.output,
            modules: lowered,
            tests,
        })
    }))
}

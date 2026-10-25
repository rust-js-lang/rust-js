//! Orchestrate analyzed crate facts, function emission, reachability and symbolic module assembly.

use super::analysis::{AnalyzedCrate, analyze_crate, is_thread_local, renamed_in};
use super::bindings::Export;
use super::{Body, CrateFacts, FnCx, FnInfo, Locals, const_js, module_file, module_symbol, static_value};
use crate::js::{self, Expr, Prop, StmtKind};
use crate::program::{ImportRequest, LoweredModule, Reexport, Unlinked, UnlinkedModule};
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::find_attr;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalModId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

/// Retained functions and their dependencies, grouped for output.
#[derive(Default)]
struct Pass {
    functions: HashMap<LocalModId, Vec<js::Function>>,
    namespaces: HashMap<LocalModId, Vec<js::Namespace>>,
    jsx: HashSet<LocalModId>,
    /// What each module runs when it's loaded, each `js::on_load!`'s
    /// (ADR 0267).
    statements: HashMap<LocalModId, Vec<Vec<js::Stmt>>>,
    /// `thread_local!`s' values, made from their lowered `init`s.
    local_consts: HashMap<LocalModId, Vec<js::Const>>,
    /// Each dictionary that's its module's `const`, or whose cache is, by
    /// its impl (ADR 0592), and the names of the `const`s each makes.
    dictionary_consts: Vec<(LocalModId, DefId)>,
    dictionary_names: HashSet<String>,
    /// The functions a block makes and gives, by their items' indices, for
    /// their holes (ADR 0296).
    holes: HashMap<u32, js::Function>,
    /// Which items each module uses from another, and which JS imports.
    references: HashSet<(LocalModId, DefId)>,
    package_uses: HashSet<(LocalModId, Export)>,
    /// The bindings each module reads through another that holds them
    /// (ADR 0633).
    held: HashSet<(LocalModId, LocalModId, String, bool, DefId)>,
}

/// How the crate is built, as its command asks.
pub struct Built<'a> {
    /// For others to use, a library (ADR 0100).
    pub library: bool,
    /// Given std's bodies to lower (ADR 0561).
    pub std_bodies: bool,
    /// Asked to abort on a panic, `-C panic=abort`, which rust-js unwinds
    /// (ADR 0090): unwinding is refused of it (ADR 0660).
    pub panics_abort: bool,
    /// `--tidy-until <stage/pass>`: tidy stops after that pass (ADR 0659).
    pub tidy_until: Option<&'a str>,
}

/// Lower every function, grouped by module. Reports all unsupported
/// features as rustc errors.
pub fn lower_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[Body<'tcx>],
    initializers: &[Body<'tcx>],
    serde_attrs: &super::SerdeAttributes,
    dependencies: &crate::library::Dependencies,
    built: Built<'_>,
) -> Option<Unlinked> {
    let Built {
        library: export_library,
        std_bodies,
        panics_abort,
        tidy_until,
    } = built;
    let sources = super::sources::CapturedSources::new(tcx);
    let AnalyzedCrate {
        bodies,
        closures,
        trait_impls,
        dictionaries,
        import_names,
        imported,
        required,
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
        cell_borrows,
        closure_facts,
        cloned,
        explicit_adapters,
        back_stepped,
        changed_vecs,
        mut drop_params,
        copied,
        stepped,
        type_facts,
        failing,
        throwing,
        caller_locations,
        asks_capacity,
        generic_consts,
        pretty_debug,
        format_options,
        debug_hex,
        quiet_locals,
        lazy_locals,
        named_expressions,
        local_functions,
        held,
    } = analyze_crate(tcx, all_bodies, initializers, dependencies, export_library, std_bodies)?;
    // Each module's default export, which a module of the crate imports as
    // its default, not by a name of its own (ADR 0251), as every importer
    // does (`default_exports::exported_as`).
    let defaulted: HashSet<DefId> = (modules.iter())
        .filter_map(|&module| super::default_exports::own_default(tcx, module))
        .collect();
    called_from_elsewhere.retain(|id| !defaulted.contains(id));
    // A library exports what its consumers can reach (ADR 0100).
    if export_library {
        called_from_elsewhere.extend(fns.keys().copied().filter(|&id| super::library::reachable(tcx, id)));
    }

    let mut const_items: HashMap<LocalModId, Vec<js::Const>> = HashMap::new();
    // The statics and constants whose initializers are lowered as code.
    let mut initialized: Vec<&Body<'tcx>> = Vec::new();
    for &def_id in consts.iter().filter(|&&d| !is_thread_local(tcx, d)) {
        let span = tcx.def_span(def_id);
        let typing_env = ty::TypingEnv::fully_monomorphized();
        let args = ty::GenericArgs::identity_for_item(tcx, def_id);
        // A `static mut` is its value's `{ value }` (ADR 0096), which any
        // module can write.
        let (value, what) = match tcx.def_kind(def_id) {
            DefKind::Static { mutability, .. } => (
                static_value(tcx, def_id)
                    .and_then(|v| const_js(tcx, v))
                    .map(|v| super::literals::const_string(tcx, def_id, v, &|span| sources.span(span)))
                    .map(|v| super::aggregates::const_object(tcx, def_id, v))
                    .map(|v| match mutability.is_mut() {
                        true => Expr::object(vec![Prop::Field(js::HELD.into(), v)]),
                        false => v,
                    }),
                "statics",
            ),
            _ => (
                super::representation::const_value_js(tcx, typing_env, def_id.to_def_id(), args, span)
                    .map(|v| super::literals::const_string(tcx, def_id, v, &|span| sources.span(span)))
                    .map(|v| super::aggregates::const_object(tcx, def_id, v)),
                "constants",
            ),
        };
        let Some(value) = value else {
            let initializer = initializers.iter().find(|body| body.def_id == def_id.to_def_id());
            // A `const { .. }` block's type is its value's.
            let ty = match (tcx.def_kind(def_id), initializer) {
                (DefKind::AnonConst, Some(body)) => body.thir[body.expr].ty,
                _ => tcx.type_of(def_id).instantiate_identity().skip_normalization(),
            };
            // A value rustc's can't say, a function's, a `dyn`'s or one too
            // large for a value tree, is its initializer's, where one JS value
            // can be every use's, and it reads no static, whose value may not
            // be made yet.
            // A `OnceLock`, only ever made empty, or a `LazyLock`, which
            // makes its value when it's used, is one JS cell too (ADRs 0317, 0318),
            // and so is a `static mut`, its value's `{ value }` (ADR 0096).
            let mutable = matches!(tcx.def_kind(def_id), DefKind::Static { mutability, .. } if mutability.is_mut());
            let once = matches!(tcx.def_kind(def_id), DefKind::Static { .. })
                && matches!(ty.kind(), ty::Adt(adt, _)
                if [super::recognition::StdItem::OnceCell, super::recognition::StdItem::LazyCell]
                    .into_iter()
                    .any(|item| super::recognition::is_std_def(tcx, adt.did(), item)));
            // A constant's each use is a value of its own, copied where it may
            // change (`named_const`), so its one JS value needn't be shareable:
            // `const EMPTY: Vec<i32> = Vec::new()`'s `[]` (ADR 0031).
            let constant = !matches!(tcx.def_kind(def_id), DefKind::Static { .. });
            if (mutable || constant || super::copies::shareable(tcx, ty) || once)
                && let Some(body) = initializer
                && !super::body_queries::reads_statics(tcx, &body.thir, tcx.parent_module_from_def_id(def_id))
            {
                initialized.push(body);
                continue;
            }
            super::refusal::refuse(tcx, span, super::refusal::Refused::Type(what, ty));
            failed = true;
            continue;
        };
        // A number is where its literal is, which it's written as (ADR 0627).
        let value = match value.kind {
            js::ExprKind::Num(_) if let Some(body) = tcx.hir_maybe_body_owned_by(def_id) => {
                value.or_at(sources.span(body.value.span))
            }
            _ => value,
        };
        // A `const { .. }` thread-local's value, the `const` in its block, is
        // its variable, by its name.
        let key = thread_local_inits.get(&def_id).copied();
        let named = key.unwrap_or(def_id);
        let info = &fns[&named.to_def_id()];
        const_items.entry(info.module).or_default().push(js::Const {
            name: info.name.clone(),
            value,
            mutable: false,
            export: tcx.visibility(named).is_public() || called_from_elsewhere.contains(&named.to_def_id()),
            span: sources.item_span(tcx, named),
        });
    }

    let function_bodies = bodies.iter().map(|body| (body.def_id, *body)).collect();
    // A trait impl's dictionary (ADR 0049) has no body of its own, so its
    // function context gets an empty one; a copied default brings its own.
    let no_body = rustc_middle::thir::Thir::new(rustc_middle::thir::BodyTy::Const(tcx.types.unit));
    // Lower each body once. Cross-module references have symbolic names until
    // the link step knows exactly which imports and local names survive.
    let mut lowered_items: Vec<(DefId, super::LoweredFn)> = Vec::new();
    let foreign = super::library::Foreign::new(tcx, dependencies);
    // A `Vec`'s capacity is kept where the crate or a library it uses asks
    // it, as what it gives that library may be asked (ADR 0419).
    let keeps_capacity = asks_capacity || foreign.keeps_capacity();
    let takes_used_drops = super::drops::TakesUsed::new(tcx, &drop_params);
    // What each function's lowering found of drops, gathered in the order
    // they're lowered, lowered or not, then resolved (ADRs 0190, 0300).
    let mut drops_observed = super::drops::DropsObserved::default();
    let ref_cells = std::cell::OnceCell::new();
    // What runs where its module loads, which reads another module's
    // `const`s as their values (`FnCx::reads_by_name`).
    // A thread-local made where it's first used runs nothing as its module
    // loads (ADR 0037, amended).
    let loads: HashSet<DefId> = (thread_local_inits.iter())
        .filter(|(_, key)| !lazy_locals.contains(key))
        .map(|(init, _)| init.to_def_id())
        .chain(initialized.iter().map(|body| body.def_id))
        .chain(
            (bodies.iter())
                .map(|body| body.def_id)
                .filter(|&id| super::bindings::is_on_load(tcx, id)),
        )
        .collect();
    let crate_facts = CrateFacts {
        foreign: &foreign,
        library: export_library,
        panics_abort,
        sources: &sources,
        mutated: &mutated,
        counted: &counted,
        cell_borrows: &cell_borrows,
        closure_facts: &closure_facts,
        cloned: &cloned,
        explicit_adapters: &explicit_adapters,
        back_stepped: &back_stepped,
        generic_consts: &generic_consts,
        changed_vecs: &changed_vecs,
        drop_params: &drop_params,
        given: super::traits::Given {
            copied: &copied,
            stepped: &stepped,
        },
        type_facts: &type_facts,
        failing: &failing,
        throwing: &throwing,
        caller_locations: &caller_locations,
        any_failing: !failing.is_empty() || foreign.any_fails(),
        keeps_capacity,
        takes_used_drops: &takes_used_drops,
        closures: &closures,
        bodies: &function_bodies,
        fns: &fns,
        imports: &import_names,
        required: &required,
        held_reads: &held.reads,
        trait_impls: &trait_impls,
        serde_attrs,
        pretty_debug,
        format_options,
        debug_hex,
        tidy_until,
        ref_cells: &ref_cells,
        quiet_locals: &quiet_locals,
        lazy_locals: &lazy_locals,
        named_expressions: &named_expressions,
        local_functions: &local_functions,
        loads: &loads,
    };
    // What a library's manifest says of its writers (ADR 0137).
    let writers_take_options = crate_facts.writers_take_options();
    // Each dictionary's cache, named apart in its module.
    let mut cache_names: HashMap<LocalModId, HashSet<String>> = HashMap::new();
    let cache_of: HashMap<DefId, String> = (dictionaries.iter())
        .map(|&id| {
            let info = &fns[&id];
            let cache = crate::names::dictionary_cache(cache_names.entry(info.module).or_default(), &info.name);
            (id, cache)
        })
        .collect();
    // A generic dictionary's cache's value, its module's `const` (ADR 0592).
    let mut cache_values: HashMap<DefId, Expr> = HashMap::new();
    let mut work: Vec<(DefId, Option<&Body<'tcx>>)> = bodies
        .iter()
        // A trait's default is copied into each impl (ADR 0049); a library's
        // is a function of its own too, for its consumers' impls (ADR 0185).
        .filter(|b| {
            let id = b.def_id;
            tcx.trait_of_assoc(id).is_none() || (export_library && super::library::reachable(tcx, id))
        })
        .map(|b| (b.def_id, Some(*b)))
        .chain(dictionaries.iter().map(|id| (*id, None)))
        .chain(initialized.iter().map(|body| (body.def_id, Some(*body))))
        .collect();
    let (mut used, mut queued) = (HashSet::new(), HashSet::new());
    let no_facts = super::body_queries::BodyFacts::default();
    let mut next = 0;
    loop {
        // Then the codecs something uses, a round at a time, each in the
        // order they're declared. A shared crate derives both for its types,
        // so one that's never used isn't lowered: its type needn't be one
        // rust-js reads or writes. A library's consumers may use one they can
        // reach (ADR 0100).
        if next == work.len() {
            let round: Vec<_> = codecs
                .iter()
                .filter(|&&id| {
                    (used.contains(&id) || (export_library && super::library::reachable(tcx, id))) && queued.insert(id)
                })
                .map(|&id| (id, None))
                .collect();
            if round.is_empty() {
                break;
            }
            work.extend(round);
        }
        let (def_id, body) = work[next];
        next += 1;
        let module = fns[&def_id].module;
        // Locals must never shadow a function or an import of this file; an
        // `on_load!` body's are at its top, beside an earlier body's, which
        // they mustn't be either (ADR 0267).
        let mut names = taken[&module].clone();
        if super::bindings::is_on_load(tcx, def_id) {
            let earlier = lowered_items
                .iter()
                .filter(|(id, _)| super::bindings::is_on_load(tcx, *id) && fns[id].module == module);
            names.extend(earlier.flat_map(|(_, item)| item.function.body.iter().flat_map(declared)));
        }
        let mut cx = FnCx {
            tcx,
            typing_env: ty::TypingEnv::post_analysis(tcx, def_id),
            given: Default::default(),
            codecs: Vec::new(),
            krate: &crate_facts,
            dependencies: Default::default(),
            drops_observed: Default::default(),
            frame: super::BodyFrame {
                thir: body.map_or(&no_body, |body| &body.thir),
                facts: body.map_or(&no_facts, |body| &body.facts),
                owner: def_id,
            },
            module,
            locals: Locals::default(),
            names,
            module_names: &taken[&module],
            jsx: false,
            writing: Default::default(),
            cloning: Vec::new(),
            ordering: Vec::new(),
            item: def_id,
            walks: Default::default(),
            drop_state: Default::default(),
        };
        let result = match body {
            // Each body, a function's or an initializer's, from its MIR (ADR 0364).
            Some(body) if let Some(mir) = &body.mir => cx.lower_fn_mir(body, mir),
            Some(_) => Err(cx.unsupported(tcx.def_span(def_id), "this body, without its MIR")),
            None if codecs.contains(&def_id) => cx.lower_codec(def_id).map(|function| super::LoweredFn {
                function,
                jsx: cx.jsx,
                dependencies: cx.dependencies.take(),
            }),
            None => cx
                .lower_dictionary(def_id, &cache_of[&def_id])
                .map(|(function, cache)| {
                    if let Some(cache) = cache {
                        cache_values.insert(def_id, cache);
                    }
                    super::LoweredFn {
                        function,
                        jsx: cx.jsx,
                        dependencies: cx.dependencies.take(),
                    }
                }),
        };
        // Of one that failed too, whose calls are checked as each other's.
        drops_observed.gather(cx.drops_observed.take());
        match result {
            Ok(lowered) => {
                used.extend(lowered.dependencies.uses.iter().map(|&(_, to)| to));
                lowered_items.push((def_id, lowered));
            }
            Err(_) => failed = true,
        }
    }
    // What a function takes no destructor of, each call of it checked (ADR 0190).
    let checks = std::mem::take(&mut drops_observed.checks);
    failed |= super::drops::check_no_drops(tcx, &mut drops_observed.no_drops, &checks);
    if failed {
        return None;
    }
    let no_drops = std::mem::take(&mut drops_observed.no_drops);
    // Only the drops each function uses (ADR 0300).
    drops_observed.keep(&takes_used_drops, &mut drop_params, &mut lowered_items);

    // What's written only where it's reached: derived Debug implementations,
    // a marker trait's dictionaries, which only a `dyn` reads (ADR 0307),
    // a `Hash`'s and a `Hasher`'s, which a map never calls (ADR 0463),
    // and those only generic code reads, their methods called as they are
    // anywhere else (`read_only_generically`); and a `PartialEq` of a type
    // equal only to itself, whose `==` is `===` and whose sets and maps
    // are JS's (ADR 0864, amended).
    // Adjacency lists rather than scanning every edge again for every reached
    // function.
    let recognition = super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    let identity_eq = |id: DefId, trait_id: DefId| {
        tcx.is_lang_item(trait_id, rustc_hir::attrs::lang_items::LangItem::PartialEq)
            && super::std_impls::equal_only_to_itself(
                &recognition,
                &function_bodies,
                tcx.type_of(id).instantiate_identity().skip_normalization(),
            )
    };
    let derived: HashSet<DefId> = trait_impls
        .iter()
        .filter(|&&id| {
            let trait_id = tcx
                .impl_trait_ref(id)
                .instantiate_identity()
                .skip_normalization()
                .def_id;
            super::recognition::known_derive(tcx, id)
                || super::traits::is_marker(tcx, &foreign, trait_id)
                || super::recognition::is_hashing(tcx, trait_id)
                || identity_eq(id, trait_id)
        })
        .flat_map(|&id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied()))
        .chain(trait_impls.iter().copied().filter(|&id| {
            let trait_id = tcx
                .impl_trait_ref(id)
                .instantiate_identity()
                .skip_normalization()
                .def_id;
            super::traits::read_only_generically(tcx, trait_id)
        }))
        .collect();
    let edges = lowered_items
        .iter()
        .flat_map(|(_, item)| item.dependencies.uses.iter().copied());
    // What each module's own items read by name of its own: a `const` the
    // module's default holds, read so, is declared, then exported (ADR 0625,
    // amended).
    let read_at_home: HashSet<DefId> = (edges.clone())
        .filter(|&(from, to)| {
            from.as_local()
                .zip(to.as_local())
                .is_some_and(|(from, to)| tcx.parent_module_from_def_id(from) == tcx.parent_module_from_def_id(to))
        })
        .map(|(_, to)| to)
        .collect();
    let roots = edges
        .clone()
        .filter(|(from, _)| !derived.contains(from))
        .map(|(_, to)| to)
        // A library's consumers may reach one (ADR 0100).
        .chain(
            derived
                .iter()
                .copied()
                .filter(|&id| export_library && super::library::reachable(tcx, id)),
        );
    let reached = crate::reachability::reachable(roots, edges);
    let mut pass = Pass::default();
    for (def_id, mut lowered) in lowered_items
        .into_iter()
        .filter(|(id, _)| !derived.contains(id) || reached.contains(id))
    {
        let module = fns[&def_id].module;
        let initializer = initialized.iter().any(|body| body.def_id == def_id);
        pass.references.extend(lowered.dependencies.references.drain());
        pass.package_uses.extend(lowered.dependencies.package_uses.drain());
        pass.held.extend(lowered.dependencies.held.drain());
        // A dictionary given nothing is its module's `const`, what its entries
        // share beside it; one given something, its function, its cache a
        // `const` beside it (ADR 0592).
        if dictionaries.contains(&def_id) {
            let span = def_id
                .as_local()
                .map_or(js::Span::NONE, |local| sources.item_span(tcx, local));
            let constant = |name: String, value: Expr, export: bool| js::Const {
                name,
                value,
                mutable: false,
                export,
                span,
            };
            pass.dictionary_consts.push((module, def_id));
            let consts = pass.local_consts.entry(module).or_default();
            let made = consts.len();
            if let Some(cache) = cache_values.remove(&def_id) {
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
                        consts.push(constant(name, value, false));
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
                    (false, _) => Expr::call(Expr::arrow(Vec::new(), function.body), Vec::new()),
                };
                // A `static mut`'s is its `{ value }` (ADR 0096).
                let value = match tcx.static_mutability(key.to_def_id()) {
                    Some(mutability) if mutability.is_mut() => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),
                    _ => value,
                };
                let info = &fns[&key.to_def_id()];
                pass.local_consts.entry(info.module).or_default().push(js::Const {
                    name: info.name.clone(),
                    value,
                    mutable: false,
                    // A `const { .. }` block has no visibility: it's its function's alone.
                    export: (tcx.def_kind(key) != DefKind::AnonConst && tcx.visibility(key).is_public())
                        || called_from_elsewhere.contains(&key.to_def_id()),
                    span: sources.item_span(tcx, key),
                });
                if lowered.jsx {
                    pass.jsx.insert(module);
                }
            }
            mut lowered => {
                if lowered.jsx {
                    pass.jsx.insert(module);
                }
                lowered.function.export |= called_from_elsewhere.contains(&def_id);
                match &fns[&def_id].owner {
                    // Its type's object is exported if any of its methods is.
                    Some(owner) => {
                        // A class's, its constructor or its prototype's
                        // (ADR 0761).
                        let member = super::classes::member(tcx, def_id);
                        if let Some(member) = member
                            && let Err(what) = super::classes::as_member(member, &mut lowered.function)
                        {
                            super::refusal::refuse(tcx, tcx.def_span(def_id), super::refusal::Refused::Feature(what));
                            failed = true;
                            continue;
                        }
                        let module_namespaces = pass.namespaces.entry(module).or_default();
                        let namespace = match module_namespaces.iter().position(|n| n.name == *owner) {
                            Some(at) => &mut module_namespaces[at],
                            None => {
                                module_namespaces.push(js::Namespace {
                                    name: owner.clone(),
                                    constants: Vec::new(),
                                    methods: Vec::new(),
                                    export: false,
                                    class: None,
                                });
                                module_namespaces.last_mut().expect("just pushed")
                            }
                        };
                        namespace.export |= lowered.function.export;
                        if let Some(member) = member {
                            let class = namespace.class.get_or_insert_with(|| js::Class {
                                constructor: String::new(),
                                prototype: Vec::new(),
                                declared: (super::classes::class_of(tcx, def_id).and_then(|adt| adt.as_local()))
                                    .map_or(js::Span::NONE, |adt| sources.item_span(tcx, adt)),
                            });
                            match member {
                                super::classes::Member::Constructor => {
                                    class.constructor = lowered.function.name.clone()
                                }
                                super::classes::Member::Method => class.prototype.push(lowered.function.name.clone()),
                                super::classes::Member::Static | super::classes::Member::Const => {}
                            }
                        }
                        namespace.methods.push(lowered.function);
                    }
                    None if super::bindings::is_on_load(tcx, def_id) => {
                        pass.statements.entry(module).or_default().push(lowered.function.body)
                    }
                    // Named in itself alone, as the Rust names it, unless
                    // what it reads is named so.
                    // Its function's, where it's written there (ADR 0308).
                    None if local_functions.contains_key(&def_id) => {
                        let mut function = lowered.function;
                        function.export = false;
                        pass.holes.insert(def_id.index.as_u32(), function);
                    }
                    None if named_expressions.contains(&def_id) => {
                        let mut function = lowered.function;
                        let mut read = HashSet::new();
                        js::visit_stmts(&function.body, &mut |name| {
                            read.insert(name.to_owned());
                        });
                        function.name = super::fresh_in(&mut read, &function.name);
                        function.export = false;
                        pass.holes.insert(def_id.index.as_u32(), function);
                    }
                    None => pass.functions.entry(module).or_default().push(lowered.function),
                }
            }
        }
    }
    if failed {
        return None;
    }
    for (module, consts) in pass.local_consts.drain() {
        const_items.entry(module).or_default().extend(consts);
    }
    // A numbered enum's variant, its module's `const` of its number, where
    // something reads it or another module or crate may, exported then, as
    // a `const` item is (ADR 0722, amended).
    let read_elsewhere: HashSet<DefId> = pass.references.iter().map(|&(_, id)| id).collect();
    for &variant in &variants {
        let id = variant.to_def_id();
        let export =
            tcx.visibility(id).is_public() || called_from_elsewhere.contains(&id) || read_elsewhere.contains(&id);
        if !export && !reached.contains(&id) {
            continue;
        }
        let info = &fns[&id];
        let adt = tcx.adt_def(tcx.parent(id));
        const_items.entry(info.module).or_default().push(js::Const {
            name: info.name.clone(),
            value: super::bindings::variant_tag(tcx, adt.variant_with_id(id)),
            mutable: false,
            export,
            span: sources.item_span(tcx, variant),
        });
    }
    // An inherent impl's constant is its type's object's, `S.ID`, as its
    // methods are (ADR 0410).
    for &def_id in &consts {
        let info = &fns[&def_id.to_def_id()];
        let Some(owner) = &info.owner else { continue };
        let Some(items) = const_items.get_mut(&info.module) else {
            continue;
        };
        let Some(at) = items.iter().position(|c| c.name == info.name) else {
            continue;
        };
        let constant = items.remove(at);
        let module_namespaces = pass.namespaces.entry(info.module).or_default();
        match module_namespaces.iter_mut().find(|n| n.name == *owner) {
            Some(namespace) => {
                namespace.export |= constant.export;
                namespace.constants.push(constant);
            }
            None => module_namespaces.push(js::Namespace {
                name: owner.clone(),
                export: constant.export,
                constants: vec![constant],
                methods: Vec::new(),
                class: None,
            }),
        }
    }
    // Each function a block makes and gives, where its block was (ADR 0296).
    let holes = &mut pass.holes;
    for constant in const_items.values_mut().flatten() {
        constant.value.each_mut(&mut |e| fill_hole(e, holes));
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

    for &(_, id) in pass.references.iter() {
        if defaulted.contains(&id) {
            continue;
        }
        let info = &fns[&id];
        if let Some(owner) = &info.owner {
            if let Some(ns) = pass
                .namespaces
                .get_mut(&info.module)
                .and_then(|ns| ns.iter_mut().find(|ns| ns.name == *owner))
            {
                ns.export = true;
            }
        } else if let Some(f) = pass
            .functions
            .get_mut(&info.module)
            .and_then(|fs| fs.iter_mut().find(|f| f.name == info.name))
        {
            f.export = true;
        }
    }
    // What each module imports of another: what that module exports its
    // item as, its name, its default or the name a `pub use` gives it
    // (`default_exports::exported_as`), each by what the importer's `use`
    // names it, `import { SandpackClient as Sandpack }`, or by its name,
    // `import { unstable_now as getCurrentTime }`.
    let mut targets: HashMap<LocalModId, HashSet<(LocalModId, String, String, String)>> = HashMap::new();
    for &(from, id) in &pass.references {
        let info = &fns[&id];
        let name = info.owner.as_ref().unwrap_or(&info.name).clone();
        let (export, local) = match info.owner {
            Some(_) => (name.clone(), name.clone()),
            None => (
                super::default_exports::exported_as(tcx, id, &name),
                renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone()),
            ),
        };
        targets
            .entry(from)
            .or_default()
            .insert((info.module, name, export, local));
    }
    // And a binding it reads through another that holds it, by its export
    // there, named as the importer's `use` names it (ADR 0633).
    for (from, holder, name, default, binding) in &pass.held {
        let local = renamed_in(tcx, *from).remove(binding).unwrap_or_else(|| name.clone());
        let export = if *default { "default".to_string() } else { name.clone() };
        (targets.entry(*from).or_default()).insert((*holder, name.clone(), export, local));
    }
    // What another module than its own calls, which stays a function.
    let called_elsewhere: HashSet<DefId> = (pass.references.iter())
        .filter(|&&(from, id)| fns[&id].module != from)
        .map(|&(_, id)| id)
        .collect();
    refuse_dictionaries_in_load_cycles(tcx, &pass, &const_items, &targets);
    let lowered = modules
        .into_iter()
        .map(|module| {
            // One `import` per JS module, of what this file uses from it.
            let mut packages: BTreeMap<&str, js::Package> = BTreeMap::new();
            for export in imported
                .iter()
                .filter(|(export, users)| {
                    users.contains(&module) || pass.package_uses.contains(&(module, (*export).clone()))
                })
                .map(|(export, _)| export)
            {
                let (from, name) = export;
                let package = packages.entry(from).or_insert_with(|| js::Package {
                    from: from.clone(),
                    default: None,
                    named: Vec::new(),
                    namespace: None,
                });
                let local = import_names[&module][export].clone();
                match name.as_str() {
                    "default" => package.default = Some(local),
                    "*" => package.namespace = Some(local),
                    _ => package.named.push((name.clone(), local)),
                }
            }
            let mut packages: Vec<js::Package> = packages.into_values().collect();
            // `js::import!("./App.css");`: `import "./App.css";`, for what a
            // module does when loaded, as a bundler's CSS does (ADR 0110).
            let mut loaded = HashSet::new();
            for attr in super::bindings::marks(tcx, module, "import") {
                let Some(from) = attr.value_str().map(|s| s.to_string()) else {
                    tcx.dcx()
                        .span_err(attr.span(), "rust-js: write it `js::import!(\"./file.css\");`");
                    continue;
                };
                loaded.insert(from.clone());
                if !packages.iter().any(|p| p.from == from) {
                    packages.push(js::Package {
                        from,
                        default: None,
                        named: Vec::new(),
                        namespace: None,
                    });
                }
            }
            // `js::directive!("use client");` and `js::export_default!(page);`,
            // what a Next.js route's module has (ADR 0192).
            let mut directives = Vec::new();
            for (directive, span) in super::bindings::directives_of(tcx, module.to_local_def_id()) {
                match directive {
                    Some(directive) => directives.push(directive.to_string()),
                    None => {
                        tcx.dcx()
                            .span_err(span, "rust-js: write it `js::directive!(\"use client\");`");
                    }
                }
            }
            // One in a body is its function's or its closure's (ADR 0349);
            // one in an `async` block's, or a constant's, has none.
            for item in tcx.hir_module_free_items(module) {
                let owner = super::bindings::directive_owner(tcx, item.owner_id.def_id);
                let owned = owner == module.to_local_def_id()
                    || matches!(tcx.def_kind(owner), DefKind::Fn | DefKind::AssocFn)
                    || (tcx.def_kind(owner) == DefKind::Closure && tcx.coroutine_kind(owner).is_none());
                if !owned
                    && let Some(attr) = (tcx.get_attrs_by_path(
                        item.owner_id.to_def_id(),
                        &[Symbol::intern("rust_js"), Symbol::intern("directive")],
                    ))
                    .next()
                {
                    tcx.dcx().span_err(
                        attr.span(),
                        "rust-js: a directive is a module's, a function's or a closure's, written first in it",
                    );
                }
            }
            // What it holds of a binding as a value of its own (ADRs 0622, 0633).
            let module_held = held.modules.get(&module).map_or(&[][..], Vec::as_slice);
            let mut default_export = None;
            let mut default_declared = false;
            // Another module's function, re-exported as the default (ADR 0240).
            let mut default_from = None;
            for (function, span) in super::default_exports::default_exports(tcx, module) {
                let unset = default_export.is_none() && default_from.is_none();
                match function {
                    Some(function)
                        if unset
                            && function
                                .as_local()
                                .is_some_and(|local| tcx.parent_module_from_def_id(local) == module) =>
                    {
                        default_export = Some(function);
                        default_declared = super::default_exports::default_declared(tcx, function);
                    }
                    Some(function) if unset && fns.get(&function).is_some_and(|info| info.owner.is_none()) => {
                        default_from = Some(function);
                    }
                    // A binding, its `const` (ADRs 0622, 0633), or one it names
                    // through another module that holds it, re-exported.
                    Some(function) if unset && super::bindings::held_path(tcx, function).is_some() => {
                        let reexported = (module_held.iter())
                            .any(|held| held.binding == function && held.default && held.through.is_some());
                        if !reexported {
                            default_export = Some(function);
                        }
                    }
                    _ => {
                        tcx.dcx().span_err(
                            span,
                            "rust-js: `js::export_default!` names one function or thread-local of its own module, once",
                        );
                    }
                }
            }
            let reexports = reexports(tcx, module, &fns, &paths, (default_from, module_held));
            let aliased_items = aliases(tcx, module, &fns);
            // A binding it holds, its `const` of what the binding reads (ADRs
            // 0622, 0633); not one it re-exports from another that holds it.
            let names = import_names.get(&module);
            const_items.entry(module).or_default().extend(
                (module_held.iter())
                    .filter(|held| held.through.is_none())
                    .map(|held| js::Const {
                        name: held.name.clone(),
                        value: super::path_read(&held.path, &required, |export| {
                            names.expect("its imports")[export].clone()
                        }),
                        export: held.exported_by_name(),
                        mutable: false,
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
            // alone, whatever its visibility: `const v = ..; export default
            // v;`, never `export const v` too, as every importer imports it
            // as the default (ADRs 0611, 0773, amended). A binding's is
            // `HeldBinding::exported_by_name`'s.
            if let Some(own) = default_export.filter(|&own| super::bindings::held_path(tcx, own).is_none()) {
                let name = super::bindings::fn_name(tcx, own);
                for constant in const_items
                    .entry(module)
                    .or_default()
                    .iter_mut()
                    .filter(|c| c.name == name)
                {
                    constant.export = false;
                }
                for function in (pass.functions.entry(module).or_default().iter_mut()).filter(|f| f.name == name) {
                    function.export = false;
                }
            }
            let no_ref_cells = super::drops::RefCells::default();
            let ref_cells = ref_cells.get().unwrap_or(&no_ref_cells);
            let declarations = super::declarations::module(
                tcx,
                module,
                (default_export, default_declared),
                &paths,
                &reexports,
                &aliased_items,
                (ref_cells, &counted),
                (&fns, module_held),
            );
            let aliases: Vec<(String, String)> = aliased_items.into_iter().map(|(_, names)| names).collect();
            let mut lowered = LoweredModule {
                path: paths[&module].clone(),
                file: module_file(tcx, module).name.clone().into_local_path(),
                code: sources.module_code(tcx, module),
                directives,
                packages,
                imports: Vec::new(),
                reexports,
                aliases,
                items: js::in_load_order(
                    (pass
                        .namespaces
                        .remove(&module)
                        .into_iter()
                        .flatten()
                        .map(js::Item::Namespace))
                    .chain(const_items.remove(&module).into_iter().flatten().map(js::Item::Const))
                    .chain(
                        pass.statements
                            .remove(&module)
                            .into_iter()
                            .flatten()
                            .map(js::Item::Statements),
                    )
                    .chain(
                        pass.functions
                            .remove(&module)
                            .into_iter()
                            .flatten()
                            .map(js::Item::Function),
                    )
                    .collect(),
                ),
                default_export: default_export.map(|function| super::bindings::fn_name(tcx, function)),
                default_declared: default_declared && read_default.is_none(),
                declarations,
                runtime: Vec::new(),
                jsx: pass.jsx.contains(&module),
                located: located(tcx, module),
            };
            // What it exports by another name it exports by that alone,
            // `function getCurrentTime() {}`, whoever else reads it (ADR
            // 0596, amended).
            let aliased = |name: &str| lowered.aliases.iter().any(|(local, _)| local == name);
            for item in &mut lowered.items {
                match item {
                    js::Item::Function(function) if aliased(&function.name) => function.export = false,
                    js::Item::Const(constant) if aliased(&constant.name) => constant.export = false,
                    _ => {}
                }
            }
            let quiet: HashSet<&str> = (quiet_locals.iter())
                .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
                .map(|key| fns[&key.to_def_id()].name.as_str())
                .collect();
            super::std_types::cell::plain_statics(&mut lowered.items, &quiet);
            // A `Drop` impl's `drop` of this module no other module calls,
            // which may be its body where its one drop is (ADR 0588).
            let drop_trait = tcx.lang_items().drop_trait();
            let drops: HashSet<String> = (fns.iter())
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
            super::drops::inlined_drops(&mut lowered.items, &drops);
            super::std_types::map::discarded_calls(&mut lowered.items);
            // A library's item its lowering asked for, where tidy took out
            // what read it, a test of a numbered enum's variant an exhaustive
            // `match` needn't write (ADR 0722): not imported, as the linker
            // imports none of the crate's modules' the module doesn't read.
            let mut read = HashSet::new();
            for item in &lowered.items {
                item.visit_vars(&mut |name| {
                    read.insert(name.to_string());
                });
            }
            let analyzed = |from: &str, export: &str| {
                (imported.get(&(from.to_string(), export.to_string()))).is_some_and(|users| users.contains(&module))
            };
            lowered.packages.retain_mut(|package| {
                let named = package.named.len();
                (package.named).retain(|(export, local)| read.contains(local) || analyzed(&package.from, export));
                named == package.named.len()
                    || !package.named.is_empty()
                    || package.default.is_some()
                    || package.namespace.is_some()
                    || loaded.contains(&package.from)
            });
            let mut imports: Vec<_> = targets.remove(&module).unwrap_or_default().into_iter().collect();
            // By what each imports, `import { MAX as LIMIT, unstable_now as getCurrentTime }`.
            imports.sort_by(|(a, an, ae, _), (b, bn, be, _)| (&paths[a], ae, an).cmp(&(&paths[b], be, bn)));
            let candidates: Vec<_> = imports
                .into_iter()
                .map(|(target, name, export, local)| ImportRequest {
                    symbol: module_symbol(target, &name),
                    export,
                    local,
                    path: paths[&target].clone(),
                })
                .collect();
            UnlinkedModule {
                module: lowered,
                imports: candidates,
                reserved_names: taken[&module].clone(),
            }
        })
        .collect();
    // A library's trait method without a default whose entries take its
    // caller's location, which its manifest, of functions, can't say to its
    // consumers' calls through them (ADR 0427).
    if export_library {
        let mut unsaid: Vec<DefId> = (caller_locations.iter())
            .filter(|&&id| tcx.trait_of_assoc(id).is_some() && !fns.contains_key(&id))
            .filter(|&&id| super::library::reachable(tcx, id))
            .copied()
            .collect();
        unsaid.sort_by_key(|&id| tcx.def_span(id));
        for id in unsaid {
            super::refusal::refuse(
                tcx,
                tcx.def_span(id),
                super::refusal::Refused::Feature(
                    "a library's trait method without a default that's given its caller's location through its dictionary",
                ),
            );
        }
    }
    // What the library does with its `RefCell`s, which its consumers agree
    // with (ADR 0863).
    let ref_cell_facts = || {
        let recognition = super::recognition::Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: &trait_impls,
            foreign: &foreign,
        };
        let no_ref_cells = super::drops::RefCells::default();
        ref_cells
            .get()
            .unwrap_or(&no_ref_cells)
            .facts(&recognition, &cell_borrows, &|t| {
                super::drops::drop_runs(recognition, (export_library, &closure_facts), &counted, t)
            })
    };
    tcx.dcx().has_errors().is_none().then_some(Unlinked {
        library: export_library.then(|| {
            super::library::exports(
                tcx,
                &fns,
                &super::library::Contracts {
                    drop_params: &drop_params,
                    failing: &failing,
                    no_drops: &no_drops,
                    type_facts: &type_facts,
                    caller_locations: &caller_locations,
                    stepped: &stepped,
                    keeps_capacity,
                    writers_take_options,
                    makes_dropping_anys: closure_facts.any_dyns_drop,
                    counted: &counted,
                    ref_cells: ref_cell_facts(),
                    lazy_locals: &lazy_locals,
                },
                (&trait_impls, &held.modules),
                dependencies,
            )
        }),
        sources: sources.output,
        modules: lowered,
        tests,
    })
}

/// What `module` re-exports, its public `pub use` of another module's
/// function, by that module's path, or of a binding of a JS module's
/// export, by its specifier: each `(export, alias)`, the function's JS name
/// or the export, and the name the `use` gives it (ADR 0240); and
/// `default`, another module's function its `js::export_default!` names.
/// The crate's modules first, then JS modules, each in its order.
fn reexports(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    fns: &HashMap<DefId, FnInfo>,
    paths: &HashMap<LocalModId, Vec<String>>,
    (default_from, held): (Option<DefId>, &[super::bindings::HeldBinding]),
) -> Vec<Reexport> {
    let mut grouped: BTreeMap<Vec<String>, Vec<(String, String)>> = BTreeMap::new();
    let mut specified: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    // A binding it names through another module that holds it, that
    // module's export, by its name and as its default (ADR 0633).
    for held in held {
        if let Some(super::bindings::HeldRead {
            holder: super::bindings::Holder::Module(holder),
            import,
            ..
        }) = &held.through
            && let Some(from) = paths.get(holder)
        {
            let named = grouped.entry(from.clone()).or_default();
            if held.export {
                named.push((import.clone(), held.name.clone()));
            }
            if held.default {
                named.push((import.clone(), "default".to_string()));
            }
        }
    }
    // Each function of another module as that module exports it, its
    // default where it's that one's (`default_exports::exported_as`).
    let exported = |function: DefId, info: &FnInfo| super::default_exports::exported_as(tcx, function, &info.name);
    if let Some(function) = default_from
        && let Some(info) = fns.get(&function)
        && let Some(from) = paths.get(&info.module)
    {
        grouped
            .entry(from.clone())
            .or_default()
            .push((exported(function, info), "default".to_string()));
    }
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Use(path, hir::UseKind::Single(ident)) = item.kind else {
            continue;
        };
        let Some(Res::Def(_, def_id)) = path.res.value_ns else {
            continue;
        };
        if !super::analysis::is_reexport(tcx, item) {
            continue;
        }
        // A binding of a JS module's export, itself, not a path in it.
        if super::bindings::is_binding(tcx, def_id)
            && let Some(written) = super::bindings::js_path(tcx, def_id)
            && let Some(((from, export), "")) = super::bindings::js_import(&written)
        {
            if export == "*" {
                let why = "it'd be `export * as ns from`; re-export each of its exports by name";
                let what = "a `pub use` of a JS module's namespace";
                super::refusal::refuse(tcx, item.span, super::refusal::Refused::Because(what, why));
                continue;
            }
            specified.entry(from).or_default().push((export, ident.to_string()));
            continue;
        }
        let Some(info) = fns.get(&def_id) else { continue };
        if info.owner.is_some() {
            continue;
        }
        // Its own item by another name is `aliases`'s.
        if info.module == module {
            continue;
        }
        let alias = match ident.name == tcx.item_name(def_id) {
            true => info.name.clone(),
            false => super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),
        };
        let Some(from) = paths.get(&info.module) else { continue };
        grouped
            .entry(from.clone())
            .or_default()
            .push((exported(def_id, info), alias));
    }
    let modules = (grouped.into_iter()).map(|(path, named)| Reexport {
        from: js::ImportFrom::Module(path),
        named,
    });
    let specifiers = (specified.into_iter()).map(|(from, named)| Reexport {
        from: js::ImportFrom::Specifier(from),
        named,
    });
    modules.chain(specifiers).collect()
}

/// `module`'s own items it exports by other names, `export { getCurrentTime
/// as unstable_now };`, each `(local, exported)` and the item, in the order
/// its `pub use`s are written: by those alone, not by its own name (ADR
/// 0596, amended; `default_exports::own_aliases`).
fn aliases(tcx: TyCtxt<'_>, module: LocalModId, fns: &HashMap<DefId, FnInfo>) -> Vec<(DefId, (String, String))> {
    (super::default_exports::own_aliases(tcx, module).into_iter())
        .filter_map(|alias| {
            let info = fns.get(&alias.item).filter(|info| info.owner.is_none())?;
            Some((alias.item, (info.name.clone(), alias.exported)))
        })
        .collect()
}

/// Each hole in `stmts` filled with its function, and each in that (ADR 0296).
fn fill_holes(stmts: &mut Vec<js::Stmt>, holes: &mut HashMap<u32, js::Function>) {
    // A function written in a body, a declaration there, before what's left
    // is taken for an expression (ADR 0308); `each_block_mut` goes on into
    // its body, for those written in it.
    js::each_block_mut(stmts, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::Expr(e) = &stmt.kind
                && let js::ExprKind::FunctionHole(index) = e.kind
                && let Some(function) = holes.remove(&index)
            {
                // Where it's written, which its comments are before.
                stmt.span = function.span;
                stmt.kind = StmtKind::Function(Box::new(function));
            }
        }
    });
    js::each_expr_mut(stmts, &mut |e| fill_hole(e, holes));
}

fn fill_hole(e: &mut Expr, holes: &mut HashMap<u32, js::Function>) {
    if let js::ExprKind::FunctionHole(index) = e.kind
        && let Some(mut function) = holes.remove(&index)
    {
        fill_holes(&mut function.body, holes);
        e.kind = js::ExprKind::Function(Box::new(function));
    }
}

/// The variables a statement declares where it is: `const x`, `let [a, b]`.
fn declared(statement: &js::Stmt) -> Vec<String> {
    match &statement.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.clone()],
        StmtKind::Destructure { pattern, .. } => pattern.names().into_iter().map(str::to_owned).collect(),
        _ => Vec::new(),
    }
}

/// Whether a `#[path]` placed `module`'s file (ADR 0273), its own or an
/// enclosing module's, `sandpack-rsc/bridge.rs` of `#[path = "sandpack-rsc"]
/// mod sandpack_rsc { mod bridge; }`: where no module path can name it. Not
/// an inline module's, which is its parent's file.
fn located(tcx: TyCtxt<'_>, module: LocalModId) -> bool {
    if module == CRATE_MOD_ID {
        return false;
    }
    let parent = tcx.parent_module_from_def_id(module.to_local_def_id());
    if Arc::ptr_eq(&module_file(tcx, module), &module_file(tcx, parent)) {
        return false;
    }
    let mut at = module;
    loop {
        if find_attr!(tcx, at.to_def_id(), Path(..)) {
            return true;
        }
        if at == CRATE_MOD_ID {
            return false;
        }
        at = tcx.parent_module_from_def_id(at.to_local_def_id());
    }
}

/// A dictionary that's its module's `const` (ADR 0592) is made when its
/// module loads. Where its module and one whose load runs code, a
/// `js::on_load!` or a value made by a call, import each other, the other
/// may load first and reach it before it's made: refused, naming both, as
/// JS's would throw where Rust's runs.
fn refuse_dictionaries_in_load_cycles(
    tcx: TyCtxt<'_>,
    pass: &Pass,
    const_items: &HashMap<LocalModId, Vec<js::Const>>,
    targets: &HashMap<LocalModId, HashSet<(LocalModId, String, String, String)>>,
) {
    // A dictionary, its cache and what its entries share run nothing of the
    // program's where they're made: not code a cycle's load runs.
    let runs_code = |module: &LocalModId| {
        pass.statements.get(module).is_some_and(|stmts| !stmts.is_empty())
            || const_items.get(module).is_some_and(|consts| {
                (consts.iter()).any(|c| !pass.dictionary_names.contains(&c.name) && c.value.has_effects())
            })
    };
    let reaches = |from: LocalModId, to: LocalModId| {
        let mut seen = HashSet::from([from]);
        let mut pending = vec![from];
        while let Some(at) = pending.pop() {
            for &(next, ..) in targets.get(&at).into_iter().flatten() {
                if next == to {
                    return true;
                }
                if seen.insert(next) {
                    pending.push(next);
                }
            }
        }
        false
    };
    let loaders: Vec<LocalModId> = targets
        .keys()
        .chain(pass.statements.keys())
        .copied()
        .filter(runs_code)
        .collect();
    for &(module, impl_id) in &pass.dictionary_consts {
        if let Some(&other) = loaders
            .iter()
            .find(|&&other| other != module && reaches(module, other) && reaches(other, module))
        {
            let what = format!(
                "this impl's dictionary, made where `{}` loads, in an import cycle with `{}`, which runs code where it loads",
                tcx.def_path_str(module.to_def_id()),
                tcx.def_path_str(other.to_def_id()),
            );
            let why = "that code may reach the dictionary before it's made: move one out of the cycle (ADR 0592)";
            super::refusal::refuse(tcx, tcx.def_span(impl_id), super::refusal::Refused::Because(&what, why));
        }
    }
}

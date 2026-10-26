//! Orchestrate analyzed crate facts, function emission, reachability and symbolic module assembly.

mod holes;
mod load_cycles;

use self::holes::{Holes, fill_hole, fill_holes};
use self::load_cycles::{ModuleJs, module_imports};
use super::analysis::{AnalyzedCrate, analyze_crate, is_thread_local, renamed_in};
use super::bindings::Export;
use super::{Body, CrateFacts, FnCx, FnInfo, Locals, const_js, module_file, module_symbol, static_value};
use crate::js::{self, Expr, Prop, StmtKind};
use crate::program::{ImportRequest, LoweredModule, Unlinked, UnlinkedModule};
use rustc_hir::def::DefKind;
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
    /// its impl (ADR 0592), that the crate's own code reaches; and the
    /// names of the `const`s each dictionary makes.
    dictionary_consts: Vec<(LocalModId, DefId)>,
    dictionary_names: HashSet<String>,
    /// What fills each item's hole, by its index.
    holes: Holes,
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
    /// What the JS modules its bindings import load of the crate's modules
    /// (ADR 0031, amended).
    pub js_modules: &'a dyn crate::program::JsModules,
}

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
    (all_bodies, initializers): (&[Body<'tcx>], &[Body<'tcx>]),
    serde_attrs: &super::SerdeAttributes,
    (dependencies, instances, made_where_used): (
        &crate::library::Dependencies,
        &super::Instances,
        &HashSet<rustc_span::def_id::LocalDefId>,
    ),
    built: Built<'_>,
) -> Lowered {
    let Built {
        library: export_library,
        std_bodies,
        panics_abort,
        tidy_until,
        js_modules,
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
        clones,
        stepped,
        type_facts,
        failing,
        throwing,
        never_unwinding,
        caller_locations,
        asks_capacity,
        generic_consts,
        pretty_debug,
        format_options,
        debug_hex,
        quiet_locals,
        lazy_locals,
        local_makers,
        named_expressions,
        local_functions,
        local_consts,
        held,
    } = match analyze_crate(
        tcx,
        all_bodies,
        initializers,
        dependencies,
        (export_library, std_bodies),
        (instances, made_where_used),
    ) {
        Some(analyzed) => analyzed,
        None => return Lowered::Crate(None),
    };
    // Each module's default export, which a module of the crate imports as
    // its default, not by a name of its own (ADR 0251), as every importer
    // does (`default_exports::exported_as`).
    let defaulted: HashSet<DefId> = (modules.iter())
        .filter_map(|&module| super::default_exports::own_default(tcx, module))
        .collect();
    // A library exports what its program does: what its consumers reach that
    // it doesn't, they compile (ADR 0914, amended). `@rust-js/std`, compiled
    // once into a package, all they may reach (ADR 0561).
    let exports_reachable = std_bodies;
    if exports_reachable {
        called_from_elsewhere.extend(fns.keys().copied().filter(|&id| super::library::reachable(tcx, id)));
    }
    // A derived serde impl's codec is one rust-js writes of its type's
    // attributes, which no consumer sees to write: a library exports each
    // its consumers can reach (ADRs 0077, 0100).
    if export_library {
        called_from_elsewhere.extend(codecs.iter().copied().filter(|&id| super::library::reachable(tcx, id)));
    }
    let foreign = super::library::Foreign::new(tcx, dependencies, instances);
    // Which of its libraries' functions may copy what the crate changes in place, which their
    // JS, copying what their own crates change, may share (ADR 0914, amended).
    let changed = super::copies::Changed {
        mutated: &mutated,
        vecs: &changed_vecs,
    };
    let copying = super::instances::Copying::new((&trait_impls, &foreign), changed);
    let given = super::traits::Given {
        copied: &copied,
        clones: &clones,
        stepped: &stepped,
    };
    // The library functions the crate's calls give more than they take,
    // compiled here first, before anything is lowered (ADR 0914).
    let analyzed = super::instances::Analyzed {
        trait_impls: &trait_impls,
        drops: (export_library, &closure_facts, &counted),
        drop_params: &drop_params,
        given,
        bodies: bodies.iter().copied().chain(closures.values().copied()).collect(),
    };
    match super::instances::needed(&foreign, instances, analyzed) {
        _ if tcx.dcx().has_errors().is_some() => return Lowered::Crate(None),
        more if !more.is_empty() => return Lowered::Instances(more),
        _ => {}
    }
    let mut const_items: HashMap<LocalModId, Vec<js::Const>> = HashMap::new();
    // Each `const` written in a function's body, its function's (ADR 0031,
    // amended).
    let mut local_const_holes: Vec<(DefId, js::Const)> = Vec::new();
    // The statics and constants whose initializers are lowered as code.
    let mut initialized: Vec<&Body<'tcx>> = Vec::new();
    for &def_id in consts.iter().filter(|&&d| !is_thread_local(tcx, d)) {
        let span = tcx.def_span(def_id);
        let typing_env = ty::TypingEnv::fully_monomorphized();
        let args = ty::GenericArgs::identity_for_item(tcx, def_id);
        let initializer = initializers.iter().find(|body| body.def_id == def_id.to_def_id());
        // A `const { .. }` block's type is its value's.
        let ty = match (tcx.def_kind(def_id), initializer) {
            (DefKind::AnonConst, Some(body)) => body.thir[body.expr].ty,
            _ => tcx.type_of(def_id).instantiate_identity().skip_normalization(),
        };
        // Its initializer, lowered as code, where one JS value can be every
        // use's, and it reads no static, whose value may not be made yet.
        // A `OnceLock`, only ever made empty, or a `LazyLock`, which
        // makes its value when it's used, is one JS cell too (ADRs 0317, 0318),
        // and so is a `Mutex`, an `RwLock` or an atomic, which every use
        // changes in place, the one place a static is, and a `static mut`,
        // its value's `{ value }` (ADR 0096).
        let mutable = matches!(tcx.def_kind(def_id), DefKind::Static { mutability, .. } if mutability.is_mut());
        let once = matches!(tcx.def_kind(def_id), DefKind::Static { .. })
            && matches!(ty.kind(), ty::Adt(adt, _)
            if [
                super::recognition::StdItem::OnceCell,
                super::recognition::StdItem::LazyCell,
                super::recognition::StdItem::Mutex,
                super::recognition::StdItem::RwLock,
                super::recognition::StdItem::Atomic,
            ]
                .into_iter()
                .any(|item| super::recognition::is_std_def(tcx, adt.did(), item)));
        // A constant's each use is a value of its own, copied where it may
        // change (`named_const`), so its one JS value needn't be shareable:
        // `const EMPTY: Vec<i32> = Vec::new()`'s `[]` (ADR 0031).
        let constant = !matches!(tcx.def_kind(def_id), DefKind::Static { .. });
        let as_code = initializer.filter(|body| {
            (mutable || constant || super::copies::shareable(tcx, ty) || once)
                && !super::body_queries::reads_statics(tcx, &body.thir, tcx.parent_module_from_def_id(def_id))
        });
        // One written of other constants read by their names, `A | B`, is
        // its initializer's, as it's written, not rustc's value of it, which
        // would lose the names, and what a library's flag says where it's
        // swapped for another (ADR 0031, amended), or of `js::null()`, a call
        // written as the `null` it gives, where rustc's `None` would be left
        // out (ADR 0935, amended).
        let by_name = |id: DefId| fns.contains_key(&id) || foreign.item(id).is_some();
        let in_place = |id: DefId| super::bindings::is_js_null(tcx, id);
        if let Some(body) = as_code
            && super::body_queries::written_of_constants(&body.thir, &by_name, &in_place)
        {
            initialized.push(body);
            continue;
        }
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
            // One written as `Math`'s constant is its, `Math.LN2` (ADR 0887).
            _ => (
                initializer
                    .and_then(|body| super::std_types::number::math::written_constant(tcx, &body.thir, body.expr))
                    .or_else(|| {
                        super::representation::const_value_js(tcx, typing_env, def_id.to_def_id(), args, span)
                            .map(|v| super::literals::const_string(tcx, def_id, v, &|span| sources.span(span)))
                            .map(|v| super::aggregates::const_object(tcx, def_id, v))
                    }),
                "constants",
            ),
        };
        let Some(value) = value else {
            // A value rustc's can't say, a function's, a `dyn`'s or one too
            // large for a value tree, is its initializer's.
            if let Some(body) = as_code {
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
        // A nullable one's `None` `null`, a nullable `const`'s too (ADR 0586, amended).
        let nullable = |id: rustc_span::def_id::LocalDefId| super::bindings::nullable_local(tcx, id.to_def_id());
        let value = match key {
            Some(key) if nullable(key) => super::std_types::cell::nullable_made(value),
            None if nullable(def_id) => js::Storage::NullForNone.written(value, false),
            _ => value,
        };
        let info = &fns[&named.to_def_id()];
        let constant = js::Const {
            name: info.name.clone(),
            value,
            mutable: false,
            export: tcx.visibility(named).is_public() || called_from_elsewhere.contains(&named.to_def_id()),
            span: sources.item_span(tcx, named),
        };
        // One written in a function's body, where it's written there.
        match local_consts.contains_key(&named.to_def_id()) {
            true => local_const_holes.push((named.to_def_id(), constant)),
            false => const_items.entry(info.module).or_default().push(constant),
        }
    }

    let function_bodies = bodies.iter().map(|body| (body.def_id, *body)).collect();
    // A trait impl's dictionary (ADR 0049) has no body of its own, so its
    // function context gets an empty one; a copied default brings its own.
    let no_body = rustc_middle::thir::Thir::new(rustc_middle::thir::BodyTy::Const(tcx.types.unit));
    // Lower each body once. Cross-module references have symbolic names until
    // the link step knows exactly which imports and local names survive.
    let mut lowered_items: Vec<(DefId, super::LoweredFn)> = Vec::new();
    // Each module's JS, where a JS file of the program's own imports it
    // from, and what it imports for its effects (`module_imports`).
    let places: Vec<ModuleJs> = (modules.iter())
        .map(|&module| ModuleJs {
            module,
            path: paths[&module].clone(),
            file: module_file(tcx, module).name.clone().into_local_path(),
            located: located(tcx, module),
            effects: (super::bindings::marks(tcx, module, "import"))
                .filter_map(|attr| Some(attr.value_str()?.to_string()))
                .collect(),
        })
        .collect();
    // A `Vec`'s capacity is kept where the crate or a library it uses asks
    // it, as what it gives that library may be asked (ADR 0419).
    let keeps_capacity = asks_capacity || foreign.keeps_capacity();
    let takes_used_drops = super::drops::TakesUsed::new(tcx, &foreign, &drop_params);
    // What each function's lowering found of drops, gathered in the order
    // they're lowered, lowered or not, then resolved (ADRs 0190, 0300).
    let mut drops_observed = super::drops::DropsObserved::default();
    let ref_cells = std::cell::OnceCell::new();
    // What runs where its module loads, which reads another module's
    // `const`s as their values where that module may load after its own
    // (`load_cycles`).
    // A thread-local made where it's first used runs nothing as its module
    // loads (ADR 0037, amended).
    let loads: HashSet<DefId> = (thread_local_inits.iter())
        .filter(|(_, key)| !lazy_locals.contains(key))
        .map(|(init, _)| init.to_def_id())
        .chain(
            (initialized.iter())
                .map(|body| body.def_id)
                .filter(|id| !local_consts.contains_key(id)),
        )
        .chain(
            (bodies.iter())
                .map(|body| body.def_id)
                .filter(|&id| super::bindings::is_on_load(tcx, id)),
        )
        .collect();
    let crate_facts = CrateFacts {
        foreign: &foreign,
        library: export_library,
        compiled_once: exports_reachable,
        asked: Default::default(),
        copying: &copying,
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
        given,
        type_facts: &type_facts,
        failing: &failing,
        throwing: &throwing,
        never_unwinding: &never_unwinding,
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
        local_consts: &local_consts,
        loads_in_cycles: Default::default(),
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
        // A trait's default is copied into each impl (ADR 0049); a library's,
        // which a consumer's impl of its trait calls, a function over `Self`,
        // the consumer's own (ADRs 0185, 0914, amended); std's, compiled
        // into `@rust-js/std`, its own too (ADR 0561).
        .filter(|b| {
            let id = b.def_id;
            tcx.trait_of_assoc(id).is_none()
                || foreign.is_instance(id)
                || (exports_reachable && super::library::reachable(tcx, id))
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
            if !round.is_empty() {
                work.extend(round);
            } else {
                // Each body run where its module loads that read another
                // module's `const` by its name, where that module may load
                // after its own: lowered again, reading its values, once
                // (ADR 0031, amended).
                if !crate_facts.loads_in_cycles.borrow().is_empty() {
                    break;
                }
                let lowered = || (lowered_items.iter()).map(|(_, item)| &item.dependencies);
                let edges = (
                    lowered().flat_map(|d| &d.references),
                    lowered().flat_map(|d| &d.held),
                    lowered().flat_map(|d| &d.package_uses),
                );
                let imports = module_imports(&places, &fns, edges, &imported, js_modules);
                // A thread-local made as its module loads, of a function of a
                // module that may load after it: made where it's first used,
                // the crate lowered again so (ADR 0037, amended).
                let cyclic = load_cycles::made_in_cycles(tcx, &local_makers, &fns, &imports);
                if !cyclic.is_empty() {
                    return Lowered::MadeWhereUsed(cyclic);
                }
                let again = load_cycles::load_cycles(tcx, &lowered_items, &fns, &loads, &imports);
                if again.is_empty() {
                    break;
                }
                let bodies: Vec<_> = (again.iter())
                    .filter_map(|&id| work.iter().find(|(at, _)| *at == id).copied())
                    .collect();
                crate_facts.loads_in_cycles.borrow_mut().extend(again);
                work.extend(bodies);
            }
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
                // One lowered again, its values read: in its place.
                match lowered_items.iter_mut().find(|(id, _)| *id == def_id) {
                    Some(earlier) => earlier.1 = lowered,
                    None => lowered_items.push((def_id, lowered)),
                }
            }
            Err(_) => failed = true,
        }
    }
    // What of its libraries the crate's lowering found it compiles itself,
    // which their JS doesn't export: the crate lowered again with them, as
    // with what its calls give more (ADR 0914, amended).
    let asked = crate_facts.asked.take();
    if !asked.is_empty() && tcx.dcx().has_errors().is_none() {
        return Lowered::Instances(super::instances::asked(tcx, asked));
    }
    // What a function takes no destructor of, each call of it checked (ADR 0190).
    let checks = std::mem::take(&mut drops_observed.checks);
    failed |= super::drops::check_no_drops(tcx, &mut drops_observed.no_drops, &checks);
    if failed {
        return Lowered::Crate(None);
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
    // Adjacency lists rather than scanning every edge again for every reached function.
    let recognition = super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    // A `PartialEq` whose `==` is `===`, whatever its `Hash`; and of a
    // type equal only to itself, whose `Hash` is its address's too.
    let identity_eq = |id: DefId, trait_id: DefId, keyed: bool| {
        let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
        tcx.is_lang_item(trait_id, rustc_hir::attrs::lang_items::LangItem::PartialEq)
            && match keyed {
                true => super::std_impls::equal_only_to_itself(&recognition, &function_bodies, ty),
                false => super::std_impls::eq_by_address(&recognition, &function_bodies, ty),
            }
    };
    // Its types equal only to themselves, which its contract lists for its
    // consumers (ADR 0864, amended).
    let identities: Vec<DefId> = (trait_impls.iter())
        .filter(|&&id| {
            identity_eq(
                id,
                tcx.impl_trait_ref(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id,
                true,
            )
        })
        .filter_map(|&id| tcx.type_of(id).instantiate_identity().skip_normalization().ty_adt_def())
        .map(|adt| adt.did())
        .collect();
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
                || identity_eq(id, trait_id, false)
                // A library's the crate compiles itself, only what it uses
                // of it (ADR 0914, amended).
                || foreign.is_instance(id)
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
    let own_roots = (edges.clone())
        .filter(|(from, _)| !derived.contains(from))
        .map(|(_, to)| to);
    // What the crate's own code reaches: a dictionary its consumers alone
    // read is read once every module of the crate has loaded, so no load
    // cycle of the crate's reaches it (ADR 0592, amended).
    let reached_here = crate::reachability::reachable(own_roots.clone(), edges.clone());
    let roots = own_roots
        // `@rust-js/std`'s consumers may reach one (ADR 0561); a library's
        // compile their own (ADR 0914, amended).
        .chain(
            derived
                .iter()
                .copied()
                .filter(|&id| exports_reachable && super::library::reachable(tcx, id)),
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
            if reached_here.contains(&def_id) {
                pass.dictionary_consts.push((module, def_id));
            }
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
                // A nullable thread-local's `None` `null` (ADR 0586, amended).
                let value = match super::bindings::nullable_local(tcx, key.to_def_id()) {
                    true => super::std_types::cell::nullable_made(value),
                    false => value,
                };
                // A `static mut`'s is its `{ value }` (ADR 0096).
                let value = match tcx.static_mutability(key.to_def_id()) {
                    Some(mutability) if mutability.is_mut() => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),
                    _ => value,
                };
                let info = &fns[&key.to_def_id()];
                let constant = js::Const {
                    name: info.name.clone(),
                    value,
                    mutable: false,
                    // A `const { .. }` block has no visibility: it's its function's alone.
                    export: (tcx.def_kind(key) != DefKind::AnonConst && tcx.visibility(key).is_public())
                        || called_from_elsewhere.contains(&key.to_def_id()),
                    span: sources.item_span(tcx, key),
                };
                match local_consts.contains_key(&key.to_def_id()) {
                    true => local_const_holes.push((key.to_def_id(), constant)),
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
                lowered.function.export |= called_from_elsewhere.contains(&def_id);
                match &fns[&def_id].owner {
                    // Its type's object is exported if any of its methods is.
                    Some(owner) => {
                        // A class's, its constructor or its prototype's
                        // (ADR 0761).
                        let member = super::classes::member(tcx, def_id);
                        if let Err(what) = super::classes::as_member(tcx, def_id, owner, &mut lowered.function) {
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
                                getters: Vec::new(),
                                declared: (super::classes::class_of(tcx, def_id).and_then(|adt| adt.as_local()))
                                    .map_or(js::Span::NONE, |adt| sources.item_span(tcx, adt)),
                            });
                            super::classes::place(member, class, &lowered.function.name);
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
                        pass.holes.functions.insert(def_id.index.as_u32(), function);
                    }
                    None if named_expressions.contains(&def_id) => {
                        let mut function = lowered.function;
                        let mut read = HashSet::new();
                        js::visit_stmts(&function.body, &mut |name| {
                            read.insert(name.to_owned());
                        });
                        function.name = super::fresh_in(&mut read, &function.name);
                        function.export = false;
                        pass.holes.functions.insert(def_id.index.as_u32(), function);
                    }
                    None => pass.functions.entry(module).or_default().push(lowered.function),
                }
            }
        }
    }
    if failed {
        return Lowered::Crate(None);
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
            // A class's member, by its class, which may be its module's
            // default (ADR 0761).
            Some(_) if let Some(class) = super::classes::class_of(tcx, id) => (
                super::default_exports::exported_as(tcx, class, &name),
                renamed_in(tcx, from).remove(&class).unwrap_or_else(|| name.clone()),
            ),
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
    let edges = (pass.references.iter(), pass.held.iter(), pass.package_uses.iter());
    let imports = module_imports(&places, &fns, edges, &imported, js_modules);
    load_cycles::refuse_dictionaries_in_load_cycles(tcx, &pass, &const_items, (&modules, &imports));
    // The exported thread-locals and `static mut`s their modules made
    // their variables.
    let mut variables: HashSet<DefId> = HashSet::new();
    let mut lowered: Vec<UnlinkedModule> = modules
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
                        default_from = Some((function, span));
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
            let reexports = super::reexports::reexports(
                tcx,
                module,
                (&fns, &foreign),
                &paths,
                (default_from, module_held),
                &sources,
            );
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
                // A class, its constructor and prototype (ADR 0761).
                for class in (pass.namespaces.entry(module).or_default().iter_mut()).filter(|n| n.name == name) {
                    class.export = false;
                }
            }
            let no_ref_cells = super::drops::RefCells::default();
            let ref_cells = ref_cells.get().unwrap_or(&no_ref_cells);
            let aliases: Vec<(String, String)> = aliased_items.iter().map(|(_, names)| names.clone()).collect();
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
                declarations: None,
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
            // Its `static mut`s, each its module's variable as a
            // thread-local of a value is (ADR 0096, amended).
            let statics = super::analysis::module_statics(tcx, &consts, module);
            let quiet: HashSet<&str> = (quiet_locals.iter())
                .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
                .chain(&statics)
                .map(|key| fns[&key.to_def_id()].name.as_str())
                .collect();
            // Its thread-locals of a value, and its `static mut`s, each its
            // module's variable where it's plain, by its name.
            let held_values: HashMap<&str, rustc_span::def_id::LocalDefId> = (thread_local_inits.values())
                .filter(|&&key| {
                    tcx.parent_module_from_def_id(key) == module && super::analysis::holds_a_value(tcx, key.to_def_id())
                })
                .chain(&statics)
                .filter_map(|&key| Some((fns.get(&key.to_def_id())?.name.as_str(), key)))
                .collect();
            // Those it exports as its variables, `export let x`, which
            // their importers read by name (ADR 0270, amended); one it
            // exports still its `{ value }`, which a JS module importing
            // it would read as the box, refused.
            let (made, boxed) = super::std_types::cell::plain_statics(
                &mut lowered.items,
                &quiet,
                &held_values.keys().copied().collect(),
            );
            for name in boxed {
                super::analysis::refuse_boxed_export(tcx, held_values[name.as_str()].to_def_id());
            }
            let module_variables: HashMap<DefId, bool> = (quiet_locals.iter())
                .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
                .chain(&statics)
                .filter_map(|key| {
                    let name = &fns[&key.to_def_id()].name;
                    let &(_, set) = made.iter().find(|(made, _)| made == name)?;
                    Some((key.to_def_id(), set))
                })
                .collect();
            variables.extend(module_variables.keys().copied());
            // A library's, by the name the module imports it as.
            let names = import_names.get(&module);
            let foreign_variables: HashSet<&str> = (foreign.all())
                .filter(|item| item.calling.has(crate::library::ItemFact::Variable) && item.member.is_none())
                .filter_map(|item| names?.get(&(item.from.clone(), item.export.clone())))
                .map(String::as_str)
                .collect();
            if !foreign_variables.is_empty() {
                super::std_types::cell::imported_variables(
                    &mut lowered.items,
                    &|e| matches!(&e.kind, js::ExprKind::Var(name) if foreign_variables.contains(name.as_str())),
                );
            }
            lowered.declarations = super::declarations::module(
                tcx,
                &foreign,
                module,
                (default_export, default_declared),
                &paths,
                &lowered.reexports,
                &aliased_items,
                (ref_cells, &counted),
                (&fns, module_held, &module_variables),
            );
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
    // What each module reads of another's variables, by their names (ADR
    // 0270, amended).
    let symbols: HashSet<js::Symbol> = (variables.iter())
        .map(|id| {
            let target = &fns[id];
            module_symbol(target.module, target.owner.as_ref().unwrap_or(&target.name))
        })
        .collect();
    if !symbols.is_empty() {
        for unlinked in &mut lowered {
            super::std_types::cell::imported_variables(
                &mut unlinked.module.items,
                &|e| matches!(&e.kind, js::ExprKind::Symbol(symbol) if symbols.contains(symbol)),
            );
        }
    }
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
    let exported = super::library::exported_names(&lowered);
    Lowered::Crate(tcx.dcx().has_errors().is_none().then(|| {
        Box::new(Unlinked {
            library: export_library.then(|| {
                super::library::exports(
                    tcx,
                    (&fns, &exported),
                    &super::library::Contracts {
                        drop_params: &drop_params,
                        given,
                        failing: &failing,
                        no_drops: &no_drops,
                        type_facts: &type_facts,
                        caller_locations: &caller_locations,
                        keeps_capacity,
                        writers_take_options,
                        makes_dropping_anys: closure_facts.any_dyns_drop,
                        makes_dropping_fn_dyns: closure_facts.makes_dropping_fn_dyns,
                        counted: &counted,
                        ref_cells: ref_cell_facts(),
                        fields: ref_cells.get().map(|found| found.fields(tcx)).unwrap_or_default(),
                        lazy_locals: &lazy_locals,
                        variables: &variables,
                        identities: &identities,
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

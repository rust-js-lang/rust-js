//! Orchestrate analyzed crate facts, function emission, reachability and symbolic module assembly.

use super::analysis::{AnalyzedCrate, analyze_crate, is_thread_local, renamed_in};
use super::bindings::Export;
use super::{Body, CrateFacts, FnCx, FnInfo, Locals, const_js, eval_const, module_file, module_symbol, static_value};
use crate::js::{self, Expr, Prop, StmtKind};
use crate::program::{ImportRequest, LoweredImport, LoweredModule, Unlinked, UnlinkedModule};
use crate::runtime::Helper;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::find_attr;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalModId};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Retained functions and their dependencies, grouped for output.
#[derive(Default)]
struct Pass {
    functions: HashMap<LocalModId, Vec<js::Function>>,
    namespaces: HashMap<LocalModId, Vec<js::Namespace>>,
    runtime: HashMap<LocalModId, HashSet<Helper>>,
    jsx: HashSet<LocalModId>,
    caches: HashMap<LocalModId, Vec<String>>,
    /// What each module runs when it's loaded, each `js::on_load!`'s
    /// (ADR 0267).
    statements: HashMap<LocalModId, Vec<Vec<js::Stmt>>>,
    /// `thread_local!`s' values, made from their lowered `init`s.
    local_consts: HashMap<LocalModId, Vec<js::Const>>,
    /// The functions a block makes and gives, by their items' indices, for
    /// their holes (ADR 0296).
    holes: HashMap<u32, js::Function>,
    /// Which items each module uses from another, and which JS imports.
    references: HashSet<(LocalModId, DefId)>,
    package_uses: HashSet<(LocalModId, Export)>,
}

/// Lower every function, grouped by module. Reports all unsupported
/// features as rustc errors.
pub fn lower_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[Body<'tcx>],
    initializers: &[Body<'tcx>],
    serde_attrs: &super::SerdeAttributes,
    dependencies: &crate::library::Dependencies,
    export_library: bool,
) -> Option<Unlinked> {
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
        codecs,
        modules,
        taken,
        fns,
        mut failed,
        mut called_from_elsewhere,
        tests,
        paths,
        mutated,
        changed_vecs,
        mut drop_params,
        copied,
        type_facts,
        failing,
        generic_consts,
        pretty_debug,
        format_options,
        plain_locals,
        plain_cells,
        read_at_once,
        named_expressions,
    } = analyze_crate(tcx, all_bodies, initializers, dependencies, export_library)?;
    // Each module's default export, which a module of the crate imports as
    // its default, not by a name of its own (ADR 0251).
    let defaults: HashMap<LocalModId, DefId> = (modules.iter())
        .filter_map(|&module| {
            super::bindings::default_exports(tcx, module)
                .into_iter()
                .filter_map(|(item, _)| item)
                .find(|item| {
                    item.as_local()
                        .is_some_and(|local| tcx.parent_module_from_def_id(local) == module)
                })
                .map(|item| (module, item))
        })
        .collect();
    let defaulted: HashSet<DefId> = defaults.values().copied().collect();
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
                    .map(|v| match mutability.is_mut() {
                        true => Expr::object(vec![Prop::Field("value".into(), v)]),
                        false => v,
                    }),
                "statics",
            ),
            _ => (
                eval_const(tcx, typing_env, def_id.to_def_id(), args, span).and_then(|v| const_js(tcx, v)),
                "constants",
            ),
        };
        let Some(value) = value else {
            let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
            // A value rustc's can't say, a function's, a `dyn`'s or one too
            // large for a value tree, is its initializer's, where one JS value
            // can be every use's, and it reads no static, whose value may not
            // be made yet.
            let mutable = matches!(tcx.def_kind(def_id), DefKind::Static { mutability, .. } if mutability.is_mut());
            if !mutable
                && super::copies::shareable(tcx, ty)
                && let Some(body) = initializers.iter().find(|body| body.def_id == def_id)
                && !super::body_queries::reads_statics(tcx, &body.thir, tcx.parent_module_from_def_id(def_id))
            {
                initialized.push(body);
                continue;
            }
            tcx.dcx()
                .span_err(span, format!("rust-js does not support {what} of type `{ty}` yet"));
            failed = true;
            continue;
        };
        // A `const { .. }` thread-local's value, the `const` in its block, is
        // its variable, by its name.
        let key = thread_local_inits.get(&def_id).copied();
        let (named, value, mutable) = match key {
            Some(key) => {
                let plain = plain_locals.get(&key).copied();
                (key, module_variable(value, plain), plain == Some(true))
            }
            None => (def_id, value, false),
        };
        let info = &fns[&named.to_def_id()];
        let span = tcx.def_span(named).source_callsite();
        const_items.entry(info.module).or_default().push(js::Const {
            name: info.name.clone(),
            value,
            mutable,
            export: tcx.visibility(named).is_public() || called_from_elsewhere.contains(&named.to_def_id()),
            span: sources.span(span),
        });
    }

    let function_bodies = bodies.iter().map(|body| (body.def_id.to_def_id(), *body)).collect();
    // A trait impl's dictionary (ADR 0049) has no body of its own, so its
    // function context gets an empty one; a copied default brings its own.
    let no_body = rustc_middle::thir::Thir::new(rustc_middle::thir::BodyTy::Const(tcx.types.unit));
    // Lower each body once. Cross-module references have symbolic names until
    // the link step knows exactly which imports and local names survive.
    let mut lowered_items: Vec<(DefId, super::LoweredFn)> = Vec::new();
    let foreign = super::library::Foreign::new(tcx, dependencies);
    let no_drops = RefCell::new(HashMap::new());
    let drop_checks = RefCell::new(Vec::new());
    let drop_uses = RefCell::new(super::drops::DropUses::new(tcx, &drop_params));
    let called_bodies = rustc_arena::TypedArena::default();
    let crate_facts = CrateFacts {
        called_bodies: &called_bodies,
        foreign: &foreign,
        library: export_library,
        sources: &sources,
        mutated: &mutated,
        generic_consts: &generic_consts,
        changed_vecs: &changed_vecs,
        drop_params: &drop_params,
        copied: &copied,
        type_facts: &type_facts,
        failing: &failing,
        any_failing: !failing.is_empty() || foreign.any_fails(),
        no_drops: &no_drops,
        drop_checks: &drop_checks,
        drop_uses: &drop_uses,
        closures: &closures,
        bodies: &function_bodies,
        fns: &fns,
        imports: &import_names,
        required: &required,
        trait_impls: &trait_impls,
        serde_attrs,
        pretty_debug,
        format_options,
        plain_locals: &plain_locals,
        plain_cells: &plain_cells,
        read_at_once: &read_at_once,
        named_expressions: &named_expressions,
    };
    let mut work: Vec<(DefId, Option<&Body<'tcx>>)> = bodies
        .iter()
        // A trait's default is copied into each impl (ADR 0049); a library's
        // is a function of its own too, for its consumers' impls (ADR 0185).
        .filter(|b| {
            let id = b.def_id.to_def_id();
            tcx.trait_of_assoc(id).is_none() || (export_library && super::library::reachable(tcx, id))
        })
        .map(|b| (b.def_id.to_def_id(), Some(*b)))
        .chain(dictionaries.iter().map(|id| (*id, None)))
        .chain(initialized.iter().map(|body| (body.def_id.to_def_id(), Some(*body))))
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
            captures: HashMap::new(),
            thir: body.map_or(&no_body, |body| &body.thir),
            body_facts: body.map_or(&no_facts, |body| &body.facts),
            module,
            locals: Locals::default(),
            names,
            module_names: &taken[&module],
            labels: HashSet::new(),
            loops: Vec::new(),
            runtime: HashSet::new(),
            jsx: false,
            writing: Default::default(),
            chains: Default::default(),
            stepping: body.map_or_else(Default::default, |body| {
                super::iterators::Stepping::of(&body.facts.stepped)
            }),
            cloning: Vec::new(),
            item: def_id,
            walks: Default::default(),
            drop_state: Default::default(),
            body_owner: def_id,
        };
        let initializer = initialized.iter().any(|body| body.def_id.to_def_id() == def_id);
        let result = match body {
            Some(body) if initializer => cx.lower_initializer(body),
            Some(body) => cx.lower_fn(body),
            None if codecs.contains(&def_id) => cx.lower_codec(def_id).map(|function| super::LoweredFn {
                function,
                runtime: std::mem::take(&mut cx.runtime),
                jsx: cx.jsx,
                dependencies: cx.dependencies.take(),
            }),
            None => {
                let cache = format!("${}", fns[&def_id].name);
                cx.lower_dictionary(def_id, &cache).map(|function| super::LoweredFn {
                    function,
                    runtime: std::mem::take(&mut cx.runtime),
                    jsx: cx.jsx,
                    dependencies: cx.dependencies.take(),
                })
            }
        };
        match result {
            Ok(lowered) => {
                used.extend(lowered.dependencies.uses.iter().map(|&(_, to)| to));
                lowered_items.push((def_id, lowered));
            }
            Err(_) => failed = true,
        }
    }
    // What a function takes no destructor of, each call of it checked (ADR 0190).
    failed |= super::drops::check_no_drops(tcx, &no_drops, &drop_checks.borrow());
    if failed {
        return None;
    }
    // Only the drops each function uses (ADR 0300).
    drop_uses.into_inner().keep(&mut drop_params, &mut lowered_items);

    // Reachability for derived Debug implementations, with adjacency lists
    // rather than scanning every edge again for every reached function.
    let derived: HashSet<DefId> = trait_impls
        .iter()
        .filter(|&&id| super::recognition::known_derive(tcx, id))
        .flat_map(|&id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied()))
        .collect();
    let edges = lowered_items
        .iter()
        .flat_map(|(_, item)| item.dependencies.uses.iter().copied());
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
        let initializer = initialized.iter().any(|body| body.def_id.to_def_id() == def_id);
        pass.references.extend(lowered.dependencies.references.drain());
        pass.package_uses.extend(lowered.dependencies.package_uses.drain());
        if dictionaries.contains(&def_id) {
            pass.caches
                .entry(module)
                .or_default()
                .push(format!("${}", fns[&def_id].name));
        }
        match lowered {
            lowered
                if let Some(key) = thread_local_inits
                    .get(&def_id.expect_local())
                    .copied()
                    .or_else(|| initializer.then(|| def_id.expect_local())) =>
            {
                // `const COUNT = { value: 0 };`: made when the module loads.
                let function = lowered.function;
                let value = match function.body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(value)),
                            ..
                        },
                    ] => value.clone(),
                    _ => Expr::call(Expr::arrow(Vec::new(), function.body), Vec::new()),
                };
                let info = &fns[&key.to_def_id()];
                let span = tcx.def_span(key).source_callsite();
                let plain = plain_locals.get(&key).copied();
                let value = module_variable(value, plain);
                pass.local_consts.entry(info.module).or_default().push(js::Const {
                    name: info.name.clone(),
                    value,
                    mutable: plain == Some(true),
                    export: tcx.visibility(key).is_public() || called_from_elsewhere.contains(&key.to_def_id()),
                    span: sources.span(span),
                });
                pass.runtime.entry(module).or_default().extend(lowered.runtime);
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
                        let module_namespaces = pass.namespaces.entry(module).or_default();
                        let export = lowered.function.export;
                        match module_namespaces.iter_mut().find(|n| n.name == *owner) {
                            Some(namespace) => {
                                namespace.export |= export;
                                namespace.methods.push(lowered.function);
                            }
                            None => module_namespaces.push(js::Namespace {
                                name: owner.clone(),
                                methods: vec![lowered.function],
                                export,
                            }),
                        }
                    }
                    None if super::bindings::is_on_load(tcx, def_id) => {
                        pass.statements.entry(module).or_default().push(lowered.function.body)
                    }
                    // Named in itself alone, as the Rust names it, unless
                    // what it reads is named so.
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
                pass.runtime.entry(module).or_default().extend(lowered.runtime);
            }
        }
    }
    for (module, consts) in pass.local_consts.drain() {
        const_items.entry(module).or_default().extend(consts);
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
    // What each module imports of another: its item's name, and the
    // module's default's, by what the importer's `use` names it.
    let mut targets: HashMap<LocalModId, HashSet<(LocalModId, String, Option<String>)>> = HashMap::new();
    for &(from, id) in &pass.references {
        let info = &fns[&id];
        let name = info.owner.as_ref().unwrap_or(&info.name).clone();
        let default = defaulted
            .contains(&id)
            .then(|| renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone()));
        targets.entry(from).or_default().insert((info.module, name, default));
    }
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
            for attr in super::bindings::marks(tcx, module, "import") {
                let Some(from) = attr.value_str().map(|s| s.to_string()) else {
                    tcx.dcx()
                        .span_err(attr.span(), "rust-js: write it `js::import!(\"./file.css\");`");
                    continue;
                };
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
            for attr in super::bindings::marks(tcx, module, "directive") {
                match attr.value_str() {
                    Some(directive) => directives.push(directive.to_string()),
                    None => {
                        tcx.dcx()
                            .span_err(attr.span(), "rust-js: write it `js::directive!(\"use client\");`");
                    }
                }
            }
            let mut default_export = None;
            // Another module's function, re-exported as the default (ADR 0240).
            let mut default_from = None;
            for (function, span) in super::bindings::default_exports(tcx, module) {
                let unset = default_export.is_none() && default_from.is_none();
                match function {
                    Some(function)
                        if unset
                            && function
                                .as_local()
                                .is_some_and(|local| tcx.parent_module_from_def_id(local) == module) =>
                    {
                        default_export = Some(function);
                    }
                    Some(function) if unset && fns.get(&function).is_some_and(|info| info.owner.is_none()) => {
                        default_from = Some(function);
                    }
                    _ => {
                        tcx.dcx().span_err(
                            span,
                            "rust-js: `js::export_default!` names one function or thread-local of its own module, once",
                        );
                    }
                }
            }
            let reexports = reexports(tcx, module, &fns, &paths, default_from);
            let declarations = super::declarations::module(tcx, module, default_export, &paths, &reexports);
            let lowered = LoweredModule {
                path: paths[&module].clone(),
                file: module_file(tcx, module).name.clone().into_local_path(),
                directives,
                packages,
                imports: Vec::new(),
                reexports,
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
                caches: pass.caches.remove(&module).unwrap_or_default(),
                default_export: default_export.map(|function| super::bindings::fn_name(tcx, function)),
                declarations,
                runtime: Vec::new(),
                jsx: pass.jsx.contains(&module),
                located: find_attr!(tcx, module.to_def_id(), Path(..)),
            };
            let mut imports: Vec<_> = targets.remove(&module).unwrap_or_default().into_iter().collect();
            imports.sort_by(|(a, an, _), (b, bn, _)| (&paths[a], an).cmp(&(&paths[b], bn)));
            let candidates: Vec<_> = imports
                .into_iter()
                .map(|(target, name, default)| ImportRequest {
                    symbol: module_symbol(target, &name),
                    export: if default.is_some() {
                        "default".to_string()
                    } else {
                        name.clone()
                    },
                    local: default.unwrap_or(name),
                    path: paths[&target].clone(),
                })
                .collect();
            UnlinkedModule {
                module: lowered,
                imports: candidates,
                reserved_names: taken[&module].clone(),
                runtime: pass.runtime.remove(&module).unwrap_or_default(),
            }
        })
        .collect();
    tcx.dcx().has_errors().is_none().then_some(Unlinked {
        library: export_library.then(|| {
            super::library::exports(
                tcx,
                &fns,
                &drop_params,
                &failing,
                &no_drops.borrow(),
                &trait_impls,
                dependencies,
            )
        }),
        sources: sources.output,
        modules: lowered,
        tests,
    })
}

/// What `module` re-exports, its public `pub use` of another module's
/// function, by that module's path: each `(export, alias)`, the function's
/// JS name and the name the `use` gives it (ADR 0240); and `default`,
/// another module's function its `js::export_default!` names.
fn reexports(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    fns: &HashMap<DefId, FnInfo>,
    paths: &HashMap<LocalModId, Vec<String>>,
    default_from: Option<DefId>,
) -> Vec<LoweredImport> {
    let mut grouped: BTreeMap<Vec<String>, Vec<(String, String)>> = BTreeMap::new();
    if let Some(info) = default_from.and_then(|function| fns.get(&function))
        && let Some(from) = paths.get(&info.module)
    {
        grouped
            .entry(from.clone())
            .or_default()
            .push((info.name.clone(), "default".to_string()));
    }
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Use(path, hir::UseKind::Single(ident)) = item.kind else {
            continue;
        };
        let Some(Res::Def(_, def_id)) = path.res.value_ns else {
            continue;
        };
        let Some(info) = fns.get(&def_id) else { continue };
        if !tcx.visibility(item.owner_id).is_public() || info.module == module || info.owner.is_some() {
            continue;
        }
        let Some(from) = paths.get(&info.module) else { continue };
        let alias = match ident.name == tcx.item_name(def_id) {
            true => info.name.clone(),
            false => ident.to_string(),
        };
        grouped
            .entry(from.clone())
            .or_default()
            .push((info.name.clone(), alias));
    }
    grouped
        .into_iter()
        .map(|(path, named)| LoweredImport { path, named })
        .collect()
}

/// Each hole in `stmts` filled with its function, and each in that (ADR 0296).
fn fill_holes(stmts: &mut [js::Stmt], holes: &mut HashMap<u32, js::Function>) {
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

/// A thread-local's value, `{ value: 0 }`: of one only read and set, what
/// its cell holds, its module's variable, a `let` if it's set (ADR 0270).
fn module_variable(value: Expr, plain: Option<bool>) -> Expr {
    match value.kind {
        js::ExprKind::Object(ref props)
            if plain.is_some()
                && let [js::Prop::Field(field, held)] = props.as_slice()
                && field == "value" =>
        {
            held.clone()
        }
        _ => value,
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

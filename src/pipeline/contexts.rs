//! The context a body is lowered in, made one way wherever it's made: each
//! of the crate's bodies, and, for a library none of whose bodies asked
//! what its `RefCell`s are, the one its contract's facts are found in.

use rust_js_lower::boundary::split::Split;
use rust_js_lower::{Body, BodyFrame, CrateFacts, FnCx, Locals, Settled, body_queries::BodyFacts};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::thir::Thir;
use rustc_middle::ty;
use rustc_span::def_id::{DefId, LocalModId};

/// What every body's context is made of: the crate's facts, and the frame
/// of a body that has no THIR of its own, a dictionary's or a codec's.
pub(super) struct Shared<'a, 'tcx> {
    pub(super) tcx: ty::TyCtxt<'tcx>,
    pub(super) krate: &'a CrateFacts<'a, 'tcx>,
    pub(super) no_body: &'a Thir<'tcx>,
    pub(super) no_facts: &'a BodyFacts<'tcx>,
}

/// The parameters each of the crate's calls fills with its own callback
/// (`FnCx::find_own_callbacks`, rd616), found of `krate`'s facts before any
/// body, in a context of the crate's first function, as a fact of the crate
/// (ADR 1148): none where it has none.
pub(super) fn own_callbacks<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    krate: &CrateFacts<'_, 'tcx>,
    no_body: &Thir<'tcx>,
) -> IndexSet<rustc_hir::HirId> {
    use rustc_hir::def::DefKind;
    let Some(function) = (tcx.hir_crate_items(()).definitions()).find(|&id| tcx.def_kind(id) == DefKind::Fn) else {
        return IndexSet::default();
    };
    let no_facts = BodyFacts::default();
    let shared = Shared {
        tcx,
        krate,
        no_body,
        no_facts: &no_facts,
    };
    let (settled, names): (Settled, IndexSet<String>) = Default::default();
    let module = tcx.parent_module_from_def_id(function);
    item_cx((&shared, &settled), function.to_def_id(), (module, &names)).find_own_callbacks()
}

/// The context of an item, `def_id`, as no body of it: what its signature
/// is, or a library's contract's facts, with nothing named in it yet.
pub(super) fn item_cx<'a, 'tcx>(
    (shared, settled): (&Shared<'a, 'tcx>, &'a Settled<'tcx>),
    def_id: DefId,
    given: (LocalModId, &'a IndexSet<String>),
) -> FnCx<'a, 'tcx> {
    fn_cx(
        (shared, settled),
        (def_id, None, None),
        given,
        IndexSet::default(),
        IndexMap::default(),
    )
}

/// The context `def_id`'s `body`, of `module`, or a `copy` of it its callers
/// need, is lowered in: `names` its locals may not take, `module_names`
/// its module's, and its scope struct's methods (ADR 1043).
pub(super) fn fn_cx<'a, 'tcx>(
    (shared, settled): (&Shared<'a, 'tcx>, &'a Settled<'tcx>),
    (def_id, body, copy): (DefId, Option<&'a Body<'tcx>>, Option<usize>),
    (module, module_names): (LocalModId, &'a IndexSet<String>),
    names: IndexSet<String>,
    scope_functions: IndexMap<DefId, rust_js_program::js::Name>,
) -> FnCx<'a, 'tcx> {
    let tcx = shared.tcx;
    FnCx {
        tcx,
        typing_env: ty::TypingEnv::post_analysis(tcx, def_id),
        given: Default::default(),
        codecs: Vec::new(),
        krate: shared.krate,
        settled,
        dependencies: Default::default(),
        drops_observed: Default::default(),
        asked_of_libraries: Default::default(),
        module_fns_made: Default::default(),
        nulled_seen: Default::default(),
        frame: BodyFrame {
            thir: body.map_or(shared.no_body, |body| &body.thir),
            facts: body.map_or(shared.no_facts, |body| &body.facts),
            owner: def_id,
        },
        module,
        locals: Locals::default(),
        names: std::cell::RefCell::new(names),
        first_of: Default::default(),
        module_names,
        jsx: false,
        writing: Default::default(),
        recursive: Default::default(),
        item: def_id,
        copy,
        walks: Default::default(),
        drop_state: Default::default(),
        scope_functions,
        scope_copies: IndexMap::default(),
        scope_self: None,
        param_origins: Vec::new(),
        scope_values: Vec::new(),
        telling_arms: Default::default(),
        made_into: None,
        made_from: Vec::new(),
        asking_made: std::cell::Cell::new(false),
        seen_through: std::cell::Cell::new(None),
    }
}

/// The crate's split functions (#2403 step 2b(a), ADR 1193), found where
/// no body asked first, in a context of the crate's first function, as
/// `own_callbacks` are; each written in two in `functions`, its JS entry
/// and its body, which another module's call of, `references`, exports.
pub(super) fn split_written<'s, 'tcx>(
    shared: &Shared<'_, 'tcx>,
    (found, bodies): (&'s rust_js_lower::crate_facts::Lazy<Split>, &[&Body<'tcx>]),
    (fns, references): (&IndexMap<DefId, rust_js_lower::FnInfo>, &IndexSet<(LocalModId, DefId)>),
    functions: rust_js_lower::boundary::split::Written<'_>,
) -> &'s Split {
    if let Some(local) = bodies.iter().find_map(|body| body.def_id.as_local()) {
        let (settled, names): (Settled, IndexSet<String>) = Default::default();
        let module = shared.tcx.parent_module_from_def_id(local);
        item_cx((shared, &settled), local.to_def_id(), (module, &names)).split();
    }
    let split = found.get_or_init_default();
    rust_js_lower::boundary::split::written((split, shared.krate.library), fns, references, functions);
    split
}

/// Each library function a module JS may call re-exports, whose own JS
/// checks nothing JS gives it (`Item::checked`): the module's own checked
/// forwarder in its re-export's place, the re-export dropped, what the
/// forwarder imports and checks with into `pass` (#2403 (Y), the lead's
/// decision).
pub(super) fn library_forwarders<'tcx, D>(
    shared: &Shared<'_, 'tcx>,
    js_reached: &rust_js_lower::reexports::JsReached,
    reexported: &mut IndexMap<LocalModId, (D, Vec<rust_js_lower::reexports::Named>)>,
    pass: &mut super::Pass,
) {
    for (module, def_id, alias) in &js_reached.library {
        // A function's, not a constant's or a static's, which JS reads, never calls.
        if shared.tcx.def_kind(*def_id) != rustc_hir::def::DefKind::Fn {
            continue;
        }
        let Some(item) = shared.krate.foreign.item(*def_id) else {
            continue;
        };
        if item.checked {
            continue;
        }
        // Named apart where its module imports the library's function by
        // that name, `f$1`, which the module exports as `f`: its imports
        // were named before it was made.
        let imported =
            (shared.krate.imports.get(module)).is_some_and(|imports| imports.values().any(|local| local == alias));
        let local = match imported {
            true => {
                let mut taken: IndexSet<String> = (shared.krate.imports.get(module).into_iter())
                    .flat_map(|imports| imports.values().cloned())
                    .collect();
                rust_js_program::names::fresh_apart(&mut taken, alias)
            }
            false => alias.clone(),
        };
        let (settled, names): (Settled, IndexSet<String>) = Default::default();
        let cx = item_cx((shared, &settled), *def_id, (*module, &names));
        let Some(function) = cx.library_forwarder(*def_id, (&local, alias), None) else {
            continue;
        };
        if let Some((_, named)) = reexported.get_mut(module) {
            let (taken, kept) = std::mem::take(named)
                .into_iter()
                .partition(|(_, _, (_, given))| given == alias);
            *named = kept;
            pass.forwarded_declared
                .entry(*module)
                .or_default()
                .extend::<Vec<_>>(taken);
        }
        super::copies::absorb(pass, *module, &mut cx.dependencies.borrow_mut());
        if imported {
            (pass.forwarder_aliases.entry(*module).or_default()).push((local, alias.clone()));
        }
        pass.functions.entry(*module).or_default().push(function);
    }
}

/// Each library module a module JS may call re-exports as a namespace,
/// `pub use dep::m as ns`, that has a function its own JS doesn't check
/// (`Item::checked`): a module of the crate's in its place, beside the module,
/// `ns.js`, which re-exports the library's every name, `export * from`, and
/// writes a checked forwarder of each unchecked one by its own name, which
/// shadows the star's (#2536, the lead's design), the module's `export * as
/// ns from` its; else the library's file, as before. A bundler still drops
/// what nothing reads of it.
pub(super) fn library_namespaces<'tcx, D>(
    shared: &Shared<'_, 'tcx>,
    js_reached: &rust_js_lower::reexports::JsReached,
    reexported: &mut IndexMap<LocalModId, (D, Vec<rust_js_lower::reexports::Named>)>,
    paths: &IndexMap<LocalModId, Vec<String>>,
) -> Vec<rust_js_program::program::UnlinkedModule> {
    use rust_js_program::js;
    let (tcx, foreign) = (shared.tcx, shared.krate.foreign);
    let mut made = Vec::new();
    for (module, named, alias, ident) in &js_reached.namespaces {
        let unchecked: Vec<(DefId, String)> = (tcx.module_children(*named).iter())
            .filter_map(|child| match child.res {
                rustc_hir::def::Res::Def(rustc_hir::def::DefKind::Fn, id) => (foreign.item(id))
                    .filter(|item| !item.checked && item.member.is_none())
                    .map(|item| (id, item.export.clone())),
                _ => None,
            })
            .collect();
        if unchecked.is_empty() {
            continue;
        }
        let Some((_, list)) = reexported.get_mut(module) else {
            continue;
        };
        let Some(entry) = list
            .iter_mut()
            .find(|(_, _, (export, given))| export == "*" && given == alias)
        else {
            continue;
        };
        let js::ImportFrom::Specifier(specifier) = entry.1.clone() else {
            continue;
        };
        let Some(file_exports) = foreign.file_exports(&specifier).cloned() else {
            continue;
        };
        let mut path = paths[module].clone();
        path.push(ident.clone());
        // Beside its module, a directory deeper than a module of a path's
        // own: the library's file as it names it.
        let from = match paths[module].is_empty() || !specifier.starts_with('.') {
            true => specifier.clone(),
            false => format!("../{}", specifier.trim_start_matches("./")),
        };
        let namespace = rust_js_program::casing::namespace_name(ident);
        let mut items: Vec<js::Item> = Vec::new();
        for (def_id, export) in &unchecked {
            let (settled, names): (Settled, IndexSet<String>) = Default::default();
            let cx = item_cx((shared, &settled), *def_id, (*module, &names));
            let callee = js::Expr::member(js::Expr::var(js::Name::module(&namespace)), export);
            let Some(function) = cx.library_forwarder(*def_id, (export, export), Some(callee)) else {
                continue;
            };
            for checker in cx
                .dependencies
                .borrow_mut()
                .checkers
                .drain(..)
                .filter_map(|(_, checker)| checker)
            {
                match checker {
                    rust_js_lower::Checker::Function(checker) => {
                        if !items
                            .iter()
                            .any(|item| matches!(item, js::Item::Function(f) if f.name == checker.name))
                        {
                            items.push(js::Item::Function(checker));
                        }
                    }
                    rust_js_lower::Checker::Shape(checker) => {
                        if !items
                            .iter()
                            .any(|item| matches!(item, js::Item::Const(c) if c.name == checker.name))
                        {
                            items.push(js::Item::Const(checker));
                        }
                    }
                }
            }
            items.push(js::Item::Function(function));
        }
        entry.1 = js::ImportFrom::Module(path.clone());
        let names: Vec<(String, String)> = (file_exports.iter())
            .filter(|name| *name != "default")
            .map(|name| (name.clone(), name.clone()))
            .collect();
        let lowered_from = from.clone();
        let lowered = rust_js_program::program::LoweredModule {
            path,
            file: rust_js_lower::module_file(tcx, *module).name.clone().into_local_path(),
            code: js::Span::NONE,
            unprinted: Vec::new(),
            directives: Vec::new(),
            packages: vec![js::Package {
                from: from.clone(),
                default: None,
                named: Vec::new(),
                namespace: Some(namespace.clone()),
                span: js::Span::NONE,
                named_at: js::Span::NONE,
            }],
            imports: Vec::new(),
            import_names: js::ImportNames::default(),
            taken_into: Vec::new(),
            reexports: vec![rust_js_program::program::Reexport {
                from: js::ImportFrom::Specifier(from.clone()),
                written: vec![(js::Span::NONE, js::Span::NONE); names.len()],
                named: names,
                span: js::Span::NONE,
                at: js::Span::NONE,
                glob: true,
                file_exports: Some(file_exports),
            }],
            aliases: Vec::new(),
            items,
            default_export: None,
            default_declared: false,
            // Its types the library's, each forwarder's its function's too:
            // `export * from` its file, which TypeScript finds the `.d.ts` of.
            declarations: Some(serde_json::json!({
                "declarations": [{ "kind": "export", "from": lowered_from, "names": [["*", "*"]] }],
            })),
            runtime: Vec::new(),
            jsx: false,
            located: false,
            types_only: false,
            reexports_alone: false,
        };
        let reserved = std::iter::once(namespace)
            .chain(unchecked.into_iter().map(|(_, export)| export))
            .collect();
        made.push(rust_js_program::program::UnlinkedModule {
            module: lowered,
            imports: Vec::new(),
            reserved_names: reserved,
        });
    }
    made
}

//! The names a crate's items have in JS, and what each module imports and
//! exports (ADRs 0021, 0100).

use crate::lower::bindings;
use crate::lower::bindings::{Export, is_binding, js_import, js_path, module_binding};
use crate::lower::traits;
use crate::lower::{Body, FnInfo, camel_case, fresh_in};
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{self as hir, ItemKind, UseKind};
use rustc_middle::thir::{ExprId, ExprKind, Pat, PatKind, StmtKind};
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// What of JS the crate's bodies use (ADRs 0024, 0028).
pub(super) struct JsUses {
    /// JS globals, whether declared here or in another crate (`web`): every
    /// module reserves them, so a local named `console` can't hide the real one.
    pub(super) globals: HashSet<String>,
    /// Exports of JS modules, with the modules that use each.
    pub(super) imported: BTreeMap<Export, HashSet<LocalModId>>,
    /// What each import is bound to in Rust, to name a default import after.
    pub(super) bound_to: HashMap<Export, HashSet<DefId>>,
    /// The exports read through `require(module)`, not imported (ADR 0305).
    pub(super) required: HashSet<Export>,
    /// An import named after what the module it's from names it: the
    /// `const` a library's module holds a binding as (ADR 0633), or its
    /// item, its default's too (ADR 0773).
    pub(super) named: HashMap<Export, String>,
    /// The bindings a module holds as its `const`, whose `#[rust_js::name]`
    /// names that `const` (ADR 0633), not their import.
    pub(super) held: HashSet<DefId>,
}

pub(super) fn js_uses<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    held: &bindings::Held,
    instances: &crate::lower::Instances,
) -> JsUses {
    let mut uses = JsUses {
        globals: HashSet::new(),
        imported: BTreeMap::new(),
        bound_to: HashMap::new(),
        required: HashSet::new(),
        named: HashMap::new(),
        held: (held.modules.values().flatten())
            .map(|binding| binding.binding)
            .collect(),
    };
    // What a module's `const` of a binding reads (ADR 0633), `import * as
    // react from "react"` of `react.__CLIENT_INTERNALS..`.
    // What each path of a module's namespace names it, `* as Scheduler`'s.
    let mut namespaces: HashMap<Export, Vec<(Option<String>, DefId)>> = HashMap::new();
    for (&module, bindings) in &held.modules {
        for binding in bindings.iter().filter(|binding| binding.through.is_none()) {
            if let Some((export, _)) = js_import(&binding.path) {
                namespaces.entry(export.clone()).or_default().push((
                    bindings::namespace_name(&binding.path).map(str::to_string),
                    binding.binding,
                ));
                uses.bound_to.entry(export.clone()).or_default().insert(binding.binding);
                uses.imported.entry(export).or_default().insert(module);
            }
        }
    }
    for body in all_bodies {
        let module = crate::lower::body_module(tcx, instances, body.def_id);
        // What `js::import!` imports when it's asked for, which nothing
        // imports statically (ADR 0304).
        let dynamic: HashSet<ExprId> = (body.thir.exprs.iter())
            .filter_map(|expr| match expr.kind {
                ExprKind::Call { fun, ref args, .. }
                    if crate::lower::fn_def(body.thir[fun].ty).is_some_and(|(id, _)| {
                        is_binding(tcx, id) && matches!(bindings::js_form(tcx, id), bindings::JsForm::Import { .. })
                    }) =>
                {
                    args.first()
                        .map(|&arg| crate::lower::body_queries::strip(&body.thir, arg))
                }
                _ => None,
            })
            .collect();
        // Each binding a body names, but one read through the module that
        // holds it, which is that module's (ADR 0633); of a body of MIR
        // alone, a library's the crate compiles (ADR 0914), each its MIR
        // names.
        let named: Vec<DefId> = (body.thir.exprs.iter_enumerated())
            .filter(|(id, _)| !dynamic.contains(id))
            .filter(|(_, expr)| !held.reads.contains_key(&expr.span))
            .filter_map(|(_, expr)| match (&expr.kind, expr.ty.kind()) {
                (ExprKind::ZstLiteral { .. }, ty::FnDef(def_id, _)) | (ExprKind::StaticRef { def_id, .. }, _) => {
                    Some(*def_id)
                }
                _ => None,
            })
            .chain(match body.root() {
                Some(_) => Vec::new(),
                None => crate::lower::instances::named(body),
            })
            .collect();
        for def_id in named {
            if !is_binding(tcx, def_id) {
                continue;
            }
            match js_path(tcx, def_id).as_deref().map(|path| (path, js_import(path))) {
                Some((_, Some((export, _)))) if bindings::requires(tcx, def_id) => {
                    uses.globals.insert("require".to_string());
                    uses.required.insert(export);
                }
                Some((path, Some((export, _)))) => {
                    namespaces
                        .entry(export.clone())
                        .or_default()
                        .push((bindings::namespace_name(path).map(str::to_string), def_id));
                    uses.bound_to.entry(export.clone()).or_default().insert(def_id);
                    uses.imported.entry(export).or_default().insert(module);
                }
                Some((path, None)) => {
                    // A global's name is its own, which no import names.
                    if let Some(name) = bindings::given_name(tcx, def_id)
                        && !uses.held.contains(&def_id)
                    {
                        tcx.dcx().span_err(
                            tcx.def_span(def_id),
                            format!(
                                "rust-js: `#[rust_js::name = {name:?}]` names a binding's import, and `{path}` is a global's, which is read by its own name (ADR 0028, amended)"
                            ),
                        );
                    }
                    uses.globals
                        .insert(path.split('.').next().unwrap_or_default().to_string());
                }
                None => {}
            }
        }
        // An untagged enum's variant told by a function its payload's type
        // names, `isValidElement`: the function, where a pattern tests it.
        let mut tested = |pat: &Pat<'tcx>| {
            pat.walk_always(|p| {
                if let PatKind::Variant {
                    adt_def,
                    args,
                    variant_index,
                    ..
                } = &p.kind
                {
                    for test in crate::lower::untagged::tests_of(tcx, *adt_def, args, adt_def.variant(*variant_index)) {
                        if let Some((export, _)) = js_import(&test) {
                            uses.imported.entry(export).or_default().insert(module);
                        }
                    }
                }
            });
        };
        for arm in body.thir.arms.iter() {
            tested(&arm.pattern);
        }
        for stmt in body.thir.stmts.iter() {
            if let StmtKind::Let { pattern, .. } = &stmt.kind {
                tested(pattern);
            }
        }
        for expr in body.thir.exprs.iter() {
            if let ExprKind::Let { pat, .. } = &expr.kind {
                tested(pat);
            }
        }
    }
    given_imports(tcx, &uses);
    // A namespace named as `import * as Scheduler` writes it, where each
    // path of its module names it so; one that names it otherwise, or
    // doesn't, refused (ADR 0256, amended).
    for (export, paths) in namespaces {
        let names: BTreeSet<Option<&str>> = paths.iter().map(|(name, _)| name.as_deref()).collect();
        match names.iter().collect::<Vec<_>>().as_slice() {
            // One that names it as it's named anyway is said twice: written
            // `from#*.path` (rd114b, ADR 1018), as a `#[rust_js::name]` that
            // repeats its default is refused.
            [Some(name)] if *name == namespace_default(tcx, &uses, &export) => {
                let at = paths
                    .iter()
                    .map(|(_, id)| *id)
                    .min_by_key(|&id| tcx.def_span(id))
                    .unwrap_or(paths[0].1);
                tcx.dcx().span_err(
                    tcx.def_span(at),
                    format!(
                        "rust-js: `{}#* as {name}` names the namespace as it's named anyway: write `{}#*.path` (ADR 1018)",
                        export.0, export.0
                    ),
                );
            }
            [Some(name)] => {
                uses.named.insert(export, name.to_string());
            }
            [None] => {}
            _ => {
                let at = paths
                    .iter()
                    .find(|(name, _)| *name != paths[0].0)
                    .map_or(paths[0].1, |(_, id)| *id);
                let written: Vec<String> = (names.iter())
                    .map(|name| name.map_or_else(|| "with no name".to_string(), |name| format!("as `{name}`")))
                    .collect();
                tcx.dcx().span_err(
                    tcx.def_span(at),
                    format!(
                        "rust-js: `{}` is imported {}: each `link_name` of its `*` names it the same, `{}#* as Name.path` (ADR 0256, amended)",
                        export.0,
                        written.join(" and "),
                        export.0
                    ),
                );
            }
        }
    }
    uses
}

/// Each import's name in each module: after the export, or the module for
/// a default or namespace import. A default import held by one `static` is
/// named after it, as JS code names an asset: `static hero_img` is `import
/// heroImg from "./hero.png"`. It's named around the globals, the module's
/// items, as `taken` has them, and its other imports, so only a file with an
/// item of its name renames it (ADR 0202). Every module names every
/// import: a library's export (ADR 0100), or one a trait's default copied
/// into an impl uses (ADR 0049), is used by modules known only once they're
/// lowered.
/// Namespaces are named last, so a module's default export gets its plain name.
pub(super) fn name_imports(
    tcx: TyCtxt<'_>,
    uses: &JsUses,
    taken: &HashMap<LocalModId, HashSet<String>>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
) -> HashMap<LocalModId, HashMap<Export, String>> {
    let (namespaces, others): (Vec<&Export>, Vec<&Export>) =
        uses.imported.keys().partition(|(_, export)| export == "*");
    let bases: Vec<(&Export, String)> = others
        .into_iter()
        .chain(namespaces)
        .map(|export_key| {
            let (from, export) = export_key;
            // What holds the export itself, `next/app#default`, not one of its
            // members, `next/app#default.getInitialProps`.
            let itself = |id: &&DefId| binds_itself(tcx, **id);
            let held_by = match uses
                .bound_to
                .get(export_key)
                .map(|held| held.iter().filter(itself).collect::<Vec<_>>())
                .unwrap_or_default()
                .as_slice()
            {
                [only] if matches!(tcx.def_kind(**only), DefKind::Static { .. }) => {
                    Some(camel_case(tcx.item_name(**only).as_str()))
                }
                // A component's, `next/image#default` of `fn Image`, is its
                // name, `import Image from "next/image"`, as JSX needs one
                // that's capitalized (ADR 0192).
                [only] if tcx.def_kind(**only) == DefKind::Fn => Some(bindings::fn_name(tcx, **only)),
                _ => None,
            };
            // A namespace whose bindings a Rust module holds, `mod ContextMenu`,
            // is that module's name (ADR 0256), where the module is the
            // namespace's, holding nothing else (`holds_only`); a default
            // class whose statics one holds, `mod app`, is the class's, `App`
            // (ADR 0355): a module of its crate's, or of a bindings crate's,
            // `next::app::app`.
            let holder = holder(tcx, uses, export_key);
            let statics = uses
                .bound_to
                .get(export_key)
                .is_some_and(|held| !held.iter().any(|id| itself(&id)));
            let base = match (export.as_str(), held_by, holder) {
                _ if uses.named.contains_key(export_key) => uses.named[export_key].clone(),
                ("default", Some(name), _) => name,
                ("default", None, Some(module)) if statics => {
                    crate::lower::upper_first(&camel_case(tcx.item_name(module).as_str()))
                }
                // A namespace PascalCase, as every namespace is, `side_menu` as
                // `SideMenu` (rd114b, ADR 1018; ADR 0256, amended). A module
                // of other items too, a file that calls a few of its members,
                // names it by what it's imported from, `Scheduler` of
                // `scheduler`, as the crate's root does (ADR 0256, amended).
                ("*", _, _) => namespace_default(tcx, uses, export_key),
                ("default", _, _) => module_binding(from),
                _ => export.clone(),
            };
            (export_key, base)
        })
        .collect();
    taken
        .iter()
        .map(|(&module, items)| {
            let mut reserved: HashSet<String> = uses.globals.iter().chain(items).cloned().collect();
            let renamed = renamed_in(tcx, module);
            // A library's item is named as the module's `use` renames it,
            // its default or its name: `use shared::enqueue_task::enqueue_task
            // as queue_macrotask;` is `import queueMacrotask from
            // "../shared/enqueueTask.js"`, as React's ReactAct has it (ADR
            // 0202, amended).
            let library_renamed: HashMap<Export, &String> = (renamed.iter())
                .filter_map(|(&id, alias)| foreign.import_key(id, module).map(|export| (export, alias)))
                .collect();
            // What the module imports is named first, so it's named as it is
            // where another module imports another of its name: next/image's
            // `Image` there, next/legacy/image's here.
            let (own, others): (Vec<_>, Vec<_>) = bases
                .iter()
                .partition(|(export, _)| uses.imported[*export].contains(&module));
            let names = (own.into_iter().chain(others))
                .map(|(export, base)| {
                    // An import is named as the module renames what holds
                    // it: `use …::Link as NextLink` is `import NextLink from
                    // "next/link"`, as react.dev's MDX `Link` has it beside
                    // its own `Link`; `use react::use_id as make_id` is
                    // `import { useId as makeId }`, the binding of the
                    // export itself, not of a member of it.
                    let default = export.1 == "default";
                    let alias = (uses.bound_to.get(*export))
                        .and_then(|held| {
                            (held.iter())
                                .filter(|&&id| default || binds_itself(tcx, id))
                                .filter_map(|id| renamed.get(id))
                                .min()
                                .cloned()
                        })
                        // Or as a binding of it the module declares names
                        // it, its `#[rust_js::name]` (ADR 0028, amended).
                        .or_else(|| given_in(tcx, uses, export, module))
                        .or_else(|| library_renamed.get(*export).map(|alias| (*alias).clone()));
                    (
                        (*export).clone(),
                        fresh_in(&mut reserved, alias.as_ref().unwrap_or(base)),
                    )
                })
                .collect();
            (module, names)
        })
        .collect()
}

/// What a binding's namespace, `export`, a `*`, is named where no path
/// names it, `#* as Name` (ADR 0256, amended): the module that holds its
/// bindings alone, `ContextMenu` of `mod ContextMenu` (`holds_only`), else
/// what it's imported from, `Scheduler` of `scheduler`, as
/// `crate::lower::namespace_name` names every namespace (rd114b, ADR
/// 1018).
fn namespace_default(tcx: TyCtxt<'_>, uses: &JsUses, export: &Export) -> String {
    match holder(tcx, uses, export) {
        Some(module) if holds_only(tcx, module, &export.0) => {
            crate::lower::namespace_name(tcx.item_name(module).as_str())
        }
        _ => crate::lower::namespace_name(bindings::file_stem(&export.0)),
    }
}

/// The one module of the crate's, or of a bindings crate's, but a root,
/// that holds each binding of `export`, `mod ContextMenu` of
/// `@radix-ui/react-context-menu#*`'s, if one does (ADR 0256).
fn holder(tcx: TyCtxt<'_>, uses: &JsUses, export: &Export) -> Option<DefId> {
    let module_of = |id: &DefId| {
        let mut module = tcx.parent(*id);
        while tcx.def_kind(module) != DefKind::Mod {
            module = tcx.parent(module);
        }
        module
    };
    let mut modules = uses.bound_to.get(export)?.iter().map(module_of);
    let first = modules.next()?;
    (!first.is_crate_root() && modules.all(|module| module == first)).then_some(first)
}

/// Whether `module` is `from`'s namespace's own, of no code: what it holds
/// are bindings of the namespace, `#*`, and the types they take, their
/// props, with what a derive or a macro writes of them and bindings of
/// their own, and its `use`s. `mod ContextMenu` is, as an `import * as`
/// is; a file of other code that calls a few of its members isn't, nor a
/// module of another module's bindings too (ADR 0256, amended).
fn holds_only(tcx: TyCtxt<'_>, module: DefId, from: &str) -> bool {
    let of_it = |id: DefId| {
        let path = js_path(tcx, id);
        path.as_deref()
            .and_then(js_import)
            .is_some_and(|((f, export), _)| f == from && export == "*")
    };
    // What declares and runs nothing: a type, a trait, and a macro, as
    // `jsx!` writes one of a component's props (ADR 0213).
    let a_type = |kind: DefKind| {
        matches!(
            kind,
            DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::TyAlias | DefKind::Trait | DefKind::Macro(_)
        )
    };
    let Some(local) = module.as_local() else {
        return tcx.module_children(module).iter().all(|child| match child.res {
            Res::Def(DefKind::Fn | DefKind::Static { .. }, id) => of_it(id),
            Res::Def(kind, _) => a_type(kind),
            _ => false,
        });
    };
    let items = tcx.hir_module_items(LocalModId::new_unchecked(local));
    let free = items.free_items().all(|id| {
        let item = tcx.hir_item(id);
        let def_id = item.owner_id.to_def_id();
        match item.kind {
            ItemKind::Use(..) | ItemKind::ForeignMod { .. } => true,
            // A derive's, or what a macro writes of the props, `jsx!`'s
            // `impl Lifetimes for PartProps` (ADR 0213), or one of bindings.
            ItemKind::Impl(..) => {
                item.span.from_expansion()
                    || (tcx.associated_item_def_ids(def_id).iter()).all(|&id| is_binding(tcx, id))
            }
            _ => a_type(tcx.def_kind(def_id)) || of_it(def_id),
        }
    });
    free && items.foreign_items().all(|id| of_it(id.owner_id.to_def_id()))
}

/// Whether a binding holds a JS module's export itself, `next/app#default`,
/// not one of its members, `next/app#default.getInitialProps`.
/// Whether each binding's `#[rust_js::name]` names its import, `import {
/// NotPendingTransition as NoPendingHostTransition }` (ADR 0028, amended):
/// one of an export itself, a name or a default, each binding of the
/// export a module declares naming it the same. One of a global, a member of an export or a
/// namespace, which no import of it names, is rust-js's error, as is a
/// second name of one import. Not one a module holds as its `const`,
/// which its name names (ADR 0633).
fn given_imports(tcx: TyCtxt<'_>, uses: &JsUses) {
    // Each import as each module that declares a binding of it names it.
    type Declared<'a> = (&'a Export, Option<LocalModId>);
    let mut named: HashMap<Declared<'_>, Vec<(String, DefId)>> = HashMap::new();
    for (export, held) in &uses.bound_to {
        for &id in held.iter().filter(|id| !uses.held.contains(*id)) {
            let Some(name) = bindings::given_name(tcx, id) else {
                continue;
            };
            if export.1 == "*" || !binds_itself(tcx, id) {
                let path = js_path(tcx, id).unwrap_or_default();
                tcx.dcx().span_err(
                    tcx.def_span(id),
                    format!(
                        "rust-js: `#[rust_js::name = {name:?}]` names a binding's import, and `{path}` imports no export of its own name: name a namespace by its path, `#* as Name.path`, or the export a member is of (ADR 0028, amended)"
                    ),
                );
                continue;
            }
            named
                .entry((export, declared_in(tcx, id)))
                .or_default()
                .push((name, id));
        }
    }
    for ((export, _), mut names) in named {
        // In the order they're written, which the error names them in.
        names.sort_by_key(|(_, id)| tcx.def_span(*id).lo());
        if let Some((name, id)) = names.iter().find(|(name, _)| *name != names[0].0) {
            tcx.dcx().span_err(
                tcx.def_span(*id),
                format!(
                    "rust-js: `{}#{}` is imported as `{}` and as `{name}`: each binding of one import names it the same (ADR 0028, amended)",
                    export.0, export.1, names[0].0
                ),
            );
        }
    }
}

/// The module a binding is declared in, of the crate's.
fn declared_in(tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
    id.as_local().map(|id| tcx.parent_module_from_def_id(id))
}

/// What `module` imports `export` by where a binding of the export itself
/// it declares says, its `#[rust_js::name]`, `import { NotPendingTransition
/// as NoPendingHostTransition }`: of each module that declares one, its
/// own (ADR 0028, amended). Not of one the module holds as its `const`.
fn given_in(tcx: TyCtxt<'_>, uses: &JsUses, export: &Export, module: LocalModId) -> Option<String> {
    let default = export.1 == "default";
    (uses.bound_to.get(export).into_iter().flatten())
        .filter(|&&id| (default || binds_itself(tcx, id)) && !uses.held.contains(&id))
        .filter(|&&id| declared_in(tcx, id) == Some(module))
        .filter_map(|&id| bindings::given_name(tcx, id))
        .min()
}

fn binds_itself(tcx: TyCtxt<'_>, id: DefId) -> bool {
    js_path(tcx, id)
        .as_deref()
        .and_then(js_import)
        .is_some_and(|(_, member)| member.is_empty())
}

/// What a module's `use`s rename, by their new names, camel case as any
/// name is (ADR 0573): `use …::Link as NextLink` is next/link's `Link` by
/// `NextLink`. Not a `pub use` that re-exports it, whose name is what it's
/// exported by (`bindings::alias_name`, ADR 0596), not the module's own.
/// A `use` of a name the item's module exports it by, `use
/// crate::scheduler::unstable_now;`, names it so, as it's exported, not
/// camelCase (`default_exports::own_aliases`, ADR 0596, amended).
pub(in crate::lower) fn renamed_in(tcx: TyCtxt<'_>, module: LocalModId) -> HashMap<DefId, String> {
    let mut renamed: HashMap<DefId, String> = HashMap::new();
    for (def_id, alias, _) in use_renames(tcx, module) {
        // Of two, the first written: one import has one name in a file,
        // and two that differ are refused (`refuse_two_names`).
        renamed.entry(def_id).or_insert(alias);
    }
    renamed
}

/// Each `use` of a module that isn't a re-export, where it's written, and
/// what it names, in each namespace: which import a `use` makes, which its
/// comments are before (rd92), the first of those naming what it imports.
pub(in crate::lower) struct UsesWritten {
    module: LocalModId,
    uses: Vec<(rustc_span::Span, Vec<DefId>)>,
}

impl UsesWritten {
    /// `module`'s, in the order written, each a whole statement, which each
    /// of a tree's leaves shares.
    pub(in crate::lower) fn of(tcx: TyCtxt<'_>, module: LocalModId) -> Self {
        let items = || tcx.hir_module_items(module).free_items().map(|id| tcx.hir_item(id));
        // A tree's, `use crate::{a::x, b::y};`: its leaves' spans are their
        // paths', the statement its list's, the outermost.
        let lists: Vec<rustc_span::Span> = (items())
            .filter(|item| matches!(item.kind, ItemKind::Use(_, UseKind::ListStem)))
            .map(|item| item.span)
            .collect();
        let statement = |span: rustc_span::Span| {
            (lists.iter().filter(|list| list.contains(span))).fold(span, |outer, &list| outer.to(list))
        };
        let mut uses: Vec<(rustc_span::Span, Vec<DefId>)> = (items())
            .filter_map(|item| {
                let ItemKind::Use(path, UseKind::Single(_)) = item.kind else {
                    return None;
                };
                if super::is_reexport(tcx, item) {
                    return None;
                }
                let named = (path.res.present_items())
                    .filter_map(|res| match res {
                        Res::Def(_, def_id) => Some(def_id),
                        _ => None,
                    })
                    .collect();
                Some((statement(item.span), named))
            })
            .collect();
        uses.sort_by_key(|(span, _)| span.lo());
        UsesWritten { module, uses }
    }

    /// The first naming what `makes` says is of the import.
    fn first(&self, makes: impl Fn(DefId) -> bool) -> Option<rustc_span::Span> {
        (self.uses.iter().find(|(_, named)| named.iter().any(|&id| makes(id)))).map(|&(span, _)| span)
    }

    /// The `use` of the import from the JS file `from`: of a library's item
    /// a consumer imports from it (ADR 0100), or a binding's.
    pub(in crate::lower) fn package_at(
        &self,
        tcx: TyCtxt<'_>,
        foreign: &crate::lower::library::Foreign<'_, '_>,
        from: &str,
    ) -> Option<rustc_span::Span> {
        self.first(|id| {
            let export = foreign.import_key(id, self.module).or_else(|| {
                (is_binding(tcx, id))
                    .then(|| js_path(tcx, id))
                    .flatten()
                    .and_then(|path| js_import(&path).map(|(export, _)| export))
            });
            export.is_some_and(|(of, _)| of == from)
        })
    }

    /// The `use` of the import from the crate's module `target`: of its
    /// item, or of it or a module it's in.
    pub(in crate::lower) fn module_at(
        &self,
        tcx: TyCtxt<'_>,
        fns: &HashMap<DefId, FnInfo>,
        target: LocalModId,
    ) -> Option<rustc_span::Span> {
        self.first(|id| match tcx.def_kind(id) {
            DefKind::Mod => tcx.is_descendant_of(target.to_def_id(), id),
            _ => {
                (fns.get(&id).map(|f| f.module))
                    .or_else(|| id.as_local().map(|local| tcx.parent_module_from_def_id(local)))
                    == Some(target)
            }
        })
    }
}

/// Each `use` of the module that renames an item, in the order written:
/// what it names, the import's name, and where it's written.
fn use_renames(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<(DefId, String, rustc_span::Span)> {
    tcx.hir_module_items(module)
        .free_items()
        .filter_map(|id| {
            let item = tcx.hir_item(id);
            match item.kind {
                ItemKind::Use(path, UseKind::Single(ident)) if !super::is_reexport(tcx, item) => {
                    let Some(Res::Def(_, def_id)) = path.res.value_ns else {
                        return None;
                    };
                    // Its `#[rust_js::name]`, the import's local name as
                    // written, `_jsxDEV` (ADR 0596, amended).
                    let given = bindings::given_name(tcx, item.owner_id.to_def_id());
                    // Nor one that names it for nothing, `use page as _;`,
                    // `js::export_default!(page)`'s.
                    if given.is_none()
                        && (ident.name == tcx.item_name(def_id) || ident.name == rustc_span::kw::Underscore)
                    {
                        return None;
                    }
                    let alias = given.or_else(|| crate::lower::default_exports::alias_named(tcx, def_id, ident.name));
                    Some((
                        def_id,
                        alias.unwrap_or_else(|| bindings::alias_cased(tcx, def_id, ident.name.as_str())),
                        item.span,
                    ))
                }
                _ => None,
            }
        })
        .collect()
}

/// A module whose `use`s give one item two names, `use dep::helper as
/// assist;` and `use dep::helper as aid;`: refused, as its file imports it
/// once, by one name, and either would leave the other unsaid (ADR 0202,
/// amended). Not its own item, which its file declares and imports not.
pub(super) fn refuse_two_names(tcx: TyCtxt<'_>, modules: &[LocalModId]) -> bool {
    let mut failed = false;
    for &module in modules {
        // A `#[rust_js::name]` names the import of a `use` of one name; of
        // a group or a glob it would name each the same.
        for id in tcx.hir_module_items(module).free_items() {
            let item = tcx.hir_item(id);
            if let ItemKind::Use(_, UseKind::ListStem | UseKind::Glob) = item.kind
                && bindings::given_name(tcx, item.owner_id.to_def_id()).is_some()
            {
                let message = "rust-js: `#[rust_js::name]` names the import of a `use` of one name, \
                    `use priorities::IMMEDIATE_PRIORITY as IMMEDIATE_SCHEDULER_PRIORITY;`: \
                    write a `use` of its own for the name it gives";
                tcx.dcx().span_err(item.span, message);
                failed = true;
            }
        }
        let mut first: HashMap<DefId, String> = HashMap::new();
        // Of its own item, its file declares it, and imports nothing.
        let own = |def_id: DefId| {
            def_id
                .as_local()
                .is_some_and(|id| tcx.parent_module_from_def_id(id) == module)
        };
        for (def_id, alias, span) in use_renames(tcx, module)
            .into_iter()
            .filter(|&(def_id, ..)| !own(def_id))
        {
            match first.get(&def_id) {
                Some(named) if *named != alias => {
                    let message = format!(
                        "rust-js: this module's file imports `{}` once, by one name, and another `use` names it `{named}`; name it once",
                        tcx.item_name(def_id)
                    );
                    tcx.dcx().span_err(span, message);
                    failed = true;
                }
                Some(_) => {}
                None => _ = first.insert(def_id, alias),
            }
        }
    }
    failed
}

/// An item's JS name before it's made unique: a trait impl's accessor
/// (`circleShape`, ADR 0049), a trait method's body (`circleShape_area`,
/// or `shape_name` for a default), or the item's own name.
pub(super) fn item_js_name(tcx: TyCtxt<'_>, def_id: DefId, trait_impls: &[DefId]) -> String {
    if trait_impls.contains(&def_id) {
        return traits::impl_name(tcx, def_id);
    }
    // A `const { .. }` block, which has no name: its function's, `mainConst`
    // (ADR 0127).
    if tcx.def_kind(def_id) == DefKind::AnonConst {
        let function = tcx.typeck_root_def_id(def_id);
        return format!("{}Const", crate::lower::lower_first(tcx.item_name(function).as_str()));
    }
    if crate::lower::classes::shown_class(tcx, def_id).is_some() {
        return "toString".to_owned();
    }
    if tcx.def_kind(def_id) == DefKind::AssocFn && tcx.inherent_impl_of_assoc(def_id).is_none() {
        let parent = tcx.parent(def_id);
        let prefix = if trait_impls.contains(&parent) {
            traits::impl_name(tcx, parent)
        } else {
            crate::lower::lower_first(tcx.item_name(parent).as_str())
        };
        return format!("{prefix}_{}", bindings::fn_name(tcx, def_id));
    }
    // A class's `const`, its prototype's property (ADR 0761).
    if let Some(name) = crate::lower::classes::const_name(tcx, def_id) {
        return name;
    }
    bindings::fn_name(tcx, def_id)
}

/// Each item's JS name, unique within its module's file, and each module's
/// names so far (`taken`, which also gets the import aliases, so local
/// variables avoid both). A method is its type's (ADR 0047): a property of
/// the object named after the type, unique in the module, and its name is
/// unique among the type's. What a module exports by its name is named
/// first, so it's what it's written as, and an item the module keeps to
/// itself is named apart from it, `emptyContextObject$1`; two exports of
/// one name are refused (`exported_by_name`, ADR 0922). The last result
/// says whether it all went well.
pub(super) fn name_items(
    tcx: TyCtxt<'_>,
    items: &[DefId],
    modules: &[LocalModId],
    globals: &HashSet<String>,
    trait_impls: &[DefId],
    named_expressions: &HashSet<DefId>,
    instances: &crate::lower::Instances,
) -> (HashMap<LocalModId, HashSet<String>>, HashMap<DefId, FnInfo>, bool) {
    let mut failed = false;
    let mut taken: HashMap<LocalModId, HashSet<String>> = modules.iter().map(|&m| (m, globals.clone())).collect();
    let mut owners: HashMap<(LocalModId, DefId), String> = HashMap::new();
    let mut methods: HashMap<(LocalModId, DefId), HashSet<String>> = HashMap::new();
    let mut fns: HashMap<DefId, FnInfo> = HashMap::new();
    let mut exports: HashMap<(LocalModId, String), DefId> = HashMap::new();
    let exported = |def_id: DefId| bindings::exported_by_name(tcx, def_id) && !named_expressions.contains(&def_id);
    let (first, rest): (Vec<DefId>, Vec<DefId>) = items.iter().copied().partition(|&def_id| exported(def_id));
    for def_id in first.into_iter().chain(rest) {
        let module = crate::lower::body_module(tcx, instances, def_id);
        let names = taken.entry(module).or_default();
        let js_name = item_js_name(tcx, def_id, trait_impls);
        if exported(def_id) {
            match exports.entry((module, js_name.clone())) {
                Entry::Occupied(other) => {
                    let message = format!(
                        "rust-js: `{}` and `{}` are both exported as `{js_name}` in JS, where an importer reads one export by that name; give one another with `#[rust_js::name = \"..\"]`",
                        tcx.def_path_str(*other.get()),
                        tcx.def_path_str(def_id),
                    );
                    (tcx.dcx().struct_span_err(tcx.def_span(def_id), message))
                        .with_span_note(
                            tcx.def_span(*other.get()),
                            format!("`{js_name}` is exported here first"),
                        )
                        .emit();
                    failed = true;
                }
                Entry::Vacant(entry) => _ = entry.insert(def_id),
            }
        }
        if trait_impls.contains(&def_id) && names.contains(&js_name) {
            let message = format!(
                "rust-js: generated trait implementation name `{js_name}` collides; put the implementations in separate modules"
            );
            tcx.dcx().span_err(tcx.def_span(def_id), message);
            failed = true;
        }
        let owner_type = tcx.inherent_impl_of_assoc(def_id).and_then(|imp| {
            match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
                ty::Adt(adt, _) => Some(adt.did()),
                _ => None,
            }
        });
        // A class's `Display`'s `fmt`, its prototype's `toString` (#358).
        let owner_type = owner_type.or_else(|| crate::lower::classes::shown_class(tcx, def_id));
        // An object's method, a function of its module (ADR 0943).
        let owner_type = owner_type
            .filter(|_| crate::lower::classes::member(tcx, def_id) != Some(crate::lower::classes::Member::Own));
        let (name, owner) = match owner_type {
            Some(ty) => {
                // A class is its constructor function, named as its struct's
                // `#[rust_js::name]` says, `FiberNode` of React's `Fiber`, as
                // a binding's class is (ADR 0761, amended).
                let named = crate::lower::classes::name(tcx, ty).unwrap_or_else(|| tcx.item_name(ty).to_string());
                let owner = owners.entry((module, ty)).or_insert_with(|| fresh_in(names, &named));
                // A property, so a name JS reserves for variables, like `new`, is fine.
                let names = methods.entry((module, ty)).or_default();
                let name = crate::names::fresh_apart(names, &js_name);
                (name, Some(owner.clone()))
            }
            None if named_expressions.contains(&def_id) => (js_name, None),
            None => (fresh_in(names, &js_name), None),
        };
        fns.insert(def_id, FnInfo { module, name, owner });
    }
    (taken, fns, failed)
}

/// The functions and `const`s used by another module: those must be
/// exported, even if private in Rust (a child module may call its parent's
/// private functions).
pub(super) fn exported_across_modules<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    instances: &crate::lower::Instances,
) -> HashSet<DefId> {
    let mut exported = HashSet::new();
    for body in all_bodies {
        let from = crate::lower::body_module(tcx, instances, body.def_id);
        // A body of MIR alone, a library's the crate compiles (ADR 0914).
        if body.root().is_none() {
            for def_id in crate::lower::instances::named(body) {
                if fns.get(&def_id).is_some_and(|target| target.module != from) {
                    exported.insert(def_id);
                }
            }
        }
        for expr in body.thir.exprs.iter() {
            let def_id = match (&expr.kind, expr.ty.kind()) {
                (ExprKind::ZstLiteral { .. }, ty::FnDef(def_id, _))
                | (ExprKind::NamedConst { def_id, .. } | ExprKind::StaticRef { def_id, .. }, _) => def_id,
                _ => continue,
            };
            if let Some(target) = fns.get(def_id)
                && target.module != from
            {
                exported.insert(*def_id);
            }
        }
    }
    // Another module's function a module's default is, `export { twice as
    // default } from "./helpers.js"`, which reads it there.
    for (module, item) in crate::lower::default_exports::defaults(tcx) {
        if fns.get(&item).is_some_and(|target| target.module != module) {
            exported.insert(item);
        }
    }
    exported
}

/// The functions a block makes and gives, `{ fn Label(..) { .. } Label }`,
/// that nothing else names: each is JS's named function expression where
/// the block is, as react.dev's `memo(function Label(..) { .. })` (ADR 0296).
pub(super) fn named_expressions<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
) -> HashSet<DefId> {
    let mut named: HashMap<DefId, usize> = HashMap::new();
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            if let ExprKind::ZstLiteral { .. } = expr.kind
                && let ty::FnDef(def_id, _) = *expr.ty.kind()
                && def_id.is_local()
            {
                *named.entry(def_id).or_default() += 1;
            }
        }
    }
    named
        .into_iter()
        .filter(|&(def_id, uses)| uses == 1 && given_by_its_block(tcx, def_id.expect_local()))
        .map(|(def_id, _)| def_id)
        .collect()
}

/// The functions written in a function's body that only it and others of
/// them name: each is a function declaration where it's written there
/// (ADR 0308). One a static's initializer names stays the module's, as that
/// initializer is.
pub(super) fn local_functions<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
    named_expressions: &HashSet<DefId>,
) -> HashMap<DefId, rustc_span::Span> {
    let mut users: HashMap<DefId, HashSet<DefId>> = HashMap::new();
    for body in bodies {
        let root = tcx.typeck_root_def_id(body.def_id);
        for expr in body.thir.exprs.iter() {
            if let ExprKind::ZstLiteral { .. } = expr.kind
                && let ty::FnDef(def_id, _) = *expr.ty.kind()
            {
                users.entry(def_id).or_default().insert(root);
            }
        }
    }
    let mut local: HashSet<DefId> = (users.keys().copied())
        .filter(|&def_id| {
            def_id.as_local().is_some_and(|local| {
                tcx.def_kind(local) == DefKind::Fn
                    && !named_expressions.contains(&def_id)
                    && !is_binding(tcx, def_id)
                    && matches!(
                        tcx.parent_hir_node(tcx.local_def_id_to_hir_id(local)),
                        hir::Node::Stmt(_)
                    )
                    && matches!(
                        tcx.def_kind(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id())),
                        DefKind::Fn | DefKind::AssocFn
                    )
                    // Not of a trait's default, whose body each impl's
                    // dictionary copies into its own module (ADR 0185): the
                    // trait's module's, which each copy imports.
                    && tcx
                        .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))
                        .is_none()
            })
        })
        .collect();
    // Named elsewhere than in its function, or in another of them that
    // isn't one, it's the module's.
    loop {
        let parent = |def_id: DefId| tcx.typeck_root_def_id(tcx.local_parent(def_id.expect_local()).to_def_id());
        let outside: Vec<DefId> = (local.iter().copied())
            .filter(|&def_id| {
                users[&def_id]
                    .iter()
                    .any(|&user| user != def_id && user != parent(def_id) && !local.contains(&user))
            })
            .collect();
        if outside.is_empty() {
            break;
        }
        for def_id in outside {
            local.remove(&def_id);
        }
    }
    // Each in the block it's written in.
    (local.into_iter())
        .filter_map(|def_id| {
            let stmt = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(def_id.expect_local()));
            let hir::Node::Stmt(stmt) = stmt else { return None };
            let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
                return None;
            };
            Some((def_id, block.span))
        })
        .collect()
}

/// The `const`s written in a function's body that only what's written in
/// that body reads: the function, its closures and the functions written
/// in it that are its own (ADR 0308). Each is a `const` in the body it's
/// written in (ADR 0031, amended), and what reads it there, by each
/// `const`, the bodies that do. One a function of the module's reads, a
/// static's or another constant's initializer, or a `const { .. }`'s, is
/// the module's. Not a thread-local's (ADR 0270), nor one of a trait's
/// default, whose body each impl's dictionary copies into its own module
/// (ADR 0185).
pub(super) fn local_consts<'a, 'tcx: 'a>(
    tcx: TyCtxt<'tcx>,
    bodies: impl Iterator<Item = &'a Body<'tcx>>,
    consts: &[LocalDefId],
    local_functions: &HashMap<DefId, rustc_span::Span>,
) -> super::LocalConsts {
    let mut readers: HashMap<DefId, HashSet<DefId>> = HashMap::new();
    for body in bodies {
        for named in crate::lower::body_queries::consts_named(&body.thir) {
            readers.entry(named).or_default().insert(body.def_id);
        }
    }
    // The function of the module's an item is written in, through the
    // closures and the functions written in it, which are in its body too.
    let function = |def_id: DefId| {
        let mut at = tcx.parent(def_id);
        while tcx.is_closure_like(at) || local_functions.contains_key(&at) {
            at = tcx.parent(at);
        }
        at
    };
    (consts.iter())
        .filter(|&&local| {
            matches!(tcx.def_kind(local), DefKind::Const { .. })
                && tcx.item_name(local) != rustc_span::kw::Underscore
                && !super::is_thread_local(tcx, local)
                && super::in_thread_local(tcx, local).is_none()
                && matches!(
                    tcx.def_kind(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id())),
                    DefKind::Fn | DefKind::AssocFn
                )
                && tcx
                    .trait_of_assoc(tcx.typeck_root_def_id(tcx.local_parent(local).to_def_id()))
                    .is_none()
        })
        .filter_map(|&local| {
            let def_id = local.to_def_id();
            let readers = readers.remove(&def_id).unwrap_or_default();
            let at = function(def_id);
            let within = readers.iter().all(|&reader| {
                reader == at
                    || ((tcx.is_closure_like(reader) || local_functions.contains_key(&reader))
                        && function(reader) == at)
            });
            // In the block it's written in, as a statement.
            let hir::Node::Stmt(stmt) = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(local)) else {
                return None;
            };
            let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
                return None;
            };
            within.then_some((def_id, (block.span, readers)))
        })
        .collect()
}

/// Is `def_id` a function its block makes and then gives as its value?
fn given_by_its_block(tcx: TyCtxt<'_>, def_id: LocalDefId) -> bool {
    if tcx.def_kind(def_id) != DefKind::Fn || tcx.generics_of(def_id).count() != 0 {
        return false;
    }
    let hir::Node::Stmt(stmt) = tcx.parent_hir_node(tcx.local_def_id_to_hir_id(def_id)) else {
        return false;
    };
    let hir::Node::Block(block) = tcx.parent_hir_node(stmt.hir_id) else {
        return false;
    };
    matches!(block.expr, Some(hir::Expr { kind: hir::ExprKind::Path(hir::QPath::Resolved(None, path)), .. })
        if path.res == Res::Def(DefKind::Fn, def_id.to_def_id()))
}

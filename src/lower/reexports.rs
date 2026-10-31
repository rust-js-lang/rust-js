//! What a module re-exports, its `pub use`s (ADR 0240, amended): each
//! statement's `export { .. } from` where it's written, from the module its
//! path names, a JS module or a library's file, by what that exports it as;
//! a module's namespace, `export * as ns from`; a class's constructor. What
//! has JS but can't be said is refused here, never left out. Which `pub
//! use`s a module's file has is `analysis::is_reexport`'s.

use super::FnInfo;
use crate::js;
use crate::program::Reexport;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use rustc_span::def_id::{CRATE_MOD_ID, DefId, LocalModId};
use std::collections::HashMap;

/// What `module` re-exports, each `pub use` statement's `export { .. } from`
/// in the order they're written, by where it's from (ADR 0240, amended):
/// another of the crate's modules, by its path, the one the `use`'s path
/// names, through which it may be a re-export itself; a JS module a
/// binding names, or a library's file, by its specifier. Each `(export,
/// alias)`: what that module exports it as, and the name the `use` gives
/// it, `*` of a module, `export * as ns from`. And `default`, another
/// module's function its `js::export_default!` names, where that's
/// written. What a `pub use` names that has JS but can't be said is
/// refused, never left out.
pub(super) fn reexports(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    (fns, foreign, forwarded): (
        &HashMap<DefId, FnInfo>,
        &super::library::Foreign<'_, '_>,
        &HashMap<DefId, DefId>,
    ),
    paths: &HashMap<LocalModId, Vec<String>>,
    (default_from, held): (Option<(DefId, rustc_span::Span)>, &[super::bindings::HeldBinding]),
    sources: &super::sources::CapturedSources,
) -> Vec<Reexport> {
    // Each name, where it's written: the statement, its `use` item's span,
    // which each of a tree's leaves shares; and where it's from.
    let mut found: Vec<(rustc_span::Span, js::ImportFrom, (String, String))> = Vec::new();
    let refuse = |span, what: &str, why: &str| {
        super::refusal::refuse(tcx, span, super::refusal::Refused::Because(what, why));
    };
    // A binding it names through another module that holds it, that
    // module's export, by its name and as its default (ADR 0633), or a
    // library's file's.
    for held in held {
        let Some(super::bindings::HeldRead { holder, import, .. }) = &held.through else {
            continue;
        };
        let from = match holder {
            super::bindings::Holder::Module(holder) => match paths.get(holder) {
                Some(from) => js::ImportFrom::Module(from.clone()),
                None => continue,
            },
            super::bindings::Holder::Library(from) => js::ImportFrom::Specifier(from.clone()),
        };
        let at = held.span.unwrap_or(rustc_span::DUMMY_SP);
        if held.export {
            found.push((at, from.clone(), (import.clone(), held.name.clone())));
        }
        if held.default {
            found.push((at, from, (import.clone(), "default".to_string())));
        }
    }
    if let Some((function, span)) = default_from
        && let Some(info) = fns.get(&function)
        && let Some(from) = paths.get(&info.module)
    {
        let export = super::default_exports::exported_as(tcx, function, info.exported());
        found.push((
            span,
            js::ImportFrom::Module(from.clone()),
            (export, "default".to_string()),
        ));
    }
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Use(path, kind) = item.kind else {
            continue;
        };
        if !tcx.visibility(item.owner_id).is_public() {
            continue;
        }
        // The whole statement, from its `pub`, which each of a tree's
        // leaves shares, its comments before it.
        let statement = match item.vis_span.is_empty() {
            true => item.span,
            false => item.vis_span.to(item.span),
        };
        let hir::UseKind::Single(ident) = kind else {
            // `pub use m::*;` of what has JS: each name it gives the module,
            // as rustc resolves it, past what the module's own items and
            // single `use`s shadow, one `export { .. } from` (rd153).
            if matches!(kind, hir::UseKind::Glob) {
                globbed(
                    tcx,
                    (module, item.owner_id.to_def_id(), statement),
                    (fns, foreign),
                    paths,
                    &mut found,
                );
            }
            continue;
        };
        let alias = |name: &str| match ident.name == tcx.item_name(named_of(path)) {
            true => name.to_string(),
            false => super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),
        };
        // Its value: a function, a constant, a thread-local or a binding.
        if let Some(Res::Def(kind, def_id)) = path.res.value_ns
            && matches!(kind, DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. })
        {
            // A binding of a JS module's export, itself, not a path in it.
            if super::bindings::is_binding(tcx, def_id) {
                if let Some(written) = super::bindings::js_path(tcx, def_id)
                    && let Some(((from, export), "")) = super::bindings::js_import(&written)
                {
                    // Its namespace too, `export * as ns from` (ADR 0240, amended).
                    found.push((statement, js::ImportFrom::Specifier(from), (export, ident.to_string())));
                }
                // A global's, the module's `const` (ADR 0622), or one held
                // through another module, above.
                continue;
            }
            if let Some(info) = fns.get(&def_id) {
                // Its own item by another name is `aliases`'s.
                if info.owner.is_some() || info.module == module {
                    continue;
                }
                // From the module its path names, which may re-export it,
                // as written, by the name it has there where the `use`
                // keeps it; else where it's declared.
                let through = written_module(path)
                    .filter(|&through| through != info.module)
                    .and_then(|through| Some((through, exported_in(tcx, fns, through, last_name(path)?, def_id)?)));
                let (from, export, alias) = match through {
                    Some((through, export)) if Some(ident.name) == last_name(path) => (through, export.clone(), export),
                    Some((through, export)) => (through, export, alias(info.exported())),
                    None => (
                        info.module,
                        super::default_exports::exported_as(tcx, def_id, info.exported()),
                        alias(info.exported()),
                    ),
                };
                let Some(from) = paths.get(&from) else { continue };
                found.push((statement, js::ImportFrom::Module(from.clone()), (export, alias)));
                continue;
            }
            // A library's, from its file, as its manifest says (ADR 0100).
            if let Some(imported) = foreign.item(def_id)
                && imported.member.is_none()
                && let Some((from, export)) = foreign.import_key(def_id, module)
            {
                let from = js::ImportFrom::Specifier(from);
                found.push((statement, from, (export, alias(&imported.name))));
                continue;
            }
            let what = format!("a `pub use` of `{}`", tcx.def_path_str(def_id));
            refuse(
                statement,
                &what,
                "it's in no JS module of rust-js's to be re-exported from",
            );
            continue;
        }
        match path.res.type_ns {
            // A module, its namespace, `export * as ns from`.
            Some(Res::Def(DefKind::Mod, named)) => {
                let from = (named.as_local()).and_then(|local| paths.get(&LocalModId::new_unchecked(local)));
                match from {
                    Some(from) => {
                        let alias = super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name);
                        found.push((
                            statement,
                            js::ImportFrom::Module(from.clone()),
                            ("*".to_string(), alias),
                        ));
                    }
                    // One with no JS of its own, of types alone, a binding
                    // crate's or std's, is nothing; another crate's of
                    // rust-js's has no namespace a consumer knows.
                    None if !named.is_local() && foreign.is_library(named.krate) => refuse(
                        statement,
                        "a `pub use` of another crate's module",
                        "`pub use` each of its items by its name",
                    ),
                    None => {}
                }
            }
            // A class, its constructor function (ADR 0761): the crate's,
            // from its module; a library's, from its file.
            Some(Res::Def(DefKind::Struct, class)) if super::classes::is_class(tcx, class) => {
                let constructor = super::classes::constructor_of(tcx, class);
                if constructor
                    .and_then(|c| fns.get(&c))
                    .is_some_and(|info| info.module == module)
                {
                    let what = "a `pub use` of its own module's class by another name";
                    refuse(statement, what, "a class is exported by its own name");
                    continue;
                }
                let crate_class = || {
                    let info = fns.get(&constructor?)?;
                    let from = paths.get(&info.module)?;
                    Some((js::ImportFrom::Module(from.clone()), info.owner.clone()?))
                };
                let library_class = || {
                    let (from, export) = foreign.import_key(constructor?, module)?;
                    Some((js::ImportFrom::Specifier(from), export))
                };
                let Some((from, export)) = crate_class().or_else(library_class) else {
                    let why = "a class is its constructor, `fn new(..) -> Self`";
                    refuse(statement, "a `pub use` of a class without a constructor", why);
                    continue;
                };
                let alias = match ident.name == tcx.item_name(class) {
                    true => export.clone(),
                    false => super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),
                };
                found.push((statement, from, (export, alias)));
            }
            // A type, a trait or a macro: no JS (ADR 0240).
            _ => {}
        }
    }
    forwarded_methods(tcx, module, (fns, forwarded), paths, &mut found);
    // In the order they're written; a statement's names from one module
    // one `export { .. } from` of it, in their order.
    found.sort_by_key(|(at, ..)| at.lo());
    let mut reexports: Vec<(rustc_span::Span, Reexport)> = Vec::new();
    for (at, from, named) in found {
        match reexports
            .iter_mut()
            .find(|(was, reexport)| was.lo() == at.lo() && reexport.from == from)
        {
            Some((_, reexport)) => reexport.named.push(named),
            None => {
                let span = match at.is_dummy() {
                    true => js::Span::NONE,
                    false => sources.span(at),
                };
                reexports.push((
                    at,
                    Reexport {
                        from,
                        named: vec![named],
                        span,
                    },
                ));
            }
        }
    }
    // A run of them from one module, nothing but their attributes between,
    // no comment or blank line, one statement, as a person lists a module's
    // names once.
    let mut merged: Vec<Reexport> = Vec::new();
    for (_, reexport) in reexports {
        match merged.last_mut() {
            Some(last)
                if last.from == reexport.from
                    && !last.span.is_none()
                    && !reexport.span.is_none()
                    && (sources
                        .output
                        .text
                        .get(last.span.hi as usize..reexport.span.lo as usize))
                    .is_some_and(only_attributes) =>
            {
                last.named.extend(reexport.named);
                last.span.hi = reexport.span.hi;
            }
            _ => merged.push(reexport),
        }
    }
    merged
}

/// Whether `between`, the text between two items, is only whitespace and
/// attributes, `#[cfg_attr(..)]`, with no comment or blank line.
fn only_attributes(between: &str) -> bool {
    let mut rest = between;
    let mut blank = false;
    loop {
        let trimmed = rest.trim_start();
        let space = &rest[..rest.len() - trimmed.len()];
        blank |= space.matches('\n').count() > 1;
        if trimmed.is_empty() {
            return !blank;
        }
        let Some(attribute) = trimmed.strip_prefix("#[") else {
            return false;
        };
        let mut depth = 1;
        let Some(end) = attribute.char_indices().find_map(|(i, c)| {
            depth += match c {
                '[' => 1,
                ']' => -1,
                _ => 0,
            };
            (depth == 0).then_some(i)
        }) else {
            return false;
        };
        rest = &attribute[end + 1..];
    }
}

/// The names a module's `pub use m::*;`, `glob`, gives it, each where
/// rustc resolves it there, past what the module's own items and single
/// `use`s shadow (rd153): a function, constant or thread-local of the
/// crate's, from its module, or a library's, from its file, by what it's
/// exported as; a type, a trait or a macro, nothing. What has JS but can't
/// be said, a binding, a class or a module, is refused, never left out.
fn globbed(
    tcx: TyCtxt<'_>,
    (module, glob, statement): (LocalModId, DefId, rustc_span::Span),
    (fns, foreign): (&HashMap<DefId, FnInfo>, &super::library::Foreign<'_, '_>),
    paths: &HashMap<LocalModId, Vec<String>>,
    found: &mut Vec<(rustc_span::Span, js::ImportFrom, (String, String))>,
) {
    use rustc_middle::metadata::Reexport as Via;
    let refuse = |what: String, why: &str| {
        super::refusal::refuse(tcx, statement, super::refusal::Refused::Because(&what, why));
    };
    for child in tcx.module_children_local(module.to_local_def_id()) {
        if !matches!(child.reexport_chain.first(), Some(Via::Glob(used)) if *used == glob) {
            continue;
        }
        let Res::Def(kind, id) = child.res else { continue };
        let name = child.ident.to_string();
        match kind {
            DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. } => {
                if let Some(info) = fns
                    .get(&id)
                    .filter(|info| info.owner.is_none() && info.module != module)
                    && let Some(from) = paths.get(&info.module)
                {
                    let export = super::default_exports::exported_as(tcx, id, info.exported());
                    let alias = match child.ident.name == tcx.item_name(id) {
                        true => info.exported().to_string(),
                        false => super::bindings::alias_name(tcx, glob, child.ident.name),
                    };
                    found.push((statement, js::ImportFrom::Module(from.clone()), (export, alias)));
                } else if let Some(imported) = foreign.item(id)
                    && imported.member.is_none()
                    && let Some((from, export)) = foreign.import_key(id, module)
                {
                    found.push((
                        statement,
                        js::ImportFrom::Specifier(from),
                        (export, imported.name.clone()),
                    ));
                } else if fns.get(&id).is_some_and(|info| info.module == module) {
                    // Its own, through a module that re-exports it: written already.
                } else {
                    refuse(
                        format!("a `pub use` of every name of a module, of `{name}`"),
                        "`pub use` it by its name, or each of the others",
                    );
                }
            }
            DefKind::Mod | DefKind::Struct if matches!(kind, DefKind::Mod) || super::classes::is_class(tcx, id) => {
                refuse(
                    format!("a `pub use` of every name of a module, of its module or class `{name}`"),
                    "`pub use` each of its names that has JS by its name",
                );
            }
            // A type, a trait or a macro: no JS (ADR 0240).
            _ => {}
        }
    }
}

/// What a `use`'s path names in its value namespace.
fn named_of(path: &hir::UsePath<'_>) -> DefId {
    match path.res.value_ns.or(path.res.type_ns) {
        Some(Res::Def(_, named)) => named,
        _ => CRATE_MOD_ID.to_def_id(),
    }
}

/// The last name a `use`'s path writes, what the module it names calls
/// its item.
fn last_name(path: &hir::UsePath<'_>) -> Option<Symbol> {
    path.segments.last().map(|segment| segment.ident.name)
}

/// The crate's module a `use`'s path names its item through, `server` of
/// `crate::server::cache`: as written, which may re-export it (ADR 0240,
/// amended).
fn written_module(path: &hir::UsePath<'_>) -> Option<LocalModId> {
    let [.., module, _] = path.segments else { return None };
    // `crate::`, `self::` and `super::` name a module too.
    match module.res {
        Res::Def(DefKind::Mod, module) => Some(LocalModId::new_unchecked(module.as_local()?)),
        _ => None,
    }
}

/// What the crate's `module` exports `item` as where a path names it
/// there by `name`: its own item's name, or its default or alias
/// (`default_exports::exported_as`); else the name a `pub use` of it there
/// gives it, which re-exports it. `None` where it does neither.
pub(super) fn exported_in(
    tcx: TyCtxt<'_>,
    fns: &HashMap<DefId, FnInfo>,
    module: LocalModId,
    name: Symbol,
    item: DefId,
) -> Option<String> {
    let info = fns.get(&item)?;
    if info.module == module {
        return match name == tcx.item_name(item) {
            true => Some(super::default_exports::exported_as(tcx, item, info.exported())),
            false => super::default_exports::alias_named(tcx, item, name),
        };
    }
    let used = reexporting_use(tcx, module, name, item)?;
    Some(match name == tcx.item_name(item) {
        true => info.name.clone(),
        false => super::bindings::alias_name(tcx, used, name),
    })
}

/// The `pub use` of `module` that re-exports `item` by `name`, which its
/// file's JS re-exports (`analysis::is_reexport`), as `pub use
/// crate::props::register_simple as register_events;` does: the `use`
/// item.
pub(super) fn reexporting_use(tcx: TyCtxt<'_>, module: LocalModId, name: Symbol, item: DefId) -> Option<DefId> {
    tcx.hir_module_items(module).free_items().find_map(|id| {
        let used = tcx.hir_item(id);
        let hir::ItemKind::Use(path, hir::UseKind::Single(ident)) = used.kind else {
            return None;
        };
        let names_it = matches!(path.res.value_ns, Some(Res::Def(_, named)) if named == item);
        (names_it && ident.name == name && super::analysis::is_reexport(tcx, used)).then(|| used.owner_id.to_def_id())
    })
}

/// Whether each item `module` holds but its `mod`s and its own imports, a
/// `use` not `pub` or an `extern crate`, is a `pub use` its file re-exports (`is_reexport`):
/// a crate's root whose only job is re-exporting, which writes no JS of its
/// own where its package's entry re-exports each from the same module
/// (the user's decision, ADR 1008, amended).
pub(super) fn reexports_alone(tcx: TyCtxt<'_>, module: LocalModId) -> bool {
    tcx.hir_module_free_items(module).all(|id| {
        let item = tcx.hir_item(id);
        match item.kind {
            // Its `mod`s, and std's prelude rustc gives every crate.
            hir::ItemKind::Mod(..) | hir::ItemKind::ExternCrate(..) => true,
            hir::ItemKind::Use(..) if !tcx.visibility(item.owner_id).is_public() => true,
            _ => super::analysis::is_reexport(tcx, item),
        }
    })
}

/// Of `module`'s namespace's methods, each that's the function it passes
/// its parameters on to (`forwarded`, rd292b, ADR 1037 amended), that
/// function by the method's name, `export { batchedUpdates } from
/// "./ReactFiberWorkLoop.js"`: a run of them of one impl, from one module,
/// nothing else between, one statement, as a `pub use` of a list is.
fn forwarded_methods(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    (fns, forwarded): (&HashMap<DefId, FnInfo>, &HashMap<DefId, DefId>),
    paths: &HashMap<LocalModId, Vec<String>>,
    found: &mut Vec<(rustc_span::Span, js::ImportFrom, (String, String))>,
) {
    let mut imps: Vec<DefId> = (forwarded.keys())
        .filter(|method| fns.get(method).is_some_and(|info| info.module == module))
        .map(|&method| tcx.parent(method))
        .collect();
    imps.sort_by_key(|&imp| tcx.def_span(imp).lo());
    imps.dedup();
    for imp in imps {
        // Each run, its methods and where each is from.
        let mut runs: Vec<Vec<(DefId, LocalModId)>> = Vec::new();
        let mut last: Option<LocalModId> = None;
        for &method in tcx.associated_item_def_ids(imp) {
            let from = forwarded.get(&method).and_then(|target| Some(fns.get(target)?.module));
            match (from, runs.last_mut()) {
                (Some(from), Some(run)) if last == Some(from) => run.push((method, from)),
                (Some(from), _) => runs.push(vec![(method, from)]),
                (None, _) => {}
            }
            last = from;
        }
        for run in runs {
            let span = |method: DefId| tcx.hir_span_with_body(tcx.local_def_id_to_hir_id(method.expect_local()));
            let statement = span(run[0].0).to(span(run[run.len() - 1].0));
            for (method, from) in run {
                let target = forwarded[&method];
                let (Some(path), Some(theirs), Some(mine)) = (paths.get(&from), fns.get(&target), fns.get(&method))
                else {
                    continue;
                };
                let export = super::default_exports::exported_as(tcx, target, theirs.exported());
                found.push((
                    statement,
                    js::ImportFrom::Module(path.clone()),
                    (export, mine.exported().to_string()),
                ));
            }
        }
    }
}

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

/// One type a module's declarations alone re-export (`type_reexports`):
/// its statement, where it's from, none where it's the module's own, and
/// `(export, alias)`.
pub(super) type TypeReexport = (rustc_span::Span, Option<js::ImportFrom>, (String, String));

/// One name a module re-exports, where it's written: the statement, which
/// its names of one module share; where it's from; and `(export, alias)`,
/// what that exports it as and the name it's re-exported by.
pub(super) type Named = (Written, js::ImportFrom, (String, String));

/// Where one name a module re-exports is written: the statement, which its
/// names of one module share and its comments are before, and its `Leaf`.
pub(super) type Written = (rustc_span::Span, Leaf);

/// The leaf of a `use` that names what's re-exported, whose comments are
/// its own, `act, // DEV-only` (rd338): where it writes the name its path
/// gives and the name it re-exports it by, `act` and `act`, `a` and `b` of
/// `a as b`; `NO_LEAF` of none, a glob's or a forwarder's.
pub(super) type Leaf = (rustc_span::Span, rustc_span::Span);

const NO_LEAF: Leaf = (rustc_span::DUMMY_SP, rustc_span::DUMMY_SP);

/// What `module` re-exports, each name of each `pub use` statement's
/// `export { .. } from`, which `assembled` orders, by where it's from (ADR
/// 0240, amended):
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
) -> Vec<Named> {
    // Each name, where it's written: the statement, its `use` item's span,
    // which each of a tree's leaves shares; and where it's from.
    let mut found: Vec<Named> = Vec::new();
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
            found.push(((at, NO_LEAF), from.clone(), (import.clone(), held.name.clone())));
        }
        if held.default {
            found.push(((at, NO_LEAF), from, (import.clone(), "default".to_string())));
        }
    }
    if let Some((function, span)) = default_from
        && let Some(info) = fns.get(&function)
        && let Some(from) = paths.get(&info.module)
    {
        let export = super::default_exports::exported_as(tcx, function, info.exported());
        found.push((
            (span, NO_LEAF),
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
        let statement = statement_of(item);
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
        // Its leaf, whose comments are its own (rd338).
        let name = path.segments.last().map_or(ident.span, |segment| segment.ident.span);
        let written = (statement, (name, ident.span));
        let alias = |name: &str| {
            let named = (ident.name, named_of(path));
            super::bindings::use_name(tcx, item.owner_id.to_def_id(), named, || name.to_string())
        };
        // Its name where it's re-exported through a module that re-exports
        // it as `export`: that, where the `use` keeps the name the path
        // gives, but its `#[rust_js::name]` (rd338).
        let through_alias = |export: &str| match Some(ident.name) == last_name(path) {
            true => super::bindings::given_name(tcx, item.owner_id.to_def_id()).unwrap_or_else(|| export.to_string()),
            false => super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),
        };
        // Its value: a function, a constant, a thread-local or a binding.
        if let Some(Res::Def(kind, def_id)) = path.res.value_ns
            && matches!(kind, DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. })
        {
            // A binding of a JS module's export, itself, not a path in it.
            if super::bindings::is_binding(tcx, def_id) {
                if let Some(path) = super::bindings::js_path(tcx, def_id)
                    && let Some(((from, export), "")) = super::bindings::js_import(&path)
                {
                    // Its namespace too, `export * as ns from` (ADR 0240, amended).
                    let alias = super::bindings::use_name(tcx, item.owner_id.to_def_id(), (ident.name, def_id), || {
                        ident.to_string()
                    });
                    found.push((written, js::ImportFrom::Specifier(from), (export, alias)));
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
                    Some((through, export)) => (through, export.clone(), through_alias(&export)),
                    None => (
                        info.module,
                        super::default_exports::exported_as(tcx, def_id, info.exported()),
                        alias(info.exported()),
                    ),
                };
                let Some(from) = paths.get(&from) else { continue };
                found.push((written, js::ImportFrom::Module(from.clone()), (export, alias)));
                continue;
            }
            // A library's: through the crate's module its path names that
            // re-exports it, from there, by its name there (rd338); else
            // from its file, as its manifest says (ADR 0100).
            if let Some(imported) = foreign.item(def_id)
                && imported.member.is_none()
            {
                if let Some((through, export)) = through_reexport(tcx, path, def_id, || imported.name.clone())
                    && let Some(from) = paths.get(&through)
                {
                    let alias = through_alias(&export);
                    found.push((written, js::ImportFrom::Module(from.clone()), (export, alias)));
                    continue;
                }
                if let Some((from, export)) = foreign.import_key(def_id, module) {
                    let from = js::ImportFrom::Specifier(from);
                    found.push((written, from, (export, alias(&imported.name))));
                    continue;
                }
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
                // Through a module that re-exports it, that module's export
                // (rd338).
                let own = || super::camel_case(ident.as_str());
                if let Some((through, export)) = through_reexport(tcx, path, named, own)
                    && let Some(from) = paths.get(&through)
                {
                    let alias = through_alias(&export);
                    found.push((written, js::ImportFrom::Module(from.clone()), (export, alias)));
                    continue;
                }
                let from = (named.as_local()).and_then(|local| paths.get(&LocalModId::new_unchecked(local)));
                match from {
                    Some(from) => {
                        let alias = super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name);
                        found.push((written, js::ImportFrom::Module(from.clone()), ("*".to_string(), alias)));
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
                // Through a module that re-exports it, from there (rd338).
                if let Some((through, named)) = through_reexport(tcx, path, class, || export.clone())
                    && let Some(through) = paths.get(&through)
                {
                    let alias = through_alias(&named);
                    found.push((written, js::ImportFrom::Module(through.clone()), (named, alias)));
                    continue;
                }
                let alias =
                    super::bindings::use_name(tcx, item.owner_id.to_def_id(), (ident.name, class), || export.clone());
                found.push((written, from, (export, alias)));
            }
            // A type, a trait or a macro: no JS (ADR 0240).
            _ => {}
        }
    }
    forwarded_methods(tcx, module, (fns, forwarded), paths, &mut found);
    found
}

/// What `module`'s `pub use`s of types re-export in its declarations
/// alone, `export type { TemporaryReferenceSet } from "..";` (rd642): each
/// type another module's declarations export by its name
/// (`declarations::declares_type`), the crate's from its module's file, a
/// library's from its file (`Foreign::declared_type`), by the name the
/// `use` gives it; the module's own by another name from none, `export
/// type { Size as Measure };`. A type has no JS to re-export (ADR 0240); a
/// class is its constructor, which `reexports` re-exports.
pub(super) fn type_reexports(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    foreign: &super::library::Foreign<'_, '_>,
    paths: &HashMap<LocalModId, Vec<String>>,
) -> Vec<TypeReexport> {
    let mut found = Vec::new();
    for id in tcx.hir_module_items(module).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Use(path, hir::UseKind::Single(ident)) = item.kind else {
            continue;
        };
        let Some(Res::Def(DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::TyAlias, named)) =
            path.res.type_ns
        else {
            continue;
        };
        if !tcx.visibility(item.owner_id).is_public() || super::classes::is_class(tcx, named) {
            continue;
        }
        let (from, export) = match named.as_local() {
            Some(local) => {
                let home = tcx.parent_module_from_def_id(local);
                let declared = tcx.visibility(named).is_public() && super::declarations::declares_type(tcx, named);
                match paths.get(&home) {
                    Some(_) if declared && home == module => (None, tcx.item_name(named).to_string()),
                    Some(from) if declared => (
                        Some(js::ImportFrom::Module(from.clone())),
                        tcx.item_name(named).to_string(),
                    ),
                    _ => continue,
                }
            }
            None => match foreign.declared_type(named) {
                Some((from, name, default)) => (
                    Some(js::ImportFrom::Specifier(from.clone())),
                    if *default { "default".to_string() } else { name.clone() },
                ),
                None => continue,
            },
        };
        found.push((statement_of(item), from, (export, ident.to_string())));
    }
    found
}

/// A `pub use`'s whole statement, from its `pub`, which each of a tree's
/// leaves shares, its comments before it.
fn statement_of(item: &hir::Item<'_>) -> rustc_span::Span {
    match item.vis_span.is_empty() {
        true => item.span,
        false => item.vis_span.to(item.span),
    }
}

/// The `export { .. } from`s of what a module re-exports, `found`
/// (`reexports`, `forwarded_functions`): in the order they're written, a
/// statement's names from one module one statement, in their order, and a
/// run of them from one module, nothing but attributes between, one too.
pub(super) fn assembled(mut found: Vec<Named>, sources: &super::sources::CapturedSources) -> Vec<Reexport> {
    // In the order they're written; a statement's names from one module
    // one `export { .. } from` of it, in their order.
    found.sort_by_key(|((at, _), ..)| at.lo());
    let at = |span: rustc_span::Span| match span.is_dummy() {
        true => js::Span::NONE,
        false => sources.span(span),
    };
    let leaf = |(name, alias): Leaf| (at(name), at(alias));
    let mut reexports: Vec<(rustc_span::Span, Reexport)> = Vec::new();
    for ((at, written), from, named) in found {
        match reexports
            .iter_mut()
            .find(|(was, reexport)| was.lo() == at.lo() && reexport.from == from)
        {
            Some((_, reexport)) => {
                reexport.named.push(named);
                reexport.written.push(leaf(written));
            }
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
                        written: vec![leaf(written)],
                        span,
                        at: span,
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
                    .is_some_and(|between| only_attributes(between, false)) =>
            {
                last.named.extend(reexport.named);
                last.written.extend(reexport.written);
                last.span.hi = reexport.span.hi;
                last.at.hi = reexport.span.hi;
            }
            _ => merged.push(reexport),
        }
    }
    merged
}

/// Whether `between`, the text between two items, is only whitespace and
/// attributes, `#[cfg_attr(..)]`, with no comment, and no blank line but
/// where `blank_lines`.
pub(in crate::lower) fn only_attributes(between: &str, blank_lines: bool) -> bool {
    let mut rest = between;
    let mut blank = false;
    loop {
        let trimmed = rest.trim_start();
        let space = &rest[..rest.len() - trimmed.len()];
        blank |= space.matches('\n').count() > 1;
        if trimmed.is_empty() {
            return blank_lines || !blank;
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
    found: &mut Vec<Named>,
) {
    use rustc_middle::metadata::Reexport as Via;
    // A glob writes no name's leaf.
    let written = (statement, NO_LEAF);
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
                    found.push((written, js::ImportFrom::Module(from.clone()), (export, alias)));
                } else if let Some(imported) = foreign.item(id)
                    && imported.member.is_none()
                    && let Some((from, export)) = foreign.import_key(id, module)
                {
                    found.push((
                        written,
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
    Some(super::bindings::use_name(tcx, used, (name, item), || info.name.clone()))
}

/// The crate's module a `pub use`'s `path` names `item` through, whose
/// own `pub use` re-exports it by the name the path gives, and what it
/// re-exports it as: that `use`'s `#[rust_js::name]`, else, where it keeps
/// the item's name, `own`, the item's JS name, else its alias camelCased
/// (`bindings::use_name`). React's experimental entry's `pub use
/// crate::react_client::FRAGMENT;` of ReactClient's `#[rust_js::name =
/// "Fragment"] pub use shared::react_symbols::REACT_FRAGMENT_TYPE as
/// FRAGMENT;` is `Fragment` of ReactClient's, as upstream's `export
/// {Fragment} from './src/ReactClient'` (rd338, ADR 0874, amended). A
/// library's item, a class or a module alike; the crate's function is
/// `exported_in`'s, which also names one by its own module's alias.
fn through_reexport(
    tcx: TyCtxt<'_>,
    path: &hir::UsePath<'_>,
    item: DefId,
    own: impl FnOnce() -> String,
) -> Option<(LocalModId, String)> {
    let through = written_module(path)?;
    let name = last_name(path)?;
    let used = reexporting_use(tcx, through, name, item)?;
    Some((through, super::bindings::use_name(tcx, used, (name, item), own)))
}

/// Where a module imports an item from: the crate's module its path names
/// it through, which re-exports it, by what that module exports it as.
pub(super) struct Through {
    pub(super) module: LocalModId,
    pub(super) export: String,
    /// The path that names it so, the first written.
    pub(super) at: rustc_span::Span,
}

/// Each item a module of the crate's `use` names through another of its
/// modules whose `pub use` re-exports it by the name the path gives, by the
/// module and the item: the crate's function or constant, or a library's,
/// imported from that module, as upstream's react-markup imports
/// `createResumableState` from `./ReactFizzConfigMarkup`, which re-exports
/// it, not from ReactFizzConfigDOM (rd650, ADR 0874, amended). Of two
/// `use`s of one item, the first written, one through its home too; a path
/// of a body, `config::create_state()`, reads its module's namespace (ADR
/// 1018), no import of the item. Whether the import is taken from there is
/// the load order's to say (`load_order::ModuleImports::imports_without_cycle`),
/// as one that makes a load cycle stays its home's.
pub(super) fn imported_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    (fns, foreign): (&HashMap<DefId, FnInfo>, &super::library::Foreign<'_, 'tcx>),
) -> HashMap<(LocalModId, DefId), Through> {
    // What a `use`'s path names `item` through, `module`, re-exports it as,
    // by `name`: of the crate's function, by `exported_in`'s name; of a
    // library's, by its `pub use`'s. None of its home, or of one that
    // doesn't re-export it.
    let reexported_as = |module: LocalModId, name: Symbol, item: DefId| match fns.get(&item) {
        Some(info) if info.owner.is_none() && info.module != module => exported_in(tcx, fns, module, name, item),
        Some(_) => None,
        None => (foreign.item(item))
            .filter(|imported| imported.member.is_none())
            .and_then(|imported| {
                let used = reexporting_use(tcx, module, name, item)?;
                Some(super::bindings::use_name(tcx, used, (name, item), || {
                    imported.name.clone()
                }))
            }),
    };
    // Each `use`'s first naming of each item, where, and through the module
    // that re-exports it, by what, or not.
    type First = (rustc_span::Span, Option<(LocalModId, String)>);
    let mut first: HashMap<(LocalModId, DefId), First> = HashMap::new();
    for id in tcx.hir_crate_items(()).free_items() {
        let used = tcx.hir_item(id);
        let hir::ItemKind::Use(path, hir::UseKind::Single(_)) = used.kind else {
            continue;
        };
        let scope = tcx.parent_module_from_def_id(used.owner_id.def_id);
        for res in path.res.present_items() {
            let Res::Def(_, item) = res else { continue };
            let through = match path.segments {
                [.., parent, last] => match parent.res {
                    Res::Def(DefKind::Mod, module) => (module.as_local().map(LocalModId::new_unchecked))
                        .filter(|&module| module != scope)
                        .and_then(|module| Some((module, reexported_as(module, last.ident.name, item)?))),
                    _ => None,
                },
                _ => None,
            };
            let at = path.span;
            match first.entry((scope, item)) {
                std::collections::hash_map::Entry::Occupied(mut was) => {
                    if at.lo() < was.get().0.lo() {
                        was.insert((at, through));
                    }
                }
                std::collections::hash_map::Entry::Vacant(entry) => _ = entry.insert((at, through)),
            }
        }
    }
    (first.into_iter())
        .filter_map(|(key, (at, through))| {
            let (module, export) = through?;
            Some((key, Through { module, export, at }))
        })
        .collect()
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
        // Of a value, or of a class or module, the type namespace's.
        let names = |res: Option<Res>| matches!(res, Some(Res::Def(_, named)) if named == item);
        let names_it = names(path.res.value_ns) || names(path.res.type_ns);
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
    found: &mut Vec<Named>,
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
            // Not one of its own module's function, which that exports
            // by the method's name (#566).
            let from = (forwarded.get(&method).and_then(|target| Some(fns.get(target)?.module)))
                .filter(|&from| from != module);
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
                let (from, named) = (
                    js::ImportFrom::Module(path.clone()),
                    (export, mine.exported().to_string()),
                );
                // A `pub use` of the function by its name, React's
                // ReactFiberReconciler's `pub use
                // crate::react_fiber_error_logger::default_on_caught_error;`
                // beside its namespace's method: one re-export of it (rd424).
                match found.iter().find(|(_, _, (_, alias))| *alias == named.1) {
                    Some((_, was, already)) if *was == from && *already == named => continue,
                    Some(((written, _), ..)) => {
                        let message = format!(
                            "rust-js: `{}` is a `pub use` of another item, and this method is its namespace's \
                             re-export of `{}` by that name: JS exports one value by a name",
                            named.1,
                            tcx.def_path_str(target)
                        );
                        tcx.dcx()
                            .struct_span_err(span(method), message)
                            .with_span_note(*written, "the `pub use`")
                            .emit();
                        continue;
                    }
                    None => found.push(((statement, NO_LEAF), from, named)),
                }
            }
        }
    }
}

/// Of `module`'s functions, `forwarders`, each that's the function it
/// passes its parameters on to at one instantiation, and that function
/// (rd575, ADR 1064): that function by the forwarder's name, `export {
/// decodeAction } from "react-server/src/ReactFlightActionServer"`, `export
/// { decode as decodeReply }` by another, the crate's from its module, a
/// library's from its file, as a `pub use` of it is (ADR 0874). A run of
/// them from one module, nothing but whitespace and attributes between,
/// one statement, as a namespace's forwarded methods are (rd292b).
pub(super) fn forwarded_functions(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    forwarders: &HashMap<DefId, DefId>,
    (fns, foreign): (&HashMap<DefId, FnInfo>, &super::library::Foreign<'_, '_>),
    paths: &HashMap<LocalModId, Vec<String>>,
    sources: &super::sources::CapturedSources,
) -> Vec<Named> {
    // Each, in the order written, where it's from and what it's exported as.
    let mut each: Vec<(rustc_span::Span, js::ImportFrom, (String, String))> = Vec::new();
    for id in tcx.hir_module_free_items(module) {
        let function = id.owner_id.to_def_id();
        let (Some(&target), Some(mine)) = (forwarders.get(&function), fns.get(&function)) else {
            continue;
        };
        let Some((from, export)) = forwarded_from(tcx, module, target, (fns, foreign), paths) else {
            continue;
        };
        let span = tcx.hir_span_with_body(tcx.local_def_id_to_hir_id(id.owner_id.def_id));
        each.push((span, from, (export, mine.exported().to_string())));
    }
    // Each run's statement, its first's to its last's.
    let text = &sources.output.text;
    let mut found: Vec<Named> = Vec::new();
    let mut run_start = 0;
    for at in 0..each.len() {
        let next = each.get(at + 1);
        let joined = next.is_some_and(|(next, from, _)| {
            let (this, next) = (sources.span(each[at].0), sources.span(*next));
            *from == each[at].1
                && (text.get(this.hi as usize..next.lo as usize)).is_some_and(|between| only_attributes(between, true))
        });
        if joined {
            continue;
        }
        let statement = each[run_start].0.to(each[at].0);
        for (_, from, named) in &each[run_start..=at] {
            found.push(((statement, NO_LEAF), from.clone(), named.clone()));
        }
        run_start = at + 1;
    }
    found
}

/// Where `module` re-exports `target`, a forwarder's function, from, and
/// what that exports it as (`forwarded_functions`): the crate's from its
/// module, a library's from its file; `None` where it's neither.
pub(super) fn forwarded_from(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    target: DefId,
    (fns, foreign): (&HashMap<DefId, FnInfo>, &super::library::Foreign<'_, '_>),
    paths: &HashMap<LocalModId, Vec<String>>,
) -> Option<(js::ImportFrom, String)> {
    match fns.get(&target) {
        Some(theirs) if target.is_local() => {
            let from = paths.get(&theirs.module)?;
            let export = super::default_exports::exported_as(tcx, target, theirs.exported());
            Some((js::ImportFrom::Module(from.clone()), export))
        }
        _ => {
            let (from, export) = foreign.import_key(target, module)?;
            Some((js::ImportFrom::Specifier(from), export))
        }
    }
}

/// Where each of `reexports`, `module`'s, is among its imports, as JS
/// loads them, where the source first names what it's from, as an import
/// is (ADR 1063, amended, rd620): a `pub use`'s where it's written; one a
/// function makes, a namespace's forwarding method's `export { createPortal
/// } from`, or a forwarder's (rd575), at the `use` of its module or of
/// what's in it, the reconciler's `use crate::react_portal;`, where one is
/// before it; `written`, a source span's place in the module's JS.
pub(super) fn place(
    tcx: TyCtxt<'_>,
    reexports: &mut [Reexport],
    uses: &super::analysis::UsesWritten,
    (fns, foreign, paths): (
        &HashMap<DefId, FnInfo>,
        &super::library::Foreign<'_, '_>,
        &HashMap<LocalModId, Vec<String>>,
    ),
    written: &dyn Fn(Option<rustc_span::Span>) -> js::Span,
) {
    let statements: Vec<js::Span> = uses.statements().map(|at| written(Some(at))).collect();
    for reexport in reexports {
        let span = reexport.span;
        if span.is_none() || statements.iter().any(|at| at.lo < span.hi && span.lo < at.hi) {
            continue;
        }
        let named = match &reexport.from {
            js::ImportFrom::Module(path) => (paths.iter())
                .find(|(_, of)| *of == path)
                .and_then(|(&target, _)| uses.module_at(tcx, fns, target)),
            js::ImportFrom::Specifier(from) => uses.package_at(tcx, foreign, from),
        };
        let named = written(named);
        if !named.is_none() && named.lo < reexport.at.lo {
            reexport.at = named;
        }
    }
}

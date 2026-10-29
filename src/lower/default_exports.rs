//! A module's default export, `js::export_default!` (ADRs 0192, 0611,
//! 0625): what each of the macro's forms makes the default; each `pub use`
//! of a module's own item by another name (`own_aliases`, ADR 0596); and
//! what a module exports an item of its own as, which every importer asks
//! (`exported_as`): a module of its crate, another crate through its
//! library's manifest, a `pub use` re-exporting it, a `js::import!` of it.

use rustc_hir::def::{DefKind, Res};
use rustc_hir::{ExprKind, ItemKind, Stmt, StmtKind, UseKind};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};
use rustc_span::{Span, Symbol, kw};

/// Whether `function`, its module's default export, is declared as it,
/// `export default function noop() {}`: of `js::export_default! { fn noop()
/// {} }`, which marks the function itself, not a `const _` naming it (ADR
/// 0611).
pub(super) fn default_declared(tcx: TyCtxt<'_>, function: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("export_default")];
    held_by_default(tcx, function) && tcx.get_attrs_by_path(function, &path).next().is_some()
}

/// Whether `def`, marked `#[rust_js::export_default]`, is the default
/// itself, as `js::export_default! { .. }` writes it: a function, or a
/// `const` whose value is the default (ADR 0625), not the `const _` the
/// naming form makes.
pub(super) fn held_by_default(tcx: TyCtxt<'_>, def: DefId) -> bool {
    match tcx.def_kind(def) {
        DefKind::Fn => true,
        DefKind::Const { .. } => tcx.item_name(def) != kw::Underscore,
        _ => false,
    }
}

/// What `item`, a function or constant of the crate's, is exported as by
/// its module, which every importer imports it by: `default`, where its
/// module's `js::export_default!` makes it the default, of either form,
/// `js::export_default! { pub fn noop() {} }` or `js::export_default!(page);`;
/// else the name its module's first `pub use` of it by another name gives
/// it, `pub use self::get_current_time as unstable_now;`'s `unstable_now`,
/// as its module exports it by that alone (`own_aliases`); else `name`, its
/// name there (ADRs 0251, 0596, 0611, 0773).
pub(super) fn exported_as(tcx: TyCtxt<'_>, item: DefId, name: &str) -> String {
    if is_own_default(tcx, item) {
        return "default".to_string();
    }
    match alias_of(tcx, item) {
        Some(alias) => alias.exported,
        None => name.to_string(),
    }
}

/// A `pub use` of its module's own function or constant by another name,
/// `pub use self::get_current_time as unstable_now;` (ADR 0596).
pub(super) struct Alias {
    /// The item it names.
    pub(super) item: DefId,
    /// The name it gives the item in Rust, which a path through it ends in.
    pub(super) name: Symbol,
    /// What the module exports the item by: the `use`'s `#[rust_js::name]`,
    /// `unstable_now`, or `name` camelCase (ADR 0573).
    pub(super) exported: String,
}

/// Each `pub use` of `module`'s own function or constant by another name,
/// in the order they're written, which its JS's `export { .. }` lists them
/// in. The module exports the item by these alone, not by its own name,
/// which Rust can't say: a `pub use` of a private item is E0364 (ADR 0596,
/// amended). Not one of a binding, which is a JS module's re-export (ADR
/// 0240).
pub(super) fn own_aliases(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<Alias> {
    (tcx.hir_module_free_items(module))
        .filter_map(|id| {
            let item = tcx.hir_item(id);
            let ItemKind::Use(path, UseKind::Single(ident)) = item.kind else {
                return None;
            };
            let Some(Res::Def(_, named)) = path.res.value_ns else {
                return None;
            };
            let own = (named.as_local()).is_some_and(|local| tcx.parent_module_from_def_id(local) == module);
            let aliased = own && super::analysis::is_reexport(tcx, item) && !super::bindings::is_binding(tcx, named);
            aliased.then(|| Alias {
                item: named,
                name: ident.name,
                exported: super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),
            })
        })
        .collect()
}

/// `item`'s first alias in its own module (`own_aliases`).
fn alias_of(tcx: TyCtxt<'_>, item: DefId) -> Option<Alias> {
    let module = tcx.parent_module_from_def_id(item.as_local()?);
    own_aliases(tcx, module).into_iter().find(|alias| alias.item == item)
}

/// What `item`'s module exports it by where its `pub use` names it `name`
/// in Rust, `unstable_now`, which a `use` of that name imports it as
/// (`own_aliases`).
pub(super) fn alias_named(tcx: TyCtxt<'_>, item: DefId, name: Symbol) -> Option<String> {
    let module = tcx.parent_module_from_def_id(item.as_local()?);
    (own_aliases(tcx, module).into_iter())
        .find(|alias| alias.item == item && alias.name == name)
        .map(|alias| alias.exported)
}

/// Whether `item` is its own module's default export (`exported_as`): what
/// that module's first `js::export_default!` makes the default, as a second
/// is refused where the module is made. Not another module's function a
/// module re-exports as its default, nor a binding it holds (ADR 0773's
/// `HeldBinding`), which aren't that module's items.
fn is_own_default(tcx: TyCtxt<'_>, item: DefId) -> bool {
    (item.as_local()).is_some_and(|local| own_or_named(tcx, tcx.parent_module_from_def_id(local)) == Some(item))
}

/// Each module's default export, by the module it's the default of: what
/// JS imports and calls with any arguments, so no call of the crate's says
/// what it's given (`only_called`); and, of another module's function,
/// what that module exports for the default's `export { twice as default }
/// from` to read (`exported_across_modules`), whoever else calls it.
pub(super) fn defaults(tcx: TyCtxt<'_>) -> Vec<(LocalModId, DefId)> {
    let mut modules = Vec::new();
    tcx.hir_for_each_module(|module| modules.push(module));
    (modules.into_iter())
        .filter_map(|module| Some((module, own_or_named(tcx, module)?)))
        .collect()
}

/// What `module`'s first `js::export_default!` makes its default, its own
/// item or another module's.
fn own_or_named(tcx: TyCtxt<'_>, module: LocalModId) -> Option<DefId> {
    default_exports(tcx, module).into_iter().next()?.0
}

/// Each of a module's `js::export_default!`s, in its order: what it makes
/// the default, the function or constant it holds (ADRs 0611, 0625), or
/// what its `const _` names, `let _ = page;` (ADR 0192); and where it's
/// written.
pub(super) fn default_exports(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<(Option<DefId>, Span)> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("export_default")];
    tcx.hir_module_free_items(module)
        .filter(|item| tcx.get_attrs_by_path(item.owner_id.to_def_id(), &path).next().is_some())
        .map(|item| {
            let def = item.owner_id.def_id;
            // Of the function itself (ADR 0611), or the constant (ADR 0625).
            if held_by_default(tcx, def.to_def_id()) {
                return (Some(def.to_def_id()), tcx.def_span(def));
            }
            // `use page as _;`: a generic function's too, which Rust can't
            // name as a value unless it's given its types.
            let named = match tcx.hir_body_owned_by(def).value.kind {
                ExprKind::Block(block, _) => match block.stmts {
                    [
                        Stmt {
                            kind: StmtKind::Item(item),
                            ..
                        },
                    ] => match tcx.hir_item(*item).kind {
                        // A function, or a `thread_local!`'s const, a memo'd
                        // component's (ADR 0248), or a binding's static (ADR 0633).
                        // Or a JS class, its constructor (ADR 0761), `export
                        // default class` in JS.
                        ItemKind::Use(path, _) => match (path.res.value_ns, path.res.type_ns) {
                            (
                                Some(Res::Def(DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }, function)),
                                _,
                            ) => Some(function),
                            (_, Some(Res::Def(DefKind::Struct, class))) if super::classes::is_class(tcx, class) => {
                                Some(class)
                            }
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            (named, tcx.def_span(def))
        })
        .collect()
}

/// What a module's `js::export_default!` makes its default: its own
/// function or thread-local, `export`, declared as the default or not,
/// `declared`; or another module's function, `from`, re-exported as it
/// (ADR 0240). A binding's is its own `const` (ADRs 0622, 0633), unless it
/// names one held through another module, re-exported. Any other is
/// refused.
#[derive(Default)]
pub(super) struct ModuleDefault {
    pub(super) export: Option<DefId>,
    pub(super) declared: bool,
    pub(super) from: Option<(DefId, Span)>,
}

pub(super) fn module_default(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    fns: &std::collections::HashMap<DefId, super::FnInfo>,
    module_held: &[super::bindings::HeldBinding],
) -> ModuleDefault {
    let mut default = ModuleDefault::default();
    for (function, span) in default_exports(tcx, module) {
        let unset = default.export.is_none() && default.from.is_none();
        match function {
            Some(function)
                if unset
                    && function
                        .as_local()
                        .is_some_and(|local| tcx.parent_module_from_def_id(local) == module) =>
            {
                default.export = Some(function);
                default.declared = default_declared(tcx, function);
            }
            Some(function) if unset && fns.get(&function).is_some_and(|info| info.owner.is_none()) => {
                default.from = Some((function, span));
            }
            Some(function) if unset && super::bindings::held_path(tcx, function).is_some() => {
                let reexported =
                    (module_held.iter()).any(|held| held.binding == function && held.default && held.through.is_some());
                if !reexported {
                    default.export = Some(function);
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
    default
}

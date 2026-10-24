//! A module's default export, `js::export_default!` (ADRs 0192, 0611,
//! 0625): what each of the macro's forms makes the default, and what a
//! module exports an item of its own as, which every importer asks
//! (`exported_as`): a module of its crate, another crate through its
//! library's manifest, a `pub use` re-exporting it, a `js::import!` of it.

use rustc_hir::def::{DefKind, Res};
use rustc_hir::{ExprKind, ItemKind, Stmt, StmtKind};
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
/// else `name`, its name there (ADRs 0251, 0611, 0773).
pub(super) fn exported_as(tcx: TyCtxt<'_>, item: DefId, name: &str) -> String {
    match is_own_default(tcx, item) {
        true => "default".to_string(),
        false => name.to_string(),
    }
}

/// Whether `item` is its own module's default export (`exported_as`).
pub(super) fn is_own_default(tcx: TyCtxt<'_>, item: DefId) -> bool {
    (item.as_local()).is_some_and(|local| own_default(tcx, tcx.parent_module_from_def_id(local)) == Some(item))
}

/// The item of `module`'s own that's its default export, what its first
/// `js::export_default!` makes the default, as a second is refused where
/// the module is made: not another module's function it re-exports, nor a
/// binding it holds (ADR 0773's `HeldBinding`).
pub(super) fn own_default(tcx: TyCtxt<'_>, module: LocalModId) -> Option<DefId> {
    let (default, _) = default_exports(tcx, module).into_iter().next()?;
    default.filter(|item| (item.as_local()).is_some_and(|local| tcx.parent_module_from_def_id(local) == module))
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
                        ItemKind::Use(path, _) => match path.res.value_ns {
                            Some(Res::Def(DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }, function)) => {
                                Some(function)
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

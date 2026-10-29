//! Which paths of the crate's bodies name one of its functions through the
//! module that has it, `plugin::extract_events(..)`, with no `use` of the
//! function: read through that module's namespace, `import * as Plugin
//! from "./plugin.js"` and `Plugin.extractEvents(..)`, as React DOM's
//! DOMPluginEventSystem writes `EnterLeaveEventPlugin.extractEvents(..)`
//! (the user's decision, rd114, ADR 1018). A `use` of the function, `use
//! plugin::extract_events;`, and a path through `crate`, `self` or `super`
//! alone, name the function, its named import.

use rustc_hir::def::{DefKind, Res};
use rustc_hir::{self as hir, intravisit};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::HashMap;

/// A path through a module: the function it names, the module, which has
/// it, the crate's or a library's, the name its namespace is imported by,
/// and the module the path is written in.
#[derive(Clone, PartialEq, Eq)]
pub(in crate::lower) struct Through {
    pub(in crate::lower) function: DefId,
    pub(in crate::lower) module: DefId,
    pub(in crate::lower) namespace: String,
    pub(in crate::lower) scope: LocalModId,
}

/// Each path of the crate's bodies that names a function through the
/// module that has it, the crate's or a library's whose manifest lists it
/// (ADR 0100), by its span, and by its call's, which a call's MIR is at:
/// `plugin::f` of `plugin::f(..)`, and `.map(plugin::f)`.
pub(in crate::lower) fn module_paths<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
) -> HashMap<Span, Through> {
    let mut paths = Paths {
        tcx,
        foreign,
        through: HashMap::new(),
    };
    for owner in tcx.hir_body_owners() {
        intravisit::walk_body(&mut paths, tcx.hir_body_owned_by(owner));
    }
    paths.through
}

/// The name a module's namespace is imported by (rd114): its own name, as
/// `crate::lower::namespace_name` cases it, `Plugin` of `plugin`; a
/// library's root's, its crate's, `Scheduler` of `scheduler`.
pub(in crate::lower) fn namespace_of(tcx: TyCtxt<'_>, module: DefId) -> String {
    crate::lower::namespace_name(tcx.item_name(module).as_str())
}

struct Paths<'a, 'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    foreign: &'a crate::lower::library::Foreign<'b, 'tcx>,
    through: HashMap<Span, Through>,
}

impl<'tcx> intravisit::Visitor<'tcx> for Paths<'_, '_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let path = match expr.kind {
            hir::ExprKind::Path(ref qpath) => Some(qpath),
            hir::ExprKind::Call(callee, _) => match callee.kind {
                hir::ExprKind::Path(ref qpath) => Some(qpath),
                _ => None,
            },
            _ => None,
        };
        if let Some(qpath) = path
            && let Some((function, module)) = through(self.tcx, self.foreign, qpath)
        {
            let namespace = namespace_of(self.tcx, module);
            let scope = self.tcx.parent_module(expr.hir_id);
            let through = Through {
                function,
                module,
                namespace,
                scope,
            };
            self.through.insert(expr.span, through);
        }
        intravisit::walk_expr(self, expr);
    }
}

/// What `qpath` names through a module: a function of that module's, the
/// crate's or a library's, by a path whose last but one segment is the
/// module by its name, `plugin` of `plugin::f` and of `crate::plugin::f`,
/// `scheduler` of `scheduler::f`. Not `f` alone, a `use`'s, nor `crate::f`,
/// `self::f` or `super::f`, which name no module but their own place's,
/// nor a function another module has that this one re-exports.
fn through(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    qpath: &hir::QPath<'_>,
) -> Option<(DefId, DefId)> {
    let hir::QPath::Resolved(None, path) = qpath else {
        return None;
    };
    let Res::Def(DefKind::Fn, function) = path.res else {
        return None;
    };
    let [.., module, _] = path.segments else {
        return None;
    };
    let Res::Def(DefKind::Mod, named) = module.res else {
        return None;
    };
    let listed = function.is_local() || foreign.item(function).is_some_and(|item| item.member.is_none());
    let through = listed && !module.ident.is_path_segment_keyword() && tcx.parent(function) == named;
    through.then_some((function, named))
}

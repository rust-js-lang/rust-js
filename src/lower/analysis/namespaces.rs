//! Which paths of the crate's bodies name one of its functions, `const`s
//! or `static`s through the module that has it, `plugin::extract_events(..)`
//! and `fiber::CURRENT`, with no `use` of it: read through that module's
//! namespace, `import * as Plugin from "./plugin.js"` and
//! `Plugin.extractEvents(..)`, as React DOM's DOMPluginEventSystem writes
//! `EnterLeaveEventPlugin.extractEvents(..)` (the user's decisions, rd114
//! and rd114a, ADR 1018). A `use` of the function, `use
//! plugin::extract_events;`, and a path through `crate`, `self` or `super`
//! alone, name the function, its named import.

use crate::lower::recognition::StdItem;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::{self as hir, intravisit};
use rustc_middle::ty::{self, TyCtxt};
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
    // A function, a `const` or a `static`, a thread-local's too, which
    // `thread_local!` makes a `const` (rd114a).
    let Res::Def(DefKind::Fn | DefKind::Const { .. } | DefKind::Static { .. }, function) = path.res else {
        return None;
    };
    let [.., module, _] = path.segments else {
        return None;
    };
    let Res::Def(DefKind::Mod, named) = module.res else {
        return None;
    };
    let listed = function.is_local() || foreign.item(function).is_some_and(|item| item.member.is_none());
    let through = listed
        && !module.ident.is_path_segment_keyword()
        && tcx.parent(function) == named
        && !holds_a_callable(tcx, function);
    through.then_some((function, named))
}

/// Whether `value`, a `const` or a `static`, a thread-local's by what its
/// key holds, may hold a function a call of it runs, a `dyn Fn` or a `fn`
/// pointer: read by its name, so a call of what it holds is `report()`,
/// its `this` `undefined`, as Rust gives a `with_this` function `None`
/// (ADR 0972). Through its namespace, `Report.report()` would give it the
/// namespace object (rd114a, ADR 1018). A function's own item calls none.
fn holds_a_callable(tcx: TyCtxt<'_>, value: DefId) -> bool {
    if tcx.def_kind(value) == DefKind::Fn {
        return false;
    }
    let ty = tcx.type_of(value).instantiate_identity().skip_normalization();
    // A thread-local's key, `LocalKey<T>`, by its `T`: the key's own `fn`
    // pointer is std's, which no read calls.
    let held = match ty.kind() {
        ty::Adt(adt, args) if crate::lower::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey) => {
            args.type_at(0)
        }
        _ => ty,
    };
    held.walk().filter_map(|arg| arg.as_type()).any(|ty| match ty.kind() {
        ty::FnPtr(..) | ty::Closure(..) => true,
        ty::Dynamic(predicates, ..) => predicates.principal_def_id().is_some_and(|id| tcx.is_fn_trait(id)),
        _ => false,
    })
}

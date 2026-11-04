//! A function that only passes its parameters on to a plain function at
//! one instantiation, which its module re-exports in its place where its
//! JS is that call alone (rd575, ADR 1064): which may be, before anything
//! is lowered, and which are, of each module's lowered functions, and
//! what its module re-exports and declares of them.

use super::super::Body;
use super::super::library::Foreign;
use crate::program::Reexport;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

/// Each function of the crate that only passes its parameters on to a
/// plain function at one instantiation (`passed_on::forwarder`, rd575,
/// ADR 1064), by that function, where nothing its JS can't show tells the
/// two apart: the call is of the function's own JS, not a copy its
/// callers need (ADR 0051), nor one the crate compiles of a library's
/// (ADR 0914), whose name is another; the function takes no drop a call
/// may leave out at its end (ADR 0098), and reads no `js::arguments()`
/// (rd332) or `js::this()` (rd683), so it takes what the forwarder's
/// callers give it, and no more;
/// and no body of the forwarder's module names it, as a re-export is no
/// name of its module's own. Its JS is the call alone, which is the
/// module's to ask (`passed_on::forwarded_call`).
pub(super) fn found<'tcx>(
    tcx: TyCtxt<'tcx>,
    (bodies, all_bodies): (&[&Body<'tcx>], &[&Body<'tcx>]),
    (foreign, trait_impls): (&Foreign<'_, 'tcx>, &[DefId]),
    js_given: &super::super::analysis::JsGiven,
    (drop_params, reads_arguments): (
        &HashMap<DefId, Vec<u32>>,
        &super::super::analysis::arguments::ReadsArguments<'tcx>,
    ),
) -> HashMap<DefId, DefId> {
    let recognition = super::super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    let own_js = |function: DefId, (target, args): (DefId, ty::GenericArgsRef<'tcx>)| match target.is_local() {
        true => {
            !reads_arguments.reads_its_call(target)
                && drop_params.get(&target).is_none_or(Vec::is_empty)
                && js_given.called(&recognition, &js_given.main(tcx, function), (target, args)) == Ok(None)
        }
        // A library's export, not one the crate compiles itself, which is
        // no item of its library's (ADR 0914).
        false => {
            !foreign.reads_its_call(target)
                && foreign
                    .item(target)
                    .is_some_and(|item| item.member.is_none() && item.calling.drop_params().is_empty())
        }
    };
    let candidates: HashMap<DefId, DefId> = (bodies.iter())
        .filter_map(|body| {
            let function = body.def_id;
            let (target, args) = super::super::passed_on::forwarder(tcx, function, &body.thir)?;
            own_js(function, (target, args)).then_some((function, target))
        })
        .collect();
    // Of those, each no body of its own module names.
    let module = |id: DefId| id.as_local().map(|local| tcx.parent_module_from_def_id(local));
    let mut named_at_home: HashSet<DefId> = HashSet::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            if let (rustc_middle::thir::ExprKind::ZstLiteral { .. }, ty::FnDef(named, _)) = (&expr.kind, expr.ty.kind())
                && candidates.contains_key(named)
                && module(*named) == module(body.def_id)
            {
                named_at_home.insert(*named);
            }
        }
    }
    (candidates.into_iter())
        .filter(|(function, _)| !named_at_home.contains(function))
        .collect()
}

/// Of `module`'s lowered `functions`, each of `forwarders` whose JS is its
/// call alone (`passed_on::forwarded_call`), by the function it passes on
/// to, taken out: its module re-exports that function in its place
/// (`reexports::forwarded_functions`, rd575, ADR 1064). Not its module's
/// default (ADR 0611), nor one a `pub use` of its module renames (ADR
/// 0596), which name its function, nor one of a function no module of
/// rust-js's has. And what only their calls read, which its module
/// needn't import.
fn in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    (module, functions): (LocalModId, &mut Vec<crate::js::Function>),
    forwarders: &HashMap<DefId, DefId>,
    (fns, foreign, paths): (
        &HashMap<DefId, super::super::FnInfo>,
        &Foreign<'_, 'tcx>,
        &HashMap<LocalModId, Vec<String>>,
    ),
    default_export: Option<DefId>,
    aliased: &[(DefId, (String, String))],
) -> (HashMap<DefId, DefId>, HashSet<String>) {
    let (mut confirmed, mut reads) = (HashMap::new(), HashSet::new());
    for (&function, &target) in forwarders {
        if fns[&function].module != module
            || default_export == Some(function)
            || aliased.iter().any(|&(item, _)| item == function)
            || super::super::reexports::forwarded_from(tcx, module, target, (fns, foreign), paths).is_none()
        {
            continue;
        }
        let Some(at) = functions.iter().position(|f| f.name == fns[&function].name) else {
            continue;
        };
        let output = tcx.fn_sig(function).skip_binder().output().skip_binder();
        let Some(callee) = super::super::passed_on::forwarded_call(&functions[at], output.is_unit()) else {
            continue;
        };
        callee.visit_vars(&mut |name| {
            reads.insert(name.to_string());
        });
        functions.remove(at);
        confirmed.insert(function, target);
    }
    (confirmed, reads)
}

/// What `module` re-exports, its `pub use`s' names `found` and the
/// functions re-exported in place of its `forwarders` whose JS is their
/// call alone, those taken out of its `functions`; and what its
/// declarations re-export, its `pub use`s' alone, as a forwarder is
/// declared as written; and what only those functions' calls read, which
/// it needn't import.
pub(super) struct InPlace {
    pub(super) reexports: Vec<Reexport>,
    pub(super) declared: Vec<Reexport>,
    pub(super) reads: HashSet<String>,
}

pub(super) fn reexported<'tcx>(
    tcx: TyCtxt<'tcx>,
    (module, functions): (LocalModId, &mut Vec<crate::js::Function>),
    forwarders: &HashMap<DefId, DefId>,
    (fns, foreign, paths): (
        &HashMap<DefId, super::super::FnInfo>,
        &Foreign<'_, 'tcx>,
        &HashMap<LocalModId, Vec<String>>,
    ),
    default_export: Option<DefId>,
    aliased: &[(DefId, (String, String))],
    (found, sources): (
        Vec<super::super::reexports::Named>,
        &super::super::sources::CapturedSources,
    ),
) -> InPlace {
    let (confirmed, reads) = in_place(
        tcx,
        (module, functions),
        forwarders,
        (fns, foreign, paths),
        default_export,
        aliased,
    );
    let named = super::super::reexports::forwarded_functions(tcx, module, &confirmed, (fns, foreign), paths, sources);
    InPlace {
        declared: super::super::reexports::assembled(found.clone(), sources),
        reexports: super::super::reexports::assembled(found.into_iter().chain(named).collect(), sources),
        reads,
    }
}

/// Where each module's functions that may be re-exports, `forwarders`,
/// would be re-exported from, which the load-order graph has of it (`load_cycles`).
pub(super) fn reexported_from(
    tcx: TyCtxt<'_>,
    modules: &[LocalModId],
    forwarders: &HashMap<DefId, DefId>,
    (fns, foreign, paths): (
        &HashMap<DefId, super::super::FnInfo>,
        &Foreign<'_, '_>,
        &HashMap<LocalModId, Vec<String>>,
    ),
    sources: &super::super::sources::CapturedSources,
) -> Vec<super::load_cycles::ReexportFrom> {
    (modules.iter())
        .flat_map(|&module| {
            let named =
                super::super::reexports::forwarded_functions(tcx, module, forwarders, (fns, foreign), paths, sources);
            named.into_iter().map(move |(_, from, _)| (module, from))
        })
        .collect()
}

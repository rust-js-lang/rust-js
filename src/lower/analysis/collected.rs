//! The bodies the crate lowers, collected before anything is analyzed:
//! each function's and closure's THIR copied before building MIR steals
//! it, std's bodies a build of std's structs compiles (ADR 0908), the
//! initializers of its statics and constants (ADR 0096) and the inline
//! `const` blocks it writes as code, their integer ranges, and then their
//! MIR (ADR 0364).

use super::*;

/// Copy the THIR of every function and closure in the crate.
///
/// Must run *before* `analysis`: building MIR for borrowck consumes ("steals")
/// the THIR, so this is our only chance to read it. Of `std_iterators`,
/// the differential job's build, no adapter is made in place: each is
/// std's struct (ADR 0908).
pub fn collect_bodies(tcx: TyCtxt<'_>, std_iterators: bool, library: bool) -> Vec<Body<'_>> {
    let mut bodies = lowered_bodies(tcx);
    // The types the crate clones, whose adapters cloned apart hold their
    // parts (ADRs 0525, 0915).
    let cloned = clones::cloned_types(tcx, &bodies.iter().collect::<Vec<_>>(), library);
    crate::lower::body_queries::across_bodies(tcx, &mut bodies, &cloned);
    if std_iterators {
        for body in &mut bodies {
            body.facts.in_place_adapters.clear();
        }
    }
    // What only an unlowered impl reaches is no JS either (ADR 0425).
    let unreached = reached_only_unlowered(tcx, &bodies);
    bodies.retain(|body| !unreached.contains(&tcx.typeck_root_def_id(body.def_id)));
    bodies
}

/// What the integers of the crate's bodies and initializers can be, of
/// each together: a function only calls reach is what its calls give it
/// (ADRs 0576, 0577). Before MIR building steals THIR, as `collect_bodies`.
pub fn collect_ranges<'tcx>(tcx: TyCtxt<'tcx>, bodies: &mut [Body<'tcx>], initializers: &mut [Body<'tcx>]) {
    let only_called = only_called::only_called(tcx);
    let all: Vec<&mut Body<'tcx>> = (bodies.iter_mut().chain(initializers.iter_mut()))
        .filter(|body| body.root().is_some())
        .collect();
    let walked: Vec<_> = (all.iter()).map(|body| (body.def_id, &body.thir, body.expr)).collect();
    let found = crate::lower::body_queries::ranges(tcx, &walked, &only_called);
    drop(walked);
    for (body, ranges) in all.into_iter().zip(found) {
        body.facts.in_bounds = ranges.in_bounds;
        body.facts.exact = ranges.exact;
        body.facts.set_empty = ranges.set_empty;
        body.facts.kept_casts.extend(ranges.kept_casts);
    }
}

/// std's bodies compiled into `@rust-js/std` (ADR 0561):
/// each from the optimized MIR of a `core` built to unwind (#424,
/// `std_sysroot`), with no THIR beside it,
/// as a dictionary has none (`pipeline.rs`'s `no_body`).
pub fn collect_std_bodies(tcx: TyCtxt<'_>) -> Vec<Body<'_>> {
    foreign_bodies(tcx, crate::lower::recognition::compiled_std(tcx).to_vec()).0
}

/// A derived impl whose methods are lowered, written only where they're
/// reached: `Debug`'s, how `{:?}` shows its type (ADR 0060), and `Hash`'s,
/// what a hasher is given of it (ADR 0463).
pub(super) fn lowered_derive(tcx: TyCtxt<'_>, id: DefId) -> bool {
    known_derive(tcx, id)
        && matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && [StdItem::Debug, StdItem::Hash].into_iter().any(|item| {
            is_std_def(
                tcx,
                tcx.impl_trait_ref(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .def_id,
                item,
            )
        })
}

/// The crate's functions a body is lowered of, each with its THIR.
fn lowered_bodies(tcx: TyCtxt<'_>) -> Vec<Body<'_>> {
    let items = tcx.hir_crate_items(());
    items
        .definitions()
        .chain(items.nested_bodies())
        .filter(|&def_id| !from_serde_derive(tcx, def_id))
        .filter(|&def_id| match tcx.def_kind(def_id) {
            // A function declared in an `extern` block is JS's (ADR 0021),
            // and so is one with `#[rust_js::link_name]` (ADR 0039).
            DefKind::Fn => !is_binding(tcx, def_id.to_def_id()),
            // A method of an `impl Type` block (ADR 0047).
            // A derived impl's is never called, but a derived `Debug`'s is
            // how `{:?}` shows its type (ADR 0060), and a derived `Hash`'s
            // what a hasher is given (ADR 0463).
            DefKind::AssocFn => {
                let parent = tcx.parent(def_id.to_def_id());
                tcx.hir_maybe_body_owned_by(def_id).is_some()
                    && (!known_derive(tcx, parent) || lowered_derive(tcx, parent))
                    && !is_unlowered_impl(tcx, parent)
                    && !is_js_object_deref(tcx, parent)
                    && !is_binding(tcx, def_id.to_def_id())
            }
            DefKind::Closure => true,
            // A trait impl's constant: its initializer, lowered in its
            // dictionary where rustc can't compute it for every type at once,
            // a generic impl's, or can't say its value, a `String`'s (ADR
            // 0176). And a trait's default, which a library's dictionary reads
            // as its initializer, never computed where no consumer reads it.
            // Of one with a body: a `type const` has none.
            DefKind::AssocConst { .. } => {
                (tcx.trait_impl_of_assoc(def_id.to_def_id()).is_some()
                    || tcx.trait_of_assoc(def_id.to_def_id()).is_some())
                    && tcx.hir_maybe_body_owned_by(def_id).is_some()
            }
            _ => false,
        })
        .filter_map(|def_id| {
            let (thir, expr) = tcx.thir_body(def_id).ok()?;
            let thir = (*thir.borrow()).clone();
            let facts = crate::lower::body_queries::BodyFacts::collect(tcx, &thir);
            Some(Body {
                def_id: def_id.to_def_id(),
                thir,
                expr,
                facts,
                mir: None,
            })
        })
        .collect()
}

/// The crate's private functions only an unlowered impl's methods call
/// (ADR 0425), a type's own `io::Read`'s helpers, or only what's found so:
/// what no JS calls, and so never lowered. One a consumer may call, or none
/// calls, is lowered as it was.
fn reached_only_unlowered(tcx: TyCtxt<'_>, bodies: &[Body<'_>]) -> IndexSet<DefId> {
    // Each function's callers, by the function each call is in.
    let mut callers: IndexMap<DefId, IndexSet<DefId>> = IndexMap::default();
    let mut calls_in = |caller: DefId, thir: &rustc_middle::thir::Thir<'_>| {
        for expr in thir.exprs.iter() {
            if let ty::FnDef(callee, _) = *expr.ty.kind()
                && callee.is_local()
            {
                callers.entry(callee).or_default().insert(caller);
            }
        }
    };
    for body in bodies {
        calls_in(tcx.typeck_root_def_id(body.def_id), &body.thir);
    }
    let unlowered: IndexSet<DefId> = (tcx.hir_crate_items(()).definitions())
        .filter(|&def_id| {
            tcx.def_kind(def_id) == DefKind::AssocFn
                && is_unlowered_impl(tcx, tcx.parent(def_id.to_def_id()))
                && tcx.hir_maybe_body_owned_by(def_id).is_some()
        })
        .map(|def_id| def_id.to_def_id())
        .collect();
    for &method in &unlowered {
        if let Ok((thir, _)) = tcx.thir_body(method.expect_local()) {
            calls_in(method, &thir.borrow());
        }
    }
    let mut unreached: IndexSet<DefId> = IndexSet::default();
    loop {
        let found = (callers.iter())
            .filter(|&(callee, by)| {
                !unreached.contains(callee)
                    && matches!(tcx.def_kind(*callee), DefKind::Fn | DefKind::AssocFn)
                    && !tcx.effective_visibilities(()).is_reachable(callee.expect_local())
                    && by
                        .iter()
                        .all(|caller| unlowered.contains(caller) || unreached.contains(caller))
            })
            .map(|(&callee, _)| callee)
            .collect::<Vec<_>>();
        if found.is_empty() {
            return unreached;
        }
        unreached.extend(found);
    }
}

/// Each body's MIR, as borrowck reads it (ADR 0364), read once every
/// body's THIR is copied: building MIR steals a body's THIR. What rustc may
/// evaluate first, a constant's or a `const fn`'s: one another body's MIR
/// evaluates, a constant its patterns test, takes that body's MIR, which
/// rustc keeps then only as it evaluates it, its MIR for evaluation.
pub fn collect_mir<'tcx>(tcx: TyCtxt<'tcx>, bodies: &mut [Body<'tcx>]) {
    let evaluated = |def_id: LocalDefId| {
        matches!(
            tcx.def_kind(def_id),
            DefKind::Const { .. } | DefKind::AssocConst { .. } | DefKind::Static { .. } | DefKind::AnonConst
        ) || tcx.is_const_fn(def_id.to_def_id())
    };
    let mut order: Vec<usize> = (0..bodies.len()).collect();
    order.sort_by_key(|&i| !bodies[i].def_id.as_local().is_some_and(evaluated));
    for i in order {
        let body = &mut bodies[i];
        // A std body has its MIR already, the toolchain's.
        let Some(local) = body.def_id.as_local() else { continue };
        if !matches!(
            tcx.def_kind(body.def_id),
            DefKind::Fn
                | DefKind::AssocFn
                | DefKind::Closure
                | DefKind::Const { .. }
                | DefKind::AssocConst { .. }
                | DefKind::Static { .. }
                | DefKind::AnonConst
        ) {
            continue;
        }
        let (built, promoted) = tcx.mir_promoted(local);
        body.mir = match (built.is_stolen() || promoted.is_stolen(), evaluated(local)) {
            (false, _) => Some(crate::lower::mir::Mir::new(
                tcx,
                built.borrow().clone(),
                promoted.borrow().clone(),
            )),
            (true, true) => {
                let def_id = body.def_id;
                Some(crate::lower::mir::Mir::new(
                    tcx,
                    tcx.mir_for_ctfe(def_id).clone(),
                    tcx.promoted_mir(def_id).clone(),
                ))
            }
            (true, false) => None,
        };
    }
    // A call of a method of a trait of classes that gives a field, that
    // field read in place (rd304, ADR 1035 amended): in each body whose
    // calls resolve to one, a default's once its `Self` is known.
    for body in bodies.iter_mut().filter(|body| body.def_id.is_local()) {
        let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body.def_id));
        if let Some(mir) = &mut body.mir {
            mir.read_fields(tcx, typing_env);
        }
    }
}

/// The `const { .. }` blocks of functions that aren't generic, whose
/// initializers are kept in case they're lowered as code.
/// An impl's associated constant whose value rustc's value tree can't say,
/// one of a function, a closure or a `dyn` (ADR 0410): lowered as code, one
/// value for every use, its type's object's, or of a trait impl, a constant
/// of its module. An inherent impl's, or a trait impl's of no type
/// parameters, whose are its dictionary's (ADR 0176); not a trait's
/// default, which is its `Self`'s. Any other's is its value, where it's used.
pub(in crate::lower) fn impl_const(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let Some(imp) = tcx.impl_of_assoc(def_id) else {
        return false;
    };
    let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
    let coded = ty.walk().filter_map(|arg| arg.as_type()).any(|t| {
        matches!(
            t.kind(),
            ty::FnPtr(..) | ty::FnDef(..) | ty::Closure(..) | ty::Dynamic(..)
        )
    });
    coded
        && (tcx.trait_impl_of_assoc(def_id).is_none()
            || !tcx
                .generics_of(imp)
                .own_params
                .iter()
                .any(|p| matches!(p.kind, ty::GenericParamDefKind::Type { .. })))
}

/// Whether `def_id` is an inherent impl's constant, not a class's, that
/// another crate or JS can name,
/// `ReactiveFlags::WATCHING` of a `pub` type: a property of its type's
/// object, `export const ReactiveFlags = { WATCHING: 2 }`, as a constant of
/// code is (ADR 0410), each use still its value where it's written, as a
/// TypeScript `const enum`'s is (#498).
pub(in crate::lower) fn exported_value_const(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::AssocConst { .. })
        && tcx.inherent_impl_of_assoc(def_id).is_some()
        && tcx.generics_of(def_id).count() == 0
        && crate::lower::classes::member(tcx, def_id).is_none()
        && def_id
            .as_local()
            .is_some_and(|local| tcx.effective_visibilities(()).is_exported(local))
}

/// Whether `d`, a body's owner, is a constant only rustc evaluates: one in
/// a type, `FixedOutput<'_, 10>`'s `10` or a const generic's default,
/// which JS never runs, and which rustc can't typecheck as a body before it
/// lowers it.
pub(in crate::lower) fn is_type_system_const(tcx: TyCtxt<'_>, d: LocalDefId) -> bool {
    tcx.def_kind(d) == DefKind::AnonConst && tcx.anon_const_kind(d) != ty::AnonConstKind::NonTypeSystemInline
}

fn inline_consts(tcx: TyCtxt<'_>) -> impl Iterator<Item = LocalDefId> {
    tcx.hir_body_owners().filter(move |&d| {
        tcx.def_kind(d) == DefKind::AnonConst
            && !is_type_system_const(tcx, d)
            && !tcx
                .generics_of(tcx.typeck_root_def_id_local(d))
                .requires_monomorphization(tcx)
    })
}

/// A `const { .. }` block whose value rustc's can't say as JS's,
/// `String::new()`: lowered as code, a constant
/// of its module, as a named one is (ADR 0127). Its value is the one where
/// it's used, of that use's types.
pub(super) fn coded_inline_consts<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> Vec<LocalDefId> {
    let inline: IndexSet<LocalDefId> = inline_consts(tcx).collect();
    let mut coded = Vec::new();
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            if let ExprKind::ConstBlock { did, args } = expr.kind
                && let Some(local) = did.as_local()
                && inline.contains(&local)
                && !coded.contains(&local)
                // Whether it has a value, which no enum's being keyed changes.
                && crate::lower::representation::const_value_js(
                    tcx,
                    &IndexSet::default(),
                    ty::TypingEnv::post_analysis(tcx, body.def_id),
                    did,
                    args,
                    expr.span,
                )
                .is_none()
            {
                coded.push(local);
            }
        }
    }
    coded
}

/// The initializers of the crate's statics and constants, copied as its
/// functions' are, before MIR building steals them: one whose value rustc's
/// can't say is lowered as code instead (ADR 0096).
pub fn collect_initializers(tcx: TyCtxt<'_>) -> Vec<Body<'_>> {
    tcx.hir_crate_items(())
        .definitions()
        .filter(|&def_id| match tcx.def_kind(def_id) {
            DefKind::Const { .. } | DefKind::Static { .. } => true,
            DefKind::AssocConst { .. } => {
                impl_const(tcx, def_id.to_def_id()) || exported_value_const(tcx, def_id.to_def_id())
            }
            _ => false,
        })
        .chain(inline_consts(tcx))
        .filter(|&def_id| !tcx.is_foreign_item(def_id) && tcx.hir_maybe_body_owned_by(def_id).is_some())
        .filter_map(|def_id| {
            let (thir, expr) = tcx.thir_body(def_id).ok()?;
            let thir = (*thir.borrow()).clone();
            let facts = crate::lower::body_queries::BodyFacts::collect(tcx, &thir);
            Some(Body {
                def_id: def_id.to_def_id(),
                thir,
                expr,
                facts,
                mir: None,
            })
        })
        .collect()
}

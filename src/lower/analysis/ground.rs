//! What the crate's analysis stands on, made of its bodies before any of
//! their facts are asked: its bodies but rustc's test harness's, its
//! libraries, `Foreign`, of the writers and the functions' identities its
//! bodies ask of them, and its trait impls, which a `Recognition` of it
//! holds, which types it clones and whether a `Vec`'s capacity is asked.
//! Split from analysis.rs by concept, as a file is at most 1500 lines. It
//! holds none of its bodies, so they can be told which adapters each makes
//! in place once it's made, before the analysis lends them (#1327).

use super::*;

/// What `of` makes of the crate.
pub(in crate::lower) struct Ground<'a, 'tcx> {
    /// rustc's test harness, which the JS runner takes the place of.
    pub(super) harness: Harness,
    pub(super) writer_facts: WriterFacts,
    pub(super) shown_params: ShownParams<'tcx>,
    pub(super) reads_fn_identity: bool,
    pub(super) foreign: super::super::library::Foreign<'a, 'tcx>,
    pub(super) trait_impls: Vec<DefId>,
    /// The types the crate clones (ADRs 0522, 0525), in place or in
    /// generic code.
    pub(in crate::lower) cloned: Cloned<'tcx>,
    /// Whether the crate asks a `Vec`'s capacity, which is kept as std grows
    /// it (ADR 0419).
    pub(super) asks_capacity: bool,
}

impl<'a, 'tcx> Ground<'a, 'tcx> {
    /// Whether a `Vec`'s capacity is kept as std grows it: the crate asks
    /// it, or a library it uses does, which what the crate gives it may be
    /// asked (ADR 0419). The one answer, read before the bodies are told
    /// which adapters each makes in place (#1528) and by the analysis.
    pub(in crate::lower) fn keeps_capacity(&self) -> bool {
        self.asks_capacity || self.foreign.keeps_capacity()
    }

    /// The crate's `Recognition`, of its trait impls and libraries.
    pub(in crate::lower) fn recognition(&self, tcx: TyCtxt<'tcx>) -> Recognition<'_, 'tcx> {
        Recognition {
            tcx,
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls: &self.trait_impls,
            foreign: &self.foreign,
        }
    }
}

/// rustc's test harness, which the JS runner takes the place of (ADR 0026).
pub(super) struct Harness {
    /// Its test markers, a `const` each.
    pub(super) markers: Vec<(LocalDefId, Symbol)>,
    main: Option<DefId>,
}

impl Harness {
    /// `all_bodies` but the harness's.
    pub(super) fn left_out<'b, 'tcx>(&self, tcx: TyCtxt<'tcx>, all_bodies: &'b [Body<'tcx>]) -> Vec<&'b Body<'tcx>> {
        let is_harness = |def_id: DefId| {
            let root = tcx.typeck_root_def_id(def_id);
            Some(root) == self.main || self.markers.iter().any(|&(marker, _)| marker.to_def_id() == root)
        };
        all_bodies.iter().filter(|body| !is_harness(body.def_id)).collect()
    }
}

/// What the crate's analysis stands on (`Ground`).
pub(in crate::lower) fn of<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[Body<'tcx>],
    dependencies: &'a crate::library::Dependencies,
    (library, std_bodies): (bool, bool),
    instances: &'a super::super::Instances,
) -> Ground<'a, 'tcx> {
    // With `--test`, rustc adds a harness: a `const` per test, marked
    // `#[rustc_test_marker]`, and a `main` that runs them with libtest. The
    // JS runner takes their place (ADR 0026), so they're left out.
    let markers: Vec<(LocalDefId, Symbol)> = tcx
        .hir_crate_items(())
        .definitions()
        .filter_map(|def_id| find_attr!(tcx, def_id, RustcTestMarker(label) => (def_id, *label)))
        .collect();
    let harness_main = tcx
        .sess
        .opts
        .test
        .then(|| tcx.entry_fn(()).map(|(main, _)| main))
        .flatten();
    let harness = Harness {
        markers,
        main: harness_main,
    };
    let all_bodies = harness.left_out(tcx, all_bodies);
    // Before its libraries are asked, whose writers it reaches are
    // compiled here where they take less (ADR 0914, amended; #1026).
    let (writer_facts, shown_params) = debug::writer_facts(tcx, &all_bodies, dependencies, (library, std_bodies));
    // And whether it compares a function's identity, of which its libraries'
    // functions that may merge two are compiled here (#1052).
    let reads_fn_identity = rc_counts::reads_fn_identity(tcx, &all_bodies);
    let foreign =
        crate::lower::library::Foreign::new(tcx, dependencies, instances, (writer_facts.writes(), reads_fn_identity));
    let trait_impls: Vec<DefId> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&id| {
            matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
                && (!known_derive(tcx, id.to_def_id())
                    || lowered_derive(tcx, id.to_def_id())
                    || serde_impl(tcx, id.to_def_id()).is_some())
                && (!from_serde_derive(tcx, id) || serde_impl(tcx, id.to_def_id()).is_some())
        })
        .map(|id| id.to_def_id())
        // And std's, whose methods a crate given std's bodies lowers (ADR
        // 0561): each impl of a body of std's, once.
        .chain(all_bodies.iter().filter_map(|body| match body.def_id.is_local() {
            true => None,
            false => tcx.trait_impl_of_assoc(body.def_id),
        }))
        // And a library's the crate compiles itself, which its JS doesn't
        // export, a marker's of no methods too (ADR 0914, amended).
        .chain(instances.impls(tcx))
        .fold(Vec::new(), |mut impls, id| {
            if !impls.contains(&id) {
                impls.push(id);
            }
            impls
        });
    let cloned = clones::cloned_types(tcx, &all_bodies, library);
    let recognition = Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls: &trait_impls,
        foreign: &foreign,
    };
    let asks_capacity = vec_uses::asks_capacity(tcx, &all_bodies)
        || recognition.extends_as_it_fills(&all_bodies)
        || vec_uses::reads_show_capacity(tcx, &all_bodies);
    Ground {
        harness,
        writer_facts,
        shown_params,
        reads_fn_identity,
        foreign,
        trait_impls,
        cloned,
        asks_capacity,
    }
}

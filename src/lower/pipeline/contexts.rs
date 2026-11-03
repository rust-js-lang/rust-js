//! The context a body is lowered in, made one way wherever it's made: each
//! of the crate's bodies, and, for a library none of whose bodies asked
//! what its `RefCell`s are, the one its contract's facts are found in.

use super::super::{Body, BodyFrame, CrateFacts, FnCx, Locals, body_queries::BodyFacts};
use rustc_middle::thir::Thir;
use rustc_middle::ty;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

/// What every body's context is made of: the crate's facts, and the frame
/// of a body that has no THIR of its own, a dictionary's or a codec's.
pub(super) struct Shared<'a, 'tcx> {
    pub(super) tcx: ty::TyCtxt<'tcx>,
    pub(super) krate: &'a CrateFacts<'a, 'tcx>,
    pub(super) no_body: &'a Thir<'tcx>,
    pub(super) no_facts: &'a BodyFacts<'tcx>,
}

/// The context `def_id`'s `body`, of `module`, or a `copy` of it its callers
/// need, is lowered in: `names` its locals may not take, `module_names`
/// its module's, and its scope struct's methods (ADR 1043).
pub(super) fn fn_cx<'a, 'tcx>(
    shared: &Shared<'a, 'tcx>,
    (def_id, body, copy): (DefId, Option<&'a Body<'tcx>>, Option<usize>),
    (module, module_names): (LocalModId, &'a HashSet<String>),
    names: HashSet<String>,
    scope_functions: HashMap<DefId, String>,
) -> FnCx<'a, 'tcx> {
    let tcx = shared.tcx;
    FnCx {
        tcx,
        typing_env: ty::TypingEnv::post_analysis(tcx, def_id),
        given: Default::default(),
        codecs: Vec::new(),
        krate: shared.krate,
        dependencies: Default::default(),
        drops_observed: Default::default(),
        frame: BodyFrame {
            thir: body.map_or(shared.no_body, |body| &body.thir),
            facts: body.map_or(shared.no_facts, |body| &body.facts),
            owner: def_id,
        },
        module,
        locals: Locals::default(),
        names,
        module_names,
        jsx: false,
        writing: Default::default(),
        recursive: Default::default(),
        item: def_id,
        copy,
        walks: Default::default(),
        drop_state: Default::default(),
        scope_functions,
        scope_copies: HashMap::new(),
        scope_self: None,
        scope_values: Vec::new(),
        telling_arms: Default::default(),
    }
}

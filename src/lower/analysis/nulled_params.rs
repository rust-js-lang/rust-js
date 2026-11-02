//! Which of the crate's functions' own `#[rust_js::nullable(..)]`
//! parameters are `null` or a value at every call, never `undefined`
//! (ADR 0861, amended, rd613, the user's decision): tested `!== null`, as
//! upstream tests them. Each is `T | null` as declared, which a JS caller
//! keeps; a call of the crate's gives what it's given as it is (ADR 0861),
//! which may be `undefined`, so one is only where each call of it the crate
//! makes gives it `null` or a value (`stored_given`'s `Nulled`), as its
//! lowering says (`NulledCalls`), and nothing uses the function but to call
//! it. Another crate's call of one gives `?? null` where what it gives may
//! be `undefined`, as its contract says (`Calling::nulled_params`); its use
//! as a value is refused.

use crate::lower::Body;
use crate::lower::refusal::{Refused, refuse};
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, BasicBlock, visit::Visitor};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// The crate's functions whose own nullable parameters may be such, and
/// where the crate's MIR calls them or uses them otherwise.
pub(in crate::lower) struct NulledParams {
    /// Each function's nullable parameters, by index, its `self` first.
    candidates: HashMap<DefId, Vec<usize>>,
    /// Each call of one: the function calling and its block, and what it
    /// calls.
    calls: HashMap<(DefId, BasicBlock), DefId>,
    /// Each used but as a call's function, given or kept as a value.
    as_values: HashSet<DefId>,
    /// What lowering sees of their calls (`record`).
    seen: RefCell<NulledCalls>,
    /// The parameters found so, once every body's calls are seen (`again`).
    nulled: RefCell<HashSet<(DefId, usize)>>,
}

/// What lowering saw of the calls of the crate's functions with nullable
/// parameters: each call lowered, by the function calling and its block,
/// and each parameter one gave what may be `undefined`.
#[derive(Default)]
pub(in crate::lower) struct NulledCalls {
    seen: HashSet<(DefId, BasicBlock)>,
    unnulled: HashSet<(DefId, usize)>,
}

impl NulledParams {
    /// The candidates of `bodies`, the crate's and its closures', and where
    /// each is called or used; each use as a value of another crate's
    /// function that has such a parameter, refused, as its calls are
    /// `null` or a value only where each gives it so (`foreign`'s).
    pub(in crate::lower) fn scan<'tcx>(
        tcx: TyCtxt<'tcx>,
        bodies: &[&Body<'tcx>],
        foreign: &dyn Fn(DefId) -> bool,
    ) -> NulledParams {
        let mut candidates = HashMap::new();
        for body in bodies {
            let id = body.def_id;
            if let Some(params) = candidate(tcx, id) {
                candidates.insert(id, params);
            }
        }
        let mut found = Found {
            candidates: &candidates,
            foreign,
            caller: None,
            calls: HashMap::new(),
            as_values: HashSet::new(),
            refused: Vec::new(),
        };
        for body in bodies {
            let Some(mir) = &body.mir else { continue };
            found.caller = Some(body.def_id);
            found.visit_body(&mir.body);
            for promoted in &mir.promoted {
                found.visit_body(promoted);
            }
        }
        let Found {
            calls,
            as_values,
            refused,
            ..
        } = found;
        for span in refused {
            _ = refuse(
                tcx,
                span,
                Refused::Because(
                    "a function of another crate's whose nullable parameter is `null` at each of its calls, given as a value",
                    "a call of it through the value may give that parameter `undefined`, which its body doesn't test for: call it where it's used, `|loading, chunks| prepare(loading, chunks)` (ADR 0861, amended)",
                ),
            );
        }
        NulledParams {
            candidates,
            calls,
            as_values,
            seen: Default::default(),
            nulled: Default::default(),
        }
    }

    /// The call of `callee`, one that `may_have` such a parameter, in
    /// `caller`'s block `at`, giving each parameter what's `null` or a value
    /// where `nulled` says so.
    pub(in crate::lower) fn record(&self, (caller, at): (DefId, BasicBlock), callee: DefId, nulled: &[bool]) {
        let mut seen = self.seen.borrow_mut();
        seen.seen.insert((caller, at));
        for (i, &nulled) in nulled.iter().enumerate() {
            if !nulled {
                seen.unnulled.insert((callee, i));
            }
        }
    }

    /// Whether `id`'s parameter `index` is found so: its body tests it
    /// `!== null`.
    pub(in crate::lower) fn is_nulled(&self, id: DefId, index: usize) -> bool {
        self.nulled.borrow().contains(&(id, index))
    }

    /// Once every body's calls are seen, each item of `work`, by its
    /// function, `id`'s, of one newly found to have such a parameter,
    /// queued again, to be lowered again, its tests of it `!== null`;
    /// whether any is, none once each is. What each call gives doesn't
    /// change with it: a nullable parameter given on is `null` or a value
    /// either way.
    pub(in crate::lower) fn again<W: Clone>(&self, work: &mut Vec<W>, id: impl Fn(&W) -> DefId) -> bool {
        let found = self.nulled(&self.seen.borrow());
        let mut nulled = self.nulled.borrow_mut();
        let again: HashSet<DefId> = (found.iter().map(|&(id, _)| id))
            .filter(|&id| !nulled.iter().any(|&(of, _)| of == id))
            .collect();
        nulled.extend(found);
        let queued: Vec<W> = work.iter().filter(|&item| again.contains(&id(item))).cloned().collect();
        work.extend(queued);
        !again.is_empty()
    }

    /// The parameters found so, by function and index, which a library's
    /// contract says.
    pub(in crate::lower) fn found(&self) -> HashSet<(DefId, usize)> {
        self.nulled.borrow().clone()
    }

    /// Whether `id`, the crate's, may have such a parameter, whose calls
    /// lowering then tells (`NulledCalls::record`).
    pub(in crate::lower) fn may_have(&self, id: DefId) -> bool {
        self.candidates.contains_key(&id)
    }

    /// The parameters, by function and index, `null` or a value at every
    /// call: each call of the function lowered, as `calls` says, each
    /// giving it so, and the function never used but to be called.
    fn nulled(&self, calls: &NulledCalls) -> HashSet<(DefId, usize)> {
        let all_seen = |id: DefId| {
            (self.calls.iter())
                .filter(|&(_, &callee)| callee == id)
                .all(|(site, _)| calls.seen.contains(site))
        };
        (self.candidates.iter())
            .filter(|&(&id, _)| !self.as_values.contains(&id) && all_seen(id))
            .flat_map(|(&id, params)| params.iter().map(move |&i| (id, i)))
            .filter(|key| !calls.unnulled.contains(key))
            .collect()
    }
}

/// The nullable parameters of `id`, by index, if it's a function of the
/// crate's that only the crate's calls and JS call by name: not a binding,
/// nor a trait's method or one of an impl of a trait, which a dictionary
/// calls (ADR 0049).
fn candidate(tcx: TyCtxt<'_>, id: DefId) -> Option<Vec<usize>> {
    match tcx.def_kind(id) {
        DefKind::Fn => {}
        DefKind::AssocFn if tcx.trait_of_assoc(id).is_none() && tcx.trait_impl_of_assoc(id).is_none() => {}
        _ => return None,
    }
    if crate::lower::bindings::is_binding(tcx, id) {
        return None;
    }
    let names = crate::lower::bindings::nullable_params(tcx, id);
    if names.is_empty() {
        return None;
    }
    let params: Vec<usize> = (tcx.fn_arg_idents(id).iter().enumerate())
        .filter(|(_, ident)| ident.is_some_and(|ident| names.contains(&ident.name)))
        .map(|(i, _)| i)
        .collect();
    (!params.is_empty()).then_some(params)
}

/// What `scan` finds walking MIR.
struct Found<'f> {
    candidates: &'f HashMap<DefId, Vec<usize>>,
    foreign: &'f dyn Fn(DefId) -> bool,
    caller: Option<DefId>,
    calls: HashMap<(DefId, BasicBlock), DefId>,
    as_values: HashSet<DefId>,
    refused: Vec<rustc_span::Span>,
}

impl<'tcx> Visitor<'tcx> for Found<'_> {
    fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
        // A call's function is called, not used: what it's given is.
        if let mir::TerminatorKind::Call { func, args, .. } = &terminator.kind
            && let Some((id, _)) = func.const_fn_def()
        {
            if self.candidates.contains_key(&id)
                && let Some(caller) = self.caller
            {
                self.calls.insert((caller, location.block), id);
            }
            for arg in args {
                self.visit_operand(&arg.node, location);
            }
            return;
        }
        self.super_terminator(terminator, location);
    }

    fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
        let ty::FnDef(id, _) = *constant.const_.ty().kind() else {
            return;
        };
        if self.candidates.contains_key(&id) {
            self.as_values.insert(id);
        } else if !id.is_local() && (self.foreign)(id) {
            self.refused.push(constant.span);
        }
    }
}

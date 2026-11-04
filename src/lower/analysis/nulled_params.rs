//! Which of the crate's functions' own `#[rust_js::nullable(..)]`
//! parameters are `null` or a value at every call, never `undefined`
//! (ADR 0861, amended, rd613, the user's decision): tested `!== null`, as
//! upstream tests them. Each is `T | null` as declared, which a JS caller
//! keeps; a call of the crate's gives what it's given as it is (ADR 0861),
//! which may be `undefined`, so one is only where each call of it the crate
//! makes gives it `null` or a value (`stored_given`'s `Nulled`), as its
//! lowering says (`NulledCalls`), and nothing uses the function but to call
//! it. Its body takes it as `null` or a value only then (`is_nulled`):
//! tested `!== null`, and stored and given on as it is (#601). Another
//! crate's call of one gives `?? null` where what it gives may be
//! `undefined`, as its contract says (`Calling::nulled_params`); its use as
//! a value is refused.
//! A `function::bind` of one with each of its parameters bound is a call of
//! it, given what's bound (rd632).
//! A namespace's method that's the function it passes its parameters on to
//! (`reached::forwarded`, #566) is that function: its body's call of it is
//! no call, and a call of it is one of the function, given each parameter
//! past its `self`; its contract says the function's (#601).
//! And which of the crate's functions return `null` or a value at each of
//! their returns, never `undefined` (`returns_nulled`, rd632): React DOM's
//! `getEventTarget`, `target.parentNode` or `target`, which a call gives
//! such a parameter as it is. Lowering finds it, and a body lowered
//! before it was found that asked is lowered again (`again`).

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
    /// Each namespace's method that's the function it passes its parameters
    /// on to, and that function (`reached::forwarded`).
    forwarded: HashMap<DefId, DefId>,
    /// The functions found to return `null` or a value (`returns_nulled`).
    returns: RefCell<Returns>,
}

/// What lowering saw of the calls of the crate's functions with nullable
/// parameters, each by the item whose lowering saw it, which a lowering
/// again sees anew: each call lowered, by the function calling and its
/// block, and each parameter one gave what may be `undefined`.
#[derive(Default)]
pub(in crate::lower) struct NulledCalls {
    seen: HashSet<(DefId, (DefId, BasicBlock))>,
    unnulled: HashSet<(DefId, (DefId, usize))>,
    /// Each parameter a call gave the caller's own nullable parameter, by
    /// index, which is `null` or a value only where that one is (rd624).
    given_on: HashSet<(DefId, (Param, Param))>,
}

/// A function's parameter, by index, its `self` first.
type Param = (DefId, usize);

/// The crate's functions lowering found return `null` or a value, and, of
/// each it hasn't, the items whose lowering asked.
#[derive(Default)]
struct Returns {
    found: HashSet<DefId>,
    asked: HashMap<DefId, HashSet<DefId>>,
}

impl NulledParams {
    /// The candidates of `bodies`, the crate's and its closures', and where
    /// each is called or used; each use as a value of another crate's
    /// function that has such a parameter, refused, as its calls are
    /// `null` or a value only where each gives it so (`foreign`'s). Each
    /// method of `forwarded` is its function, whose candidate it is not:
    /// its body, that function's JS, calls nothing, and a call or use of it
    /// is the function's (#601).
    pub(in crate::lower) fn scan<'tcx>(
        tcx: TyCtxt<'tcx>,
        bodies: &[&Body<'tcx>],
        forwarded: &HashMap<DefId, DefId>,
        foreign: &dyn Fn(DefId) -> bool,
    ) -> NulledParams {
        let mut candidates = HashMap::new();
        for body in bodies {
            let id = body.def_id;
            if let Some(params) = candidate(tcx, id).filter(|_| !forwarded.contains_key(&id)) {
                candidates.insert(id, params);
            }
        }
        let mut found = Found {
            tcx,
            candidates: &candidates,
            forwarded,
            foreign,
            caller: None,
            calls: HashMap::new(),
            as_values: HashSet::new(),
            refused: Vec::new(),
        };
        for body in bodies {
            let Some(mir) = &body.mir else { continue };
            if forwarded.contains_key(&body.def_id) {
                continue;
            }
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
            forwarded: forwarded.clone(),
            returns: Default::default(),
        }
    }

    /// Of `callee`, the function a call of it is, and how many of the
    /// call's arguments come before that function's: of a namespace's
    /// method that's its function, the function, its `self` past (#601).
    fn called(&self, callee: DefId) -> (DefId, usize) {
        match self.forwarded.get(&callee) {
            Some(&function) => (function, 1),
            None => (callee, 0),
        }
    }

    /// The call of `callee`, one that `may_have` such a parameter, in
    /// `caller`'s block `at`, which lowering `item` saw, giving each
    /// parameter what's `null` or a value where `nulled` says so.
    /// A parameter given the caller's own nullable one, `given_on`'s index,
    /// is so only where that one is, which `nulled` tells once every call
    /// is seen (rd624); of a caller that may have none, a closure's, as
    /// declared.
    /// A call of a namespace's method that's its function is the
    /// function's, given each argument past its `self` (#601).
    pub(in crate::lower) fn record(
        &self,
        (item, (caller, at)): (DefId, (DefId, BasicBlock)),
        callee: DefId,
        nulled: &[bool],
        given_on: &[Option<usize>],
    ) {
        let (callee, past) = self.called(callee);
        let (nulled, given_on) = (
            nulled.get(past..).unwrap_or_default(),
            given_on.get(past..).unwrap_or_default(),
        );
        let mut seen = self.seen.borrow_mut();
        seen.seen.insert((item, (caller, at)));
        for (i, &nulled) in nulled.iter().enumerate() {
            if !nulled {
                seen.unnulled.insert((item, (callee, i)));
            } else if let Some(&Some(param)) = given_on.get(i)
                && self.candidates.contains_key(&caller)
            {
                seen.given_on.insert((item, ((callee, i), (caller, param))));
            }
        }
    }

    /// Whether `callee`'s every return is found `null` or a value, never
    /// `undefined`, as its lowering found (`returned_nulled`); as lowering
    /// `item` asks, which is lowered again once it's found, where it wasn't
    /// yet.
    pub(in crate::lower) fn returns_nulled(&self, item: DefId, callee: DefId) -> bool {
        let mut returns = self.returns.borrow_mut();
        if returns.found.contains(&callee) {
            return true;
        }
        returns.asked.entry(callee).or_default().insert(item);
        false
    }

    /// That `id`'s every return is `null` or a value, as its own lowering
    /// found: what's assigned its result each a `Some(..)`, a nullable
    /// field's read, a call of what gives `null` for `None` (`stored_given`).
    /// What its lowering again finds is found once, as what it's found of
    /// only grows.
    pub(in crate::lower) fn returned_nulled(&self, id: DefId) {
        self.returns.borrow_mut().found.insert(id);
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
    /// change with it: a nullable parameter given on is said of the one it's
    /// given (`record`'s `given_on`), whichever it is.
    /// First, each item that asked whether a function returns `null` or a
    /// value before it was found to, queued again, what it saw of calls
    /// forgotten to be seen anew; the parameters are found only once none
    /// is (rd632).
    pub(in crate::lower) fn again<W: Clone>(&self, work: &mut Vec<W>, id: impl Fn(&W) -> DefId) -> bool {
        let askers: HashSet<DefId> = {
            let mut returns = self.returns.borrow_mut();
            let Returns { found, asked } = &mut *returns;
            let newly: Vec<DefId> = asked.keys().filter(|callee| found.contains(callee)).copied().collect();
            newly
                .iter()
                .flat_map(|callee| asked.remove(callee).unwrap_or_default())
                .collect()
        };
        if !askers.is_empty() {
            let mut seen = self.seen.borrow_mut();
            seen.seen.retain(|(item, _)| !askers.contains(item));
            seen.unnulled.retain(|(item, _)| !askers.contains(item));
            seen.given_on.retain(|(item, _)| !askers.contains(item));
            drop(seen);
            let asked: Vec<W> = work
                .iter()
                .filter(|&item| askers.contains(&id(item)))
                .cloned()
                .collect();
            // Of each asker lowered here: not a namespace's method that's
            // the function it passes on to, which has no JS of its own (#596).
            if !asked.is_empty() {
                work.extend(asked);
                return true;
            }
        }
        let found = self.nulled(&self.seen.borrow());
        let mut nulled = self.nulled.borrow_mut();
        let again: HashSet<DefId> = (found.iter().map(|&(id, _)| id))
            .filter(|&id| !nulled.iter().any(|&(of, _)| of == id))
            .collect();
        nulled.extend(found);
        let queued: Vec<W> = work.iter().filter(|&item| again.contains(&id(item))).cloned().collect();
        let lowered_again = !queued.is_empty();
        work.extend(queued);
        lowered_again
    }

    /// The parameters found so, by function and index, which a library's
    /// contract says: of a namespace's method that's its function, the
    /// function's, each past its `self` (#601).
    pub(in crate::lower) fn found(&self) -> HashSet<(DefId, usize)> {
        let nulled = self.nulled.borrow();
        let methods = (self.forwarded.iter()).flat_map(|(&method, &function)| {
            (nulled.iter())
                .filter(move |&&(of, _)| of == function)
                .map(move |&(_, i)| (method, i + 1))
        });
        nulled.iter().copied().chain(methods).collect()
    }

    /// Whether `id`, the crate's, may have such a parameter, whose calls
    /// lowering then tells (`NulledCalls::record`): a candidate, or a
    /// namespace's method that's one (#601).
    pub(in crate::lower) fn may_have(&self, id: DefId) -> bool {
        self.candidates.contains_key(&self.called(id).0)
    }

    /// The parameters, by function and index, `null` or a value at every
    /// call: each call of the function lowered, as `calls` says, each
    /// giving it so, and the function never used but to be called.
    fn nulled(&self, calls: &NulledCalls) -> HashSet<(DefId, usize)> {
        let seen: HashSet<(DefId, BasicBlock)> = calls.seen.iter().map(|&(_, site)| site).collect();
        let unnulled: HashSet<(DefId, usize)> = calls.unnulled.iter().map(|&(_, key)| key).collect();
        let all_seen = |id: DefId| {
            (self.calls.iter())
                .filter(|&(_, &callee)| callee == id)
                .all(|(site, _)| seen.contains(site))
        };
        let mut nulled: HashSet<(DefId, usize)> = (self.candidates.iter())
            .filter(|&(&id, _)| !self.as_values.contains(&id) && all_seen(id))
            .flat_map(|(&id, params)| params.iter().map(move |&i| (id, i)))
            .filter(|key| !unnulled.contains(key))
            .collect();
        // What's given another's own parameter on is so only where that one
        // is, each passing on followed to its end: the parameters left once
        // none given one that isn't so remains (rd624).
        loop {
            let loose: Vec<(DefId, usize)> = (calls.given_on.iter())
                .filter(|(_, (given, of))| nulled.contains(given) && !nulled.contains(of))
                .map(|&(_, (given, _))| given)
                .collect();
            if loose.is_empty() {
                break nulled;
            }
            for key in loose {
                nulled.remove(&key);
            }
        }
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

/// Whether `id` is a function of the crate's whose returns lowering may
/// find each `null` or a value (`NulledParams::returns_nulled`): one only
/// its crate's calls and JS call by name, as `candidate`'s, whose result
/// is an `Option` not `#[rust_js::nullable]`, which says so already; or a
/// closure of the crate's whose result is an `Option`, which only a call
/// of the crate's asks of, `catch_unwind(|| ..)`'s `Ok` (rd16). What JS
/// gets back of a call of it isn't asked: its returns are its body's.
pub(in crate::lower) fn may_return_nulled(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let option = |output: ty::Ty<'_>| {
        matches!(output.kind(),
            ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option))
    };
    if tcx.def_kind(id) == DefKind::Closure {
        return id.is_local()
            && tcx.coroutine_kind(id).is_none()
            && matches!(tcx.type_of(id).instantiate_identity().skip_normalization().kind(),
                ty::Closure(_, args) if option(args.as_closure().sig().output().skip_binder()));
    }
    let callable = match tcx.def_kind(id) {
        DefKind::Fn => true,
        DefKind::AssocFn => tcx.trait_of_assoc(id).is_none() && tcx.trait_impl_of_assoc(id).is_none(),
        _ => false,
    };
    callable
        && id.is_local()
        && !crate::lower::bindings::is_binding(tcx, id)
        && !crate::lower::bindings::nullable_result(tcx, id)
        && option(tcx.fn_sig(id).skip_binder().skip_binder().output())
}

/// What `scan` finds walking MIR.
struct Found<'f, 'tcx> {
    tcx: TyCtxt<'tcx>,
    candidates: &'f HashMap<DefId, Vec<usize>>,
    /// Each namespace's method that's its function, and that function, a
    /// call or use of the method being one of the function (#601).
    forwarded: &'f HashMap<DefId, DefId>,
    foreign: &'f dyn Fn(DefId) -> bool,
    caller: Option<DefId>,
    calls: HashMap<(DefId, BasicBlock), DefId>,
    as_values: HashSet<DefId>,
    refused: Vec<rustc_span::Span>,
}

impl<'tcx> Visitor<'tcx> for Found<'_, 'tcx> {
    fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
        // A `function::bind` of one with each of its parameters bound, its
        // tuple's items, is a call of it, given those (rd632): what calls
        // the function it makes gives it nothing else.
        if let mir::TerminatorKind::Call { func, args, .. } = &terminator.kind
            && let Some((bind, generic_args)) = func.const_fn_def()
            && crate::lower::recognition::is_function_bind(self.tcx, bind)
            && let Some(id) = bound_all(self.tcx, generic_args).map(|id| self.function(id))
            && self.candidates.contains_key(&id)
            && let Some(caller) = self.caller
        {
            self.calls.insert((caller, location.block), id);
            for arg in args.iter().skip(1) {
                self.visit_operand(&arg.node, location);
            }
            return;
        }
        // A call's function is called, not used: what it's given is.
        if let mir::TerminatorKind::Call { func, args, .. } = &terminator.kind
            && let Some((id, _)) = func.const_fn_def()
        {
            let id = self.function(id);
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
        let id = self.function(id);
        if self.candidates.contains_key(&id) {
            self.as_values.insert(id);
        } else if !id.is_local() && (self.foreign)(id) {
            self.refused.push(constant.span);
        }
    }
}

impl Found<'_, '_> {
    /// The function a call or use of `id` is: of a namespace's method
    /// that's its function, that function (#601).
    fn function(&self, id: DefId) -> DefId {
        self.forwarded.get(&id).copied().unwrap_or(id)
    }
}

/// The function a `function::bind` of `generic_args` binds each parameter
/// of, its tuple as many as it has: `bind(complete_root, None, (root,
/// lanes))` of `complete_root(root, lanes)`.
pub(in crate::lower) fn bound_all<'tcx>(tcx: TyCtxt<'tcx>, generic_args: ty::GenericArgsRef<'tcx>) -> Option<DefId> {
    let mut types = generic_args.types();
    let ty::FnDef(id, _) = *types.next()?.kind() else {
        return None;
    };
    let ty::Tuple(items) = types.nth(1)?.kind() else {
        return None;
    };
    (items.len() == tcx.fn_sig(id).skip_binder().skip_binder().inputs().len()).then_some(id)
}

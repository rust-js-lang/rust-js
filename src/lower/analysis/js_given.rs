//! A function JS calls, whose type parameters JS gives any value, and the
//! copies of the crate's functions that its calls, and its crate's, need
//! where the two need a `Some` of one apart (ADR 0051, amended).
//!
//! A `Some` of a type parameter is the value where no call gives it what
//! may look like `None` (`unboxed_params`), and a box where one does. A
//! function JS calls is given any JS value, `undefined` too, and gives JS
//! the value its `Some` holds, as its declaration's `T | undefined` says:
//! its JS, the one its module exports, takes each of its type parameters
//! so. Where its crate's own Rust gives one of them what may look like
//! `None`, that call needs the box, and calls a copy of it that boxes,
//! compiled from the same MIR beside it. What either passes on a type
//! parameter to is copied so in turn, where what it's given needs its
//! `Some` otherwise than the function's own JS has it.

use std::collections::{BTreeSet, HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::lower::Body;
use crate::lower::recognition::Recognition;

/// What a lowering of a function takes its type parameters as, by their
/// indices: those a `Some` of which is the value, and of those, those JS
/// gives, which may be `undefined` or `null` as they're given.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(in crate::lower) struct Params {
    pub unboxed: BTreeSet<u32>,
    pub js: BTreeSet<u32>,
}

/// Each function's lowerings: the one its module names it by, its JS's,
/// and the copies its callers need besides, each named apart.
#[derive(Default)]
pub(in crate::lower) struct JsGiven {
    /// The generic functions only their callers call, whose calls each
    /// call the lowering that takes what they give as they give it.
    routed: HashSet<DefId>,
    /// Of those, the ones JS calls, whose own JS takes each type parameter
    /// as JS gives it.
    facing: HashSet<DefId>,
    /// The type parameters a `Some` of which is the value as Rust's calls
    /// give them (`unboxed_params`).
    unboxed: HashSet<(DefId, u32)>,
    /// Each function's copies, by the order they were found, and their names.
    copies: HashMap<DefId, Vec<(Params, String)>>,
}

/// What a call gives one of its callee's type parameters, as the lowering
/// it's in takes its own.
enum Given {
    /// What may look like `None`, whose `Some` is a box.
    Nullish,
    /// What JS gave, whose `Some` is the value, `undefined` too.
    Js,
    /// What's never `undefined` or `null`.
    Defined,
}

impl JsGiven {
    /// Of the crate's functions `items`, those its JS exports whose type
    /// parameters a call gives otherwise than JS does, each copy each of
    /// their calls needs, named apart in its module by `name`.
    pub(in crate::lower) fn of_crate<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        all_bodies: &[&Body<'tcx>],
        (items, unboxed): (&HashSet<DefId>, &HashSet<(DefId, u32)>),
        copyable: &dyn Fn(DefId) -> bool,
        name: &mut dyn FnMut(DefId, &str) -> String,
    ) -> JsGiven {
        let tcx = recognition.tcx;
        let routed: HashSet<DefId> = (items.iter().copied())
            .filter(|&id| {
                tcx.typeck_root_def_id(id) == id
                    && super::mutation::copies_by_callers(tcx, recognition.foreign, id)
                    && !super::mutation::type_params(tcx, id).is_empty()
                    && copyable(id)
            })
            .collect();
        let mut given = JsGiven {
            facing: (routed.iter().copied())
                .filter(|&id| js_facing(tcx, recognition.foreign, id))
                .collect(),
            routed,
            unboxed: unboxed.clone(),
            copies: HashMap::new(),
        };
        // What each function's bodies, its closures' too, call.
        let mut calls: HashMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>)>> = HashMap::new();
        for body in all_bodies {
            let caller = tcx.typeck_root_def_id(body.def_id);
            let called = crate::lower::instances::calls_of(body).into_iter();
            let routed = called.filter(|&(callee, ..)| given.routed.contains(&callee));
            (calls.entry(caller).or_default()).extend(routed.map(|(callee, args, _)| (callee, args)));
        }
        // From each function's own JS, a generic one's or not.
        let mut work: Vec<(DefId, Params)> = (calls.keys().copied()).map(|id| (id, given.main(tcx, id))).collect();
        while let Some((caller, params)) = work.pop() {
            for &(callee, args) in calls.get(&caller).into_iter().flatten() {
                let Some(needed) = given.needed(recognition, &params, (callee, args)) else {
                    continue;
                };
                if given.copy_of(callee, &needed).is_some() {
                    continue;
                }
                // Named for what it does apart from its own JS: of one JS
                // calls, the copy Rust's calls box in; of another, the one
                // JS's values reach.
                let named = name(
                    callee,
                    if given.facing.contains(&callee) {
                        "Boxed"
                    } else {
                        "Unboxed"
                    },
                );
                given.copies.entry(callee).or_default().push((needed.clone(), named));
                work.push((callee, needed));
            }
        }
        given
    }

    /// What `id`'s own JS, the one its module names it by, takes its type
    /// parameters as: of one JS calls, each as JS gives it; of any other,
    /// as its crate's calls give them.
    pub(in crate::lower) fn main(&self, tcx: TyCtxt<'_>, id: DefId) -> Params {
        let all: BTreeSet<u32> = super::mutation::type_params(tcx, id).into_iter().collect();
        match self.facing.contains(&id) {
            true => Params {
                unboxed: all.clone(),
                js: all,
            },
            false => Params {
                unboxed: all
                    .into_iter()
                    .filter(|&index| self.unboxed.contains(&(id, index)))
                    .collect(),
                js: BTreeSet::new(),
            },
        }
    }

    /// What lowering `copy` of `id` takes its type parameters as: its own
    /// JS's (`main`) where it's none.
    pub(in crate::lower) fn params(&self, tcx: TyCtxt<'_>, id: DefId, copy: Option<usize>) -> Params {
        match copy {
            Some(at) => self.copies[&id][at].0.clone(),
            None => self.main(tcx, id),
        }
    }

    /// Each copy of `id`'s, by its index, and its name.
    pub(in crate::lower) fn copies(&self, id: DefId) -> impl Iterator<Item = (usize, &str)> {
        (self.copies.get(&id).into_iter().flatten().enumerate()).map(|(at, (_, name))| (at, name.as_str()))
    }

    /// The name of `id`'s copy `at`.
    pub(in crate::lower) fn name(&self, id: DefId, at: usize) -> &str {
        &self.copies[&id][at].1
    }

    /// The copy of `id` that takes its type parameters as `params`.
    fn copy_of(&self, id: DefId, params: &Params) -> Option<usize> {
        (self.copies.get(&id)?.iter()).position(|(made, _)| made == params)
    }

    /// Which lowering of `callee` a call in one that takes its own type
    /// parameters as `params` calls, given `args`: its own JS where that
    /// takes what the call gives as it gives it, `None`; else the copy
    /// that does, by its index, or `Err` where none was made.
    pub(in crate::lower) fn called<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        params: &Params,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Result<Option<usize>, ()> {
        if !self.routed.contains(&callee) {
            return Ok(None);
        }
        match self.needed(recognition, params, (callee, args)) {
            None => Ok(None),
            Some(needed) => self.copy_of(callee, &needed).map(Some).ok_or(()),
        }
    }

    /// What a call needs `callee`, one `routed`, to take its type parameters as, where its
    /// own JS doesn't: a `Some` of what may look like `None` a box, and of
    /// what JS gave the value.
    fn needed<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        params: &Params,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<Params> {
        let tcx = recognition.tcx;
        let (mut nullish, mut js) = (BTreeSet::new(), BTreeSet::new());
        for index in super::mutation::type_params(tcx, callee) {
            let Some(given) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
                continue;
            };
            match given_as(recognition, params, given) {
                Given::Nullish => _ = nullish.insert(index),
                Given::Js => _ = js.insert(index),
                Given::Defined => {}
            }
        }
        let main = self.main(tcx, callee);
        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) {
            return None;
        }
        let unboxed = (super::mutation::type_params(tcx, callee).into_iter())
            .filter(|&index| self.unboxed.contains(&(callee, index)) || js.contains(&index))
            .collect();
        Some(Params { unboxed, js })
    }
}

/// What `given`, which a call gives a type parameter, is, as the lowering
/// the call is in takes its own type parameters (`params`).
fn given_as<'tcx>(recognition: &Recognition<'_, 'tcx>, params: &Params, given: Ty<'tcx>) -> Given {
    match crate::lower::representation::payload_through(recognition, given).kind() {
        ty::Param(param) if params.js.contains(&param.index) => Given::Js,
        ty::Param(param) if params.unboxed.contains(&param.index) => Given::Defined,
        _ if crate::lower::representation::may_look_like_none(recognition, given) => Given::Nullish,
        _ => Given::Defined,
    }
}

/// Whether JS calls `id`, the crate's function its module exports: a
/// function, or an inherent impl's method, that's `pub`, and no library's
/// function the crate compiles itself (ADR 0914), which no other crate or
/// JS calls.
pub(in crate::lower) fn js_facing(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    id: DefId,
) -> bool {
    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && tcx.visibility(id).is_public()
        && !foreign.is_instance(id)
        && (tcx.def_kind(id) != DefKind::AssocFn || tcx.inherent_impl_of_assoc(id).is_some())
}

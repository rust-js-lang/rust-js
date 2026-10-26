//! Which thread-locals are made where they're first used, as Rust makes
//! each (ADR 0037, amended): those whose initializer runs code of the
//! crate's that could tell when it runs, or may panic. One that runs only
//! what std's and bindings' calls make, or what a function of the crate's
//! that only makes a value makes, React's `createCursor(null)`, is made as
//! its module loads, as JS writes it, `createContext([])`: the guide's
//! semantics list that difference. Of those, one whose maker is of a
//! module that may load after its own is made where it's first used too,
//! which the pipeline finds of the modules' imports (`made_by`).

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::fn_pointers::FnPointers;

use crate::lower::Body;

/// The thread-locals made where they're first used, and of those made as
/// their modules load, the crate's functions each one's initializer has
/// make its value (`makes_only`), by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    pub(in crate::lower) made_by: HashMap<LocalDefId, Vec<DefId>>,
}

/// The thread-locals, by their keys, whose initializer (`inits`, each
/// initializer's key) runs the crate's code or may panic; a fn pointer's
/// call runs what `pointers` says it may be (ADR 0907). And those made so
/// already, `made_where_used`, which a lowering of the crate found make
/// their value by a function of a module that may load after theirs.
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    (inits, made_where_used): (&HashMap<LocalDefId, LocalDefId>, &HashSet<LocalDefId>),
    pointers: &FnPointers<'tcx>,
) -> LazyLocals {
    let by_id: HashMap<DefId, &Body<'tcx>> = bodies.iter().map(|&body| (body.def_id, body)).collect();
    let mut found = LazyLocals {
        lazy: made_where_used.clone(),
        made_by: HashMap::new(),
    };
    for body in bodies {
        let Some(init) = body.def_id.as_local() else { continue };
        let Some(&key) = inits.get(&init) else { continue };
        let Some(mir) = &body.mir else { continue };
        let mut makers = Makers {
            tcx,
            bodies: &by_id,
            asked: Vec::new(),
            found: Vec::new(),
        };
        if made_where_used.contains(&key) || runs_the_crates(tcx, init, &mir.body, (pointers, &mut makers)) {
            found.lazy.insert(key);
        } else if !makers.found.is_empty() {
            found.made_by.insert(key, makers.found);
        }
    }
    found
}

/// What an initializer calls of the crate's that only makes a value
/// (`makes_only`), each found, and those being asked, which call
/// themselves.
struct Makers<'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    bodies: &'b HashMap<DefId, &'b Body<'tcx>>,
    asked: Vec<DefId>,
    found: Vec<DefId>,
}

impl<'tcx> Makers<'_, 'tcx> {
    /// Whether `id`, a function of the crate's, only makes a value of what
    /// it's given, as React's `createCursor` makes its `StackCursor`: it
    /// reads no `static` or thread-local, may not panic, drops nothing of
    /// the crate's with a destructor, and calls only std's that only make
    /// a value (`std_makes_only`), and functions of the crate's that do, not
    /// itself; no binding's, closure's, `dyn`'s or fn pointer's. What
    /// it runs then can't tell when, and is the same made as its module
    /// loads.
    fn makes_only(&mut self, id: DefId) -> bool {
        let tcx = self.tcx;
        if self.found.contains(&id) {
            return true;
        }
        if self.asked.contains(&id) || tcx.def_kind(id) != DefKind::Fn {
            return false;
        }
        let Some(mir) = self.bodies.get(&id).and_then(|body| body.mir.as_ref()) else {
            return false;
        };
        let body = &mir.body;
        let typing_env = ty::TypingEnv::post_analysis(tcx, id);
        // What it reads of a `static` or a thread-local.
        struct Reads<'tcx> {
            tcx: TyCtxt<'tcx>,
            state: bool,
        }
        impl<'tcx> mir::visit::Visitor<'tcx> for Reads<'tcx> {
            fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
                self.state |= constant.check_static_ptr(self.tcx).is_some();
            }
            fn visit_rvalue(&mut self, rvalue: &mir::Rvalue<'tcx>, location: mir::Location) {
                self.state |= matches!(rvalue, mir::Rvalue::ThreadLocalRef(_));
                self.super_rvalue(rvalue, location);
            }
        }
        let mut reads = Reads { tcx, state: false };
        mir::visit::Visitor::visit_body(&mut reads, body);
        if reads.state {
            return false;
        }
        self.asked.push(id);
        let makes = body.basic_blocks.iter().all(|data| match &data.terminator().kind {
            TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => false,
            TerminatorKind::Call { func, .. } => match crate::lower::fn_def(func.ty(&body.local_decls, tcx)) {
                Some((callee, args)) if !crate::lower::bindings::is_binding(tcx, callee) => {
                    match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args) {
                        Ok(Some(instance)) => match instance.def {
                            ty::InstanceKind::Item(callee) if callee.is_local() => self.makes_only(callee),
                            ty::InstanceKind::Item(callee) => std_makes_only(tcx, callee),
                            _ => false,
                        },
                        _ => false,
                    }
                }
                _ => false,
            },
            TerminatorKind::Drop { place, .. } => !drops_the_crates(tcx, place.ty(&body.local_decls, tcx).ty),
            _ => true,
        });
        self.asked.pop();
        if makes {
            self.found.push(id);
        }
        makes
    }
}

/// Whether `id`, std's, only makes a value of what it's given: a `const
/// fn` of no trait, asking nothing of its type parameters but their size,
/// `RefCell::new`, `Vec::new`; not a `const` trait's method, `Iterator`'s,
/// nor one bound by a trait, whose impl may be the crate's, or a closure.
fn std_makes_only(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let sized = tcx.lang_items().sized_trait();
    tcx.is_const_fn(id)
        && tcx.trait_of_assoc(id).is_none()
        && (tcx.clauses_of(id).instantiate_identity(tcx).into_iter()).all(|(clause, _)| {
            match clause.skip_normalization().kind().skip_binder() {
                ty::ClauseKind::Trait(predicate) => Some(predicate.def_id()) == sized,
                _ => true,
            }
        })
}

/// Whether a value of `ty` holds one of the crate's with a destructor,
/// whose drop runs it.
fn drops_the_crates<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    ty.walk().any(|part| {
        part.as_type()
            .and_then(|ty| ty.ty_adt_def())
            .is_some_and(|adt| adt.did().is_local() && adt.has_dtor(tcx))
    })
}

/// Whether `body`, `init`'s, runs code of the crate's or may panic: a call
/// of a body of the crate's, a function, a method of its impl or a closure,
/// but a function that only makes a value (`Makers`, which keeps it), or
/// of what rust-js can't see the body of; a fn pointer's call of what it
/// may be (`pointers`), where any is the crate's or it may be anything; a
/// drop of a value of the crate's with a destructor; a check that may
/// fail; or a call that never returns, a panic's.
fn runs_the_crates<'tcx>(
    tcx: TyCtxt<'tcx>,
    init: LocalDefId,
    body: &mir::Body<'tcx>,
    (pointers, makers): (&FnPointers<'tcx>, &mut Makers<'_, 'tcx>),
) -> bool {
    let typing_env = ty::TypingEnv::post_analysis(tcx, init.to_def_id());
    // Whether calling `def_id`, of `args`, runs the crate's code.
    let runs = |def_id: DefId, args: ty::GenericArgsRef<'tcx>, makers: &mut Makers<'_, 'tcx>| {
        if tcx.is_closure_like(def_id) {
            return def_id.is_local();
        }
        !crate::lower::bindings::is_binding(tcx, def_id)
            && match crate::lower::items::resolve_instance_in(tcx, typing_env, def_id, args) {
                Ok(Some(instance)) => match instance.def {
                    ty::InstanceKind::Item(id) => id.is_local() && !makers.makes_only(id),
                    // A shim of std's, a clone's or a drop's, runs what it's
                    // of: the crate's where that is.
                    _ => (instance.args.types()).any(|ty| ty.ty_adt_def().is_some_and(|adt| adt.did().is_local())),
                },
                _ => true,
            }
    };
    body.basic_blocks.iter().any(|data| match &data.terminator().kind {
        TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => true,
        TerminatorKind::Call { func, .. } => {
            let ty = func.ty(&body.local_decls, tcx);
            match crate::lower::fn_def(ty) {
                Some((def_id, args)) => runs(def_id, args, makers),
                // A fn pointer's runs what it may be (ADR 0907): of a
                // binding's alone, `js::date::now`, none of the crate's.
                None if ty.is_fn_ptr() => (pointers.may_be(tcx, ty))
                    .is_none_or(|may_be| may_be.iter().any(|&(def_id, args)| runs(def_id, args, makers))),
                // A closure's or a `dyn Fn`'s, which may be the crate's.
                None => true,
            }
        }
        // A drop of a value of the crate's that has a destructor runs it.
        TerminatorKind::Drop { place, .. } => drops_the_crates(tcx, place.ty(&body.local_decls, tcx).ty),
        _ => false,
    })
}

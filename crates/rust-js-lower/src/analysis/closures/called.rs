//! How generic code calls what it's given (#2031): which of the crate's
//! closures it calls by value, `FnOnce`'s `call_once`, which drops a closure
//! once it's run, where JS's call of a closure rustc infers `Fn` or `FnMut`
//! drops none of what it holds; and which of its functions' type parameters
//! hold what such a call calls, so a boxed `Fn` given there is given as one
//! whose call drops it (#2033).
//!
//! A call by value is a naming, in any body the crate reads, of a function
//! it has no body of, `FnOnce::call_once`, std's or a binding's, that calls
//! what it's given there by `FnOnce` alone: of a type parameter its caller
//! calls so too, or, a binding's or a function trait's, which drops nothing
//! it calls, of anything. What each closure given there is, through every
//! caller, is the callers' instances' carrier's (`instances_of`, ADR 1115).

use crate::instances::{Callers, Held, OnTheWay, callers_of, held_of, on_the_way};
use crate::recognition::{function_kind, is_std_crate};
use crate::{Body, bindings};
use rust_js_program::collections::IndexSet;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, ClosureKind, Ty, TyCtxt};

/// What generic code calls by value and by `&` of what it's given.
#[derive(Default)]
pub(super) struct Called<'tcx> {
    /// The closures rustc infers `Fn` or `FnMut` that a call by value may
    /// call: its `call_once` drops each once it's run.
    pub(super) by_value: IndexSet<DefId>,
    /// Those a call by `&` or `&mut` may call, or a call by value of a
    /// reference to one.
    pub(super) by_ref: IndexSet<DefId>,
    /// Each function's type parameter whose value a call by value calls.
    pub(super) params_by_value: IndexSet<(DefId, u32)>,
    /// Each whose value a call by `&` or `&mut` calls.
    pub(super) params_by_ref: IndexSet<(DefId, u32)>,
    /// How many calls by value were of what no caller the crate sees
    /// answers, whose closures are each the crate gives generic code.
    pub(super) unseen: usize,
    /// Each boxed `Fn` a call by value calls, given as a parameter's value,
    /// the function and the parameter with its `dyn` (#2033).
    pub(super) boxes_given: IndexSet<(DefId, u32, Ty<'tcx>)>,
    /// And each `dyn` of one held in what's given, or given back (#2050).
    pub(super) boxes_held: IndexSet<Ty<'tcx>>,
}

/// How a call calls what it's given.
#[derive(Clone, Copy, PartialEq)]
enum By {
    Value,
    Ref,
}

/// What generic code calls of what `all_bodies` give it, `names`' namings.
pub(super) fn called<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    names: &crate::call_graph::Names<'tcx>,
    compiled: &dyn Fn(DefId) -> bool,
) -> Called<'tcx> {
    let callers = callers_of(tcx, names, all_bodies);
    let held = held_of(tcx, all_bodies);
    let bodies: IndexSet<DefId> = all_bodies.iter().map(|body| body.def_id).collect();
    let mut called = Called::default();
    let mut unseen_ref = 0;
    for (&callee, namings) in callers.iter() {
        if bodies.contains(&callee) {
            continue;
        }
        let declared = ty::GenericArgs::identity_for_item(tcx, callee);
        // A binding's call, or a function trait's, drops nothing it calls;
        // std's drops what it calls by `&` as it returns (ADR 0470).
        let fn_trait = (tcx.trait_of_assoc(callee)).is_some_and(|tr| tcx.fn_trait_kind_from_def_id(tr).is_some());
        let binding = bindings::is_binding(tcx, callee);
        let drops_none = binding || fn_trait || compiled(callee);
        // A library's function the crate has no body of, or its trait's
        // method, may call what it's
        // given by value, or by `&`, however its bounds read: a closure given
        // there is called both ways, but one its own parameter is bound to
        // call by `FnOnce` alone, which nothing calls by `&`; a boxed `Fn`
        // in what it's given can't be given as one whose call drops it.
        let library = !binding && !fn_trait && !callee.is_local() && !is_std_crate(tcx, callee);
        for &(caller, args, _) in namings {
            for (at, arg) in args.iter().enumerate() {
                let (Some(arg), Some(param)) = (arg.as_type(), declared.get(at).and_then(|p| p.as_type())) else {
                    continue;
                };
                if library {
                    let once = function_kind(tcx, callee, param) == Some(ClosureKind::FnOnce);
                    called.given_unseen(
                        tcx,
                        (&callers, &held),
                        (caller, tcx.erase_and_anonymize_regions(arg)),
                        once,
                    );
                    continue;
                }
                let given = function_kind(tcx, caller, arg);
                let by = match function_kind(tcx, callee, param) {
                    Some(ClosureKind::FnOnce) if given == Some(ClosureKind::FnOnce) || drops_none => By::Value,
                    _ if matches!(given, Some(ClosureKind::Fn | ClosureKind::FnMut)) => By::Ref,
                    _ => continue,
                };
                let arg = tcx.erase_and_anonymize_regions(arg);
                if !called.site(tcx, (&callers, &held), (caller, arg), by) && by == By::Ref {
                    unseen_ref += 1;
                }
            }
        }
    }
    // What can't be seen may be any closure the crate gives generic code.
    if called.unseen > 0 || unseen_ref > 0 {
        let given = given_by_value(&callers);
        if called.unseen > 0 {
            called.by_value.extend(given.iter().copied());
        }
        if unseen_ref > 0 {
            called.by_ref.extend(given);
        }
    }
    called
}

impl<'tcx> Called<'tcx> {
    /// What `caller` gives a library's function as `arg`, which it may call
    /// by value or by `&`, or by value alone where `once`, its parameter's
    /// bound: each closure on the way to it, or in it, and each boxed `Fn`,
    /// which no call can give as one that drops it but its own argument
    /// where `once`, rule 4's (#2031, #2050).
    fn given_unseen(
        &mut self,
        tcx: TyCtxt<'tcx>,
        (callers, held): (&Callers<'tcx>, &Held<'tcx>),
        (caller, arg): (DefId, Ty<'tcx>),
        once: bool,
    ) {
        let mut on = OnTheWay::default();
        if !on_the_way(tcx, (callers, held), (caller, arg), false, &mut on) {
            self.unseen += 1;
        }
        let itself = matches!(arg.kind(), ty::Closure(..));
        let [by_value, by_ref] = on.closures;
        for closure in by_value.into_iter().chain(by_ref) {
            self.by_value.insert(closure);
            if !(once && itself) {
                self.by_ref.insert(closure);
            }
        }
        self.boxes_held.extend(on.boxes_held);
        self.boxes_held.extend(on.boxes_given.into_iter().map(|(_, _, ty)| ty));
        let mut inside = IndexSet::default();
        closures_in(arg, &mut inside);
        for closure in inside.into_iter().filter(|_| !itself) {
            self.by_value.insert(closure);
            self.by_ref.insert(closure);
        }
        self.boxes_held
            .extend(boxes_in(tcx, arg).into_iter().filter(|_| !(once && arg.is_box())));
    }

    /// A call `by` value or by `&` of `arg`, what `caller` gives there: each
    /// closure and each type parameter on the way to it (`on_the_way`).
    /// False where it can't be seen.
    fn site(
        &mut self,
        tcx: TyCtxt<'tcx>,
        (callers, held): (&Callers<'tcx>, &Held<'tcx>),
        (caller, arg): (DefId, Ty<'tcx>),
        by: By,
    ) -> bool {
        let mut on = OnTheWay::default();
        let seen = on_the_way(tcx, (callers, held), (caller, arg), by == By::Ref, &mut on);
        let [params_by_value, params_by_ref] = on.params;
        let [by_value, by_ref] = on.closures;
        self.boxes_given.extend(on.boxes_given);
        self.boxes_held.extend(on.boxes_held);
        self.params_by_value.extend(params_by_value);
        self.params_by_ref.extend(params_by_ref);
        self.by_value.extend(by_value);
        self.by_ref.extend(by_ref);
        if !seen && by == By::Value {
            self.unseen += 1;
        }
        seen
    }
}

/// Each closure rustc infers `Fn` or `FnMut` that a naming gives generic
/// code by value, as or in what it gives, not behind a reference.
fn given_by_value(callers: &Callers<'_>) -> IndexSet<DefId> {
    let mut given = IndexSet::default();
    for (_, args, _) in callers.values().flatten() {
        args.types().for_each(|ty| closures_in(ty, &mut given));
    }
    given
}

/// Each closure rustc infers `Fn` or `FnMut` that `ty` is or holds, not
/// behind a reference.
fn closures_in(ty: Ty<'_>, out: &mut IndexSet<DefId>) {
    match *ty.kind() {
        ty::Closure(id, args) => {
            if args.as_closure().kind() != ClosureKind::FnOnce {
                out.insert(id);
            }
        }
        ty::Adt(_, args) => args.types().for_each(|ty| closures_in(ty, out)),
        ty::Tuple(tys) => tys.iter().for_each(|ty| closures_in(ty, out)),
        ty::Array(ty, _) | ty::Slice(ty) => closures_in(ty, out),
        _ => {}
    }
}

/// Each `dyn Fn` or `dyn FnMut` a box `ty` holds, not behind a reference,
/// is.
fn boxes_in<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    let mut boxes = Vec::new();
    let mut todo = vec![ty];
    while let Some(ty) = todo.pop() {
        match *ty.kind() {
            ty::Dynamic(predicates, ..)
                if (predicates.principal_def_id()).is_some_and(|tr| {
                    matches!(
                        tcx.fn_trait_kind_from_def_id(tr),
                        Some(ClosureKind::Fn | ClosureKind::FnMut)
                    )
                }) =>
            {
                boxes.push(ty)
            }
            ty::Adt(_, args) => todo.extend(args.types()),
            ty::Tuple(tys) => todo.extend(tys.iter()),
            ty::Array(ty, _) | ty::Slice(ty) => todo.push(ty),
            _ => {}
        }
    }
    boxes
}

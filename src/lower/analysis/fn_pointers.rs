//! What a fn pointer may be (ADR 0903): of each fn pointer type, the
//! functions and closures the crate's code makes one of, each coercion of
//! one in any of its bodies; or anything, where a value of the type may
//! come from code that isn't the crate's, a library's or JS's, which may
//! make one of any function, or a JS function of its own.
//!
//! A fn pointer is made only by a coercion, of a function or of a closure
//! that captures nothing, which rustc's typeck records wherever it's made.
//! One the crate didn't make reaches its code only from outside it: a
//! library's or a binding's call gives it, or is given what may hold one
//! to change; a library's static or constant holds it; JS or a consumer
//! calls an exported function with it, or is given one that holds it; or
//! std's code makes it of what it isn't given, as `transmute` does.

use crate::lower::bindings::is_binding;
use crate::lower::recognition::Recognition;
use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{self, Visitor};
use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind, PointerCoercion};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt, TypeckResults};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Of each fn pointer type, by `key`, what the crate makes one of; and
/// which types a value of which may come from outside its code, which
/// may be any function.
#[derive(Default)]
pub(in crate::lower) struct FnPointers<'tcx> {
    made: HashMap<Ty<'tcx>, Vec<(DefId, ty::GenericArgsRef<'tcx>)>>,
    open: HashSet<Ty<'tcx>>,
    /// A value of a type rustc can't normalize may come from outside,
    /// which may hold any fn pointer.
    all_open: bool,
}

impl<'tcx> FnPointers<'tcx> {
    /// What each fn pointer type of the crate's code may be: each body's
    /// coercions, and what may come from outside it.
    pub(in crate::lower) fn of_crate(recognition: &Recognition<'_, 'tcx>) -> Self {
        let tcx = recognition.tcx;
        let mut found = FnPointers::default();
        for owner in tcx
            .hir_body_owners()
            .filter(|&owner| !super::is_type_system_const(tcx, owner))
        {
            // What JS or a consumer may call it with, or be given by it.
            if matches!(tcx.def_kind(owner), DefKind::Fn | DefKind::AssocFn)
                && crate::lower::library::reachable(tcx, owner.to_def_id())
            {
                let sig = tcx.instantiate_bound_regions_with_erased(
                    tcx.fn_sig(owner).instantiate_identity().skip_normalization(),
                );
                for ty in sig.inputs_and_output {
                    found.given_in(recognition, ty);
                }
            }
            let mut made = Made {
                recognition,
                results: tcx.typeck(owner),
                found: &mut found,
            };
            made.visit_body(tcx.hir_body_owned_by(owner));
        }
        found
    }

    /// What a value of `ptr`, a fn pointer type, may be: each function or
    /// closure, of its generic arguments, the crate makes one of; `None`,
    /// anything.
    pub(in crate::lower) fn may_be(
        &self,
        tcx: TyCtxt<'tcx>,
        ptr: Ty<'tcx>,
    ) -> Option<&[(DefId, ty::GenericArgsRef<'tcx>)]> {
        let key = key(tcx, ptr)?;
        if self.all_open || self.open.contains(&key) {
            return None;
        }
        Some(self.made.get(&key).map_or(&[], Vec::as_slice))
    }

    /// `f` made a fn pointer of type `ptr`.
    fn made(&mut self, tcx: TyCtxt<'tcx>, ptr: Ty<'tcx>, f: DefId, args: ty::GenericArgsRef<'tcx>) {
        match key(tcx, ptr) {
            // Of generic code's arguments, which only its callers say.
            Some(key) if !args.has_param() => self.made.entry(key).or_default().push((f, args)),
            Some(key) => _ = self.open.insert(key),
            None => {}
        }
    }

    /// A value of `ty` may come from outside the crate's code: each fn
    /// pointer type it may hold may be any function.
    fn given_in(&mut self, recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) {
        let held = held(recognition, ty);
        self.all_open |= held.any;
        self.open.extend(held.keys);
    }
}

/// A fn pointer type as each of its values is, whatever its lifetimes:
/// `for<'a> fn(&'a u32)` and `fn(&'static u32)` are one. `None` of one
/// that's not a fn pointer, or whose signature has a binder of its own,
/// whose types aren't compared.
fn key<'tcx>(tcx: TyCtxt<'tcx>, ptr: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::FnPtr(..) = ptr.kind() else { return None };
    let sig = tcx.instantiate_bound_regions_with_erased(ptr.fn_sig(tcx));
    let nested = (sig.inputs_and_output.iter())
        .flat_map(|ty| ty.walk())
        .filter_map(|part| part.as_type())
        .any(|part| matches!(part.kind(), ty::FnPtr(..) | ty::Dynamic(..)));
    (!nested).then(|| tcx.erase_and_anonymize_regions(Ty::new_fn_ptr(tcx, ty::Binder::dummy(sig))))
}

/// The fn pointer types a value of a type may hold, through what it's
/// made of, the fields of a type that isn't std's too; and whether it's of
/// a type rustc can't normalize, which may hold any. Not a type
/// parameter's: generic code given one from outside calls it only by a
/// bound, as anything, and gives it to the crate's code only where a call
/// of it says the type, whose result `Made::call` holds.
#[derive(Default)]
struct Held<'tcx> {
    keys: HashSet<Ty<'tcx>>,
    any: bool,
}

fn held<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> Held<'tcx> {
    fn walk<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        ty: Ty<'tcx>,
        seen: &mut HashSet<Ty<'tcx>>,
        held: &mut Held<'tcx>,
    ) {
        let tcx = recognition.tcx;
        for part in ty.walk().filter_map(|part| part.as_type()) {
            match part.kind() {
                ty::FnPtr(..) => match key(tcx, part) {
                    Some(key) => _ = held.keys.insert(key),
                    None => held.any = true,
                },
                ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Projection { .. } | ty::Inherent { .. } | ty::Free { .. },
                        ..
                    },
                ) if !part.has_param() => {
                    match tcx.try_normalize_erasing_regions(
                        ty::TypingEnv::fully_monomorphized(),
                        ty::Unnormalized::new_wip(part),
                    ) {
                        Ok(normal) if normal != part => walk(recognition, normal, seen, held),
                        _ => held.any = true,
                    }
                }
                ty::Adt(adt, args)
                    if (adt.did().is_local() || !recognition.in_sysroot(adt.did())) && seen.insert(part) =>
                {
                    for field in adt.all_fields() {
                        walk(recognition, field.ty(tcx, args).skip_normalization(), seen, held);
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Held::default();
    walk(recognition, ty, &mut HashSet::new(), &mut found);
    found
}

/// Each coercion to a fn pointer of a body, and each value that may come
/// from outside the crate's code.
struct Made<'a, 'b, 'tcx> {
    recognition: &'a Recognition<'b, 'tcx>,
    results: &'tcx TypeckResults<'tcx>,
    found: &'a mut FnPointers<'tcx>,
}

impl<'tcx> Made<'_, '_, 'tcx> {
    /// A call of `f`, giving `result` of `inputs`: a library's or a
    /// binding's may give any fn pointer, and change one it's given what
    /// holds; std's gives only what it's given, but where it makes one of
    /// what it isn't, as `transmute` does.
    fn call(&mut self, f: DefId, result: Ty<'tcx>, inputs: &[Ty<'tcx>]) {
        let recognition = self.recognition;
        let tcx = recognition.tcx;
        if matches!(tcx.def_kind(f), DefKind::Ctor(..)) {
            return;
        }
        // Generic code of the crate's gives what its caller's types say,
        // which it may have from outside as a type parameter's.
        if f.is_local() && !is_binding(tcx, f) {
            if tcx
                .fn_sig(f)
                .instantiate_identity()
                .skip_normalization()
                .output()
                .skip_binder()
                .has_param()
            {
                self.found.open.extend(held(recognition, result).keys);
            }
            return;
        }
        if !f.is_local() && recognition.in_sysroot(f) {
            if crate::lower::recognition::gives_what_it_isnt_given(tcx, f) {
                self.found.given_in(recognition, result);
            }
            return;
        }
        for &ty in inputs.iter().chain([&result]) {
            self.found.given_in(recognition, ty);
        }
    }
}

impl<'tcx> Visitor<'tcx> for Made<'_, '_, 'tcx> {
    fn visit_expr(&mut self, expr: &'tcx hir::Expr<'tcx>) {
        let tcx = self.recognition.tcx;
        let results = self.results;
        let mut from = results.expr_ty(expr);
        for adjustment in results.expr_adjustments(expr) {
            match adjustment.kind {
                Adjust::Pointer(PointerCoercion::ReifyFnPointer(_)) => match *from.kind() {
                    ty::FnDef(f, args) => self.found.made(tcx, adjustment.target, f, args.skip_binder()),
                    _ => self.found.given_in(self.recognition, adjustment.target),
                },
                Adjust::Pointer(PointerCoercion::ClosureFnPointer(_)) => match *from.kind() {
                    ty::Closure(closure, args) => self.found.made(tcx, adjustment.target, closure, args),
                    _ => self.found.given_in(self.recognition, adjustment.target),
                },
                // A safe one's functions, as an unsafe one: not followed.
                Adjust::Pointer(PointerCoercion::UnsafeFnPointer) => {
                    self.found.given_in(self.recognition, adjustment.target)
                }
                // A library's `Deref`, which may give what it holds nowhere.
                Adjust::Deref(DerefAdjustKind::Overloaded(_))
                    if let ty::Adt(adt, _) = from.peel_refs().kind()
                        && !adt.did().is_local()
                        && !self.recognition.in_sysroot(adt.did()) =>
                {
                    self.found.given_in(self.recognition, adjustment.target)
                }
                _ => {}
            }
            from = adjustment.target;
        }
        match expr.kind {
            hir::ExprKind::Call(callee, args) => {
                if let ty::FnDef(f, _) = *results.expr_ty_adjusted(callee).kind() {
                    let inputs: Vec<Ty<'tcx>> = args.iter().map(|arg| results.expr_ty_adjusted(arg)).collect();
                    self.call(f, results.expr_ty(expr), &inputs);
                }
            }
            hir::ExprKind::MethodCall(_, receiver, args, _) => {
                if let Some(f) = results.type_dependent_def_id(expr.hir_id) {
                    let inputs: Vec<Ty<'tcx>> = (std::iter::once(receiver).chain(args))
                        .map(|arg| results.expr_ty_adjusted(arg))
                        .collect();
                    self.call(f, results.expr_ty(expr), &inputs);
                }
            }
            // An operator of a library's type's impl.
            hir::ExprKind::Binary(_, a, b) | hir::ExprKind::AssignOp(_, a, b) | hir::ExprKind::Index(a, b, _) => {
                if let Some(f) = results.type_dependent_def_id(expr.hir_id) {
                    self.call(
                        f,
                        results.expr_ty(expr),
                        &[results.expr_ty_adjusted(a), results.expr_ty_adjusted(b)],
                    );
                }
            }
            hir::ExprKind::Unary(_, a) => {
                if let Some(f) = results.type_dependent_def_id(expr.hir_id) {
                    self.call(f, results.expr_ty(expr), &[results.expr_ty_adjusted(a)]);
                }
            }
            // A library's static or constant.
            hir::ExprKind::Path(ref qpath) => {
                if let Res::Def(DefKind::Static { .. } | DefKind::Const { .. } | DefKind::AssocConst { .. }, id) =
                    results.qpath_res(qpath, expr.hir_id)
                    && !id.is_local()
                {
                    self.found.given_in(self.recognition, results.expr_ty(expr));
                }
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
    }
}

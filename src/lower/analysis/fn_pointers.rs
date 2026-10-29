//! What a fn pointer may be (ADR 0907): of each fn pointer type, the
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
use rustc_middle::ty::{self, Ty, TyCtxt, TypeFoldable, TypeFolder, TypeVisitableExt, TypeckResults};
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
    /// The types whose values may be a library's own Rust functions, read
    /// of a field its contract says holds only those (ADR 0330, amended):
    /// what they run is the library's, not followed, but never JS's.
    library_rust: HashSet<Ty<'tcx>>,
    /// The crate's structs' fields a consumer may read whose fn pointers
    /// may only be Rust functions (`rust_only`), its contract's to say.
    rust_fields: Vec<DefId>,
    /// Whether its code may hold a function JS made, or one
    /// `js::function::with_this` makes, either of which may read its `this`:
    /// it calls a binding's or a library's function, which may give one, or
    /// JS may call one of its functions with a function value (#402).
    js_functions: bool,
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
                found.js_functions |= (sig.inputs().iter()).any(|ty| {
                    (ty.walk().filter_map(|part| part.as_type()))
                        .any(|part| matches!(part.kind(), ty::Dynamic(..) | ty::Param(_) | ty::FnPtr(..)))
                });
            }
            let mut made = Made {
                recognition,
                results: tcx.typeck(owner),
                found: &mut found,
            };
            made.visit_body(tcx.hir_body_owned_by(owner));
        }
        found.rust_fields = found.fields_of_rust(recognition);
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
        if self.all_open || self.open.contains(&key) || self.library_rust.contains(&key) {
            return None;
        }
        Some(self.made.get(&key).map_or(&[], Vec::as_slice))
    }

    /// Whether a value of `ptr`, a fn pointer type, may only be a Rust
    /// function, which never counts its arguments: each the crate makes
    /// one of is its own function or closure, none a binding's, and any
    /// other may only be a library's own, of a field its contract says
    /// holds only those (ADR 0330, amended). A call of it is one of
    /// rust-js's own.
    pub(in crate::lower) fn rust_only(&self, tcx: TyCtxt<'tcx>, ptr: Ty<'tcx>) -> bool {
        let Some(key) = key(tcx, ptr) else { return false };
        !self.all_open
            && !self.open.contains(&key)
            && (self.made.get(&key).into_iter().flatten()).all(|&(f, _)| f.is_local() && !is_binding(tcx, f))
    }

    /// Whether a function its code calls may read its `this`: one JS made,
    /// or `js::function::with_this`'s, a binding's (`js_functions`). Where
    /// none may, a call through a field, `s.f()`, gives it nothing it reads
    /// (#402).
    pub(in crate::lower) fn may_read_this(&self) -> bool {
        self.js_functions
    }

    /// What its contract says of its structs' fields (ADR 0330, amended):
    /// each one a consumer may read whose fn pointers may only be Rust
    /// functions (`rust_only`), which the consumer reads as the library's
    /// own (`FieldFact::RustFunctions`).
    pub(in crate::lower) fn rust_fields(&self) -> impl Iterator<Item = (DefId, crate::library::FieldFact)> + '_ {
        (self.rust_fields.iter()).map(|&field| (field, crate::library::FieldFact::RustFunctions))
    }

    /// Of the crate's structs a consumer may reach, each field that holds a
    /// fn pointer, each of which may only be a Rust function.
    fn fields_of_rust(&self, recognition: &Recognition<'_, 'tcx>) -> Vec<DefId> {
        let tcx = recognition.tcx;
        let mut fields = Vec::new();
        for id in tcx.hir_crate_items(()).definitions() {
            if !matches!(tcx.def_kind(id), DefKind::Struct) || !crate::lower::library::reachable(tcx, id.to_def_id()) {
                continue;
            }
            let adt = tcx.adt_def(id);
            for field in adt.all_fields() {
                let held = held(
                    recognition,
                    tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                );
                if !held.any
                    && !held.keys.is_empty()
                    && held.rust.is_empty()
                    && held.keys.iter().all(|&key| self.rust_only(tcx, key))
                {
                    fields.push(field.did);
                }
            }
        }
        fields
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
        self.library_rust.extend(held.rust);
    }
}

/// A fn pointer type as each of its values is, whatever its lifetimes,
/// those its signature binds too, a `dyn Fn`'s and a nested fn pointer's:
/// `for<'a> fn(&'a u32)` and `fn(&'static u32)` are one, and so are
/// `fn(&dyn for<'a> Fn(&'a u32))` and `fn(&dyn Fn(&'static u32))`, which
/// subtyping may make one of the other. `None` of one that's not a fn
/// pointer.
fn key<'tcx>(tcx: TyCtxt<'tcx>, ptr: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::FnPtr(..) = ptr.kind() else { return None };
    Some(ptr.fold_with(&mut Lifetimeless(tcx)))
}

/// A type with no lifetime, bound or free, and no binder of any.
struct Lifetimeless<'tcx>(TyCtxt<'tcx>);

impl<'tcx> TypeFolder<TyCtxt<'tcx>> for Lifetimeless<'tcx> {
    fn cx(&self) -> TyCtxt<'tcx> {
        self.0
    }

    fn fold_binder<T: TypeFoldable<TyCtxt<'tcx>>>(&mut self, t: ty::Binder<'tcx, T>) -> ty::Binder<'tcx, T> {
        ty::Binder::dummy(t.skip_binder().fold_with(self))
    }

    fn fold_region(&mut self, _: ty::Region<'tcx>) -> ty::Region<'tcx> {
        self.0.lifetimes.re_erased
    }
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
    /// Those held only by a library's field its contract says holds only
    /// its own Rust functions (`FieldFact::RustFunctions`).
    rust: HashSet<Ty<'tcx>>,
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
                // What its signature holds, the walk goes on to.
                ty::FnPtr(..) => {
                    if let Some(key) = key(tcx, part) {
                        held.keys.insert(key);
                    }
                }
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
                        let ty = field.ty(tcx, args).skip_normalization();
                        // A library's field of its own Rust functions alone.
                        if recognition
                            .foreign
                            .field_has(field.did, crate::library::FieldFact::RustFunctions)
                        {
                            let mut theirs = Held::default();
                            walk(recognition, ty, seen, &mut theirs);
                            held.rust.extend(theirs.keys.into_iter().chain(theirs.rust));
                            held.any |= theirs.any;
                            continue;
                        }
                        walk(recognition, ty, seen, held);
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
        // A binding's or a library's, which may give a JS function.
        self.found.js_functions = true;
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

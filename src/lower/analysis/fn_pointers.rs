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
//!
//! And, of the same calls, which structs code that isn't the crate's may
//! make a value of (rd204, the user's decision): a JS object the program
//! didn't build, `js::cast`'s, a binding's or a library's result, or what
//! JS calls a function it's given with, whose properties a getter may
//! answer, each read as often as Rust reads it (ADR 1022).

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
    /// What each function `js::function::with_this` makes in its code is
    /// called with and gives, `(A, R)` of `with_this::<F, A, R>`, its
    /// lifetimes erased: the one kind of function that reads a `this` Rust
    /// gives, `None`, where a call through a field gives none (#402, the
    /// user's decision, #404).
    with_this: HashSet<(Ty<'tcx>, Ty<'tcx>)>,
    /// Whether one is made in generic code, of what its callers give.
    with_this_generic: bool,
    /// What code that isn't the crate's gives, `made_outside`'s, each a type
    /// a value of which may be a JS object, or hold one.
    outside: Vec<Ty<'tcx>>,
    /// Each cast of a value the program has, `js::cast(Some(js::unknown(x)))`,
    /// of the type it's of and the type it's cast to: what it's cast to is
    /// that value, a JS object only where a value of that type may be one
    /// (rd221).
    recast: Vec<(Ty<'tcx>, Ty<'tcx>)>,
    /// The structs a value code that isn't the crate's makes may be, or
    /// hold, through what it's made of, and whether one of a type rustc
    /// can't normalize may be any (`JsMade`).
    js_made: JsMade,
    /// The structs the crate's code makes a value of, a literal's: the
    /// program's own data, which a JS object of its type
    /// is taken to be, as a value given back through `Unknown` is (rd221;
    /// ADR 1022, amended).
    built: HashSet<DefId>,
    /// Each field of a `dyn Fn` of the crate's structs, and whether each
    /// write of it the crate makes, a literal's or an assignment's, is of
    /// a closure or a function of its own, `Box::new(move || ..)`, never
    /// a with_this function's, and its place never lent mutably (#405).
    own_fields: HashMap<DefId, bool>,
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
        found.rust_fields = found.fields_of_rust(recognition);
        // A struct the program makes values of is its own data, whatever JS
        // gives of its type, and so is what it holds: not one of a binding
        // crate's, a dictionary, which only JS's objects are where a binding
        // gives one (rd221).
        let built = std::mem::take(&mut found.built);
        let own = |adt: DefId| {
            (adt.is_local() || recognition.foreign.in_library(adt))
                && (built.contains(&adt) || recognition.foreign.built(adt))
        };
        // What a cast of the program's value is cast to is a JS object where
        // that value may be one, until nothing more may be.
        let mut made = JsMade::default();
        let mut outside = std::mem::take(&mut found.outside);
        let mut recast = std::mem::take(&mut found.recast);
        while !outside.is_empty() {
            for ty in outside.drain(..) {
                made.walk(recognition, &own, ty, &mut HashSet::new());
            }
            let (js, program): (Vec<_>, Vec<_>) = recast.into_iter().partition(|&(of, _)| {
                let mut of_it = JsMade::default();
                of_it.walk(recognition, &own, of, &mut HashSet::new());
                of_it.any || of_it.structs.iter().any(|adt| made.structs.contains(adt))
            });
            recast = program;
            outside = js.into_iter().map(|(_, to)| to).collect();
        }
        found.js_made = made;
        found.built = built;
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

    /// Whether a function called with `args`, a tuple, giving `output`,
    /// may be one `js::function::with_this` makes in the crate's code, of
    /// that signature, which reads the `this` Rust gives it, `None`: a call
    /// of one through a field, `(s.f)()`, gives it none, `const f = s.f;
    /// f()`. Another, JS's own, is called through its object, `s.f()`, as
    /// JS calls it (#402, the user's decision, #404).
    pub(in crate::lower) fn may_read_this(
        &self,
        tcx: TyCtxt<'tcx>,
        (args, output): (Ty<'tcx>, Ty<'tcx>),
        field: Option<DefId>,
    ) -> bool {
        // A field the crate fills with its own closures alone (#405).
        if field.is_some_and(|field| self.own_fields.get(&field) == Some(&true)) {
            return false;
        }
        self.with_this_generic
            || self.with_this.contains(&(
                tcx.erase_and_anonymize_regions(args),
                tcx.erase_and_anonymize_regions(output),
            ))
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

    /// Whether a value of the struct `adt` may be a JS object the program
    /// didn't build, whose properties a getter may answer, so each read of
    /// a field of it is JS's, as often as Rust reads it (rd204, the user's
    /// decision; ADR 1022): one code that isn't the crate's made, `js::cast`'s
    /// or a binding's result.
    pub(in crate::lower) fn js_may_make(&self, adt: DefId) -> bool {
        self.js_made.structs.contains(&adt) || (self.js_made.any && !self.built.contains(&adt))
    }

    /// The structs the crate's code makes a value of (`built`), which its
    /// contract says, so a consumer takes them as the program's own data,
    /// as the crate does (rd221; ADR 1022, amended).
    pub(in crate::lower) fn built(&self) -> impl Iterator<Item = DefId> + '_ {
        self.built.iter().copied()
    }

    /// A value of `ty` code that isn't the crate's made: each struct it is
    /// or holds, through its fields and what a function it holds gives, may
    /// be a JS object. What such a function is called with is the crate's,
    /// and so is what JS calls a function of the crate's with: its own
    /// objects, given back, a bound function's or a state's (ADR 1022).
    fn made_outside(&mut self, ty: Ty<'tcx>) {
        self.outside.push(ty);
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

/// The structs a value code that isn't the crate's made may be or hold,
/// and whether a value of any may be, one of a type rustc can't normalize
/// (rd204, ADR 1022).
#[derive(Default)]
struct JsMade {
    structs: HashSet<DefId>,
    any: bool,
}

impl JsMade {
    /// Each struct a value of `ty` code that isn't the crate's made may be or
    /// hold, past a struct that's the program's own data (`own`), whose
    /// values are the program's whatever their type says they hold.
    fn walk<'tcx>(
        &mut self,
        recognition: &Recognition<'_, 'tcx>,
        own: &dyn Fn(DefId) -> bool,
        ty: Ty<'tcx>,
        seen: &mut HashSet<Ty<'tcx>>,
    ) {
        let tcx = recognition.tcx;
        if !seen.insert(ty) {
            return;
        }
        // What a pointer points at, as a reference's.
        if let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, ty) {
            return self.walk(recognition, own, pointee, seen);
        }
        match *ty.kind() {
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => self.walk(recognition, own, inner, seen),
            ty::Tuple(parts) => {
                for part in parts {
                    self.walk(recognition, own, part, seen);
                }
            }
            ty::Adt(adt, args) => {
                // Std's types hold what their arguments say; another's, its
                // fields.
                let std = !adt.did().is_local() && recognition.in_sysroot(adt.did());
                if adt.is_struct() && !std {
                    if own(adt.did()) {
                        return;
                    }
                    self.structs.insert(adt.did());
                }
                match std {
                    true => {
                        for part in args.types() {
                            self.walk(recognition, own, part, seen);
                        }
                    }
                    false => {
                        for field in adt.all_fields() {
                            self.walk(recognition, own, field.ty(tcx, args).skip_normalization(), seen);
                        }
                    }
                }
            }
            // What a function gives.
            ty::FnPtr(..) => {
                let sig = tcx.instantiate_bound_regions_with_erased(ty.fn_sig(tcx));
                self.walk(recognition, own, sig.output(), seen);
            }
            // A `dyn Fn(A) -> R`'s `R`; another trait's arguments, what it
            // may give.
            ty::Dynamic(predicates, ..) => {
                for predicate in predicates.iter() {
                    match predicate.skip_binder() {
                        ty::ExistentialPredicate::Trait(t) if !tcx.is_fn_trait(t.def_id) => {
                            for part in t.args.types() {
                                self.walk(recognition, own, part, seen);
                            }
                        }
                        ty::ExistentialPredicate::Projection(p) => {
                            if let Some(part) = p.term.as_type() {
                                self.walk(recognition, own, part, seen);
                            }
                        }
                        _ => {}
                    }
                }
            }
            ty::Alias(..) if !ty.has_param() => match tcx
                .try_normalize_erasing_regions(ty::TypingEnv::fully_monomorphized(), ty::Unnormalized::new_wip(ty))
            {
                Ok(normal) if normal != ty => self.walk(recognition, own, normal, seen),
                _ => self.any = true,
            },
            _ => {}
        }
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
        for &ty in inputs.iter().chain([&result]) {
            self.found.given_in(recognition, ty);
        }
        // What a binding gives back is the program's (`#[rust_js::gives_back]`);
        // what serde gives, a struct its `Deserialize` builds, is the
        // program's own data (`built`).
        if !crate::lower::bindings::gives_back(tcx, f) {
            self.found.made_outside(result);
        }
    }
}

impl<'tcx> Made<'_, '_, 'tcx> {
    /// Of a call of `f`, a binding that gives what it's given, `this` itself,
    /// `js::cast`'s and `js::unknown`'s, of `args`: the type of the value the
    /// program has that it's given, through `Some(..)`, a borrow, an `Rc`'s
    /// clone and casts, `js::cast(Some(js::unknown_of(Rc::clone(queue))))`
    /// of an `Rc<UpdateQueue>`, a struct of the program's (rd221). `None` of
    /// another call, or of what may be anything, a `&Unknown`.
    fn recast_of(&self, f: DefId, args: &'tcx [hir::Expr<'tcx>]) -> Option<Ty<'tcx>> {
        let tcx = self.recognition.tcx;
        let this = |f: DefId| {
            is_binding(tcx, f)
                && matches!(
                    crate::lower::bindings::js_form(tcx, f),
                    crate::lower::bindings::JsForm::This
                )
        };
        if !this(f) {
            return None;
        }
        let [given] = args else { return None };
        let mut given: &'tcx hir::Expr<'tcx> = given;
        loop {
            given = match given.kind {
                hir::ExprKind::AddrOf(_, _, inner) => inner,
                hir::ExprKind::Call(callee, [inner]) => match *self.results.expr_ty(callee).kind() {
                    ty::FnDef(id, _)
                        if this(id)
                            || (tcx.trait_of_assoc(id) == tcx.lang_items().clone_trait()
                                && crate::lower::recognition::rc_pointee(
                                    tcx,
                                    self.results.expr_ty(inner).peel_refs(),
                                )
                                .is_some())
                            || matches!(tcx.def_kind(id), DefKind::Ctor(..))
                                && tcx.lang_items().option_some_variant() == Some(tcx.parent(id)) =>
                    {
                        inner
                    }
                    _ => break,
                },
                _ => break,
            };
        }
        let of = self.results.expr_ty(given);
        let mut peeled = of.peel_refs();
        while let Some(pointee) = crate::lower::recognition::rc_pointee(tcx, peeled) {
            peeled = pointee.peel_refs();
        }
        match peeled.kind() {
            ty::Adt(adt, _)
                if adt.is_struct() && (adt.did().is_local() || self.recognition.foreign.in_library(adt.did())) =>
            {
                Some(of)
            }
            _ => None,
        }
    }

    /// The field of the crate's struct `place` reads, `s.f`, of a `dyn Fn`'s.
    fn field_of(&self, place: &hir::Expr<'tcx>) -> Option<DefId> {
        let hir::ExprKind::Field(base, _) = place.kind else {
            return None;
        };
        let ty::Adt(adt, _) = self.results.expr_ty_adjusted(base).peel_refs().kind() else {
            return None;
        };
        if !adt.is_struct() {
            return None;
        }
        let field = adt
            .non_enum_variant()
            .fields
            .get(self.results.field_index(place.hir_id))?;
        Some(field.did)
    }

    /// A write of `field`, of `value`, or of what's lent mutably: kept as
    /// one of the crate's own closures where it is one, of a field of a
    /// `dyn Fn`.
    fn field_written(&mut self, field: DefId, value: Option<&'tcx hir::Expr<'tcx>>) {
        let tcx = self.recognition.tcx;
        let declared = tcx.type_of(field).instantiate_identity().skip_normalization();
        if !(declared.walk().filter_map(|part| part.as_type())).any(|part| matches!(part.kind(), ty::Dynamic(..))) {
            return;
        }
        let own = value.is_some_and(|value| self.own_function(value));
        *self.found.own_fields.entry(field).or_insert(true) &= own;
    }

    /// Whether `value` is a closure or a function of the crate's, or a box
    /// or an `Rc` of one, `Box::new(move || ..)`: never a with_this one.
    fn own_function(&self, value: &'tcx hir::Expr<'tcx>) -> bool {
        let tcx = self.recognition.tcx;
        let value = match value.kind {
            hir::ExprKind::AddrOf(_, _, inner) => inner,
            _ => value,
        };
        if matches!(self.results.expr_ty(value).kind(), ty::Closure(..) | ty::FnDef(..)) {
            return true;
        }
        let made = |id: DefId| {
            crate::lower::recognition::is_rc_new(tcx, id)
                || crate::lower::recognition::is_std_def(tcx, id, crate::lower::recognition::StdItem::BoxNew)
        };
        matches!(value.kind, hir::ExprKind::Call(callee, [inner])
            if matches!(*self.results.expr_ty(callee).kind(), ty::FnDef(id, _) if made(id))
                && self.own_function(inner))
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
                    self.found.given_in(self.recognition, adjustment.target);
                    self.found.made_outside(adjustment.target);
                }
                _ => {}
            }
            from = adjustment.target;
        }
        match expr.kind {
            hir::ExprKind::Call(callee, args) => {
                if let ty::FnDef(f, generic_args) = *results.expr_ty_adjusted(callee).kind() {
                    // A function `with_this` makes: what it's called with
                    // and gives, `A` and `R` of `with_this::<F, A, R>`.
                    if matches!(
                        crate::lower::bindings::js_form(tcx, f),
                        crate::lower::bindings::JsForm::WithThis
                    ) && let [_, called, gives] = generic_args.skip_binder().types().collect::<Vec<_>>()[..]
                    {
                        let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
                        match called.has_param() || gives.has_param() {
                            true => self.found.with_this_generic = true,
                            false => _ = self.found.with_this.insert((erased(called), erased(gives))),
                        }
                    }
                    let inputs: Vec<Ty<'tcx>> = args.iter().map(|arg| results.expr_ty_adjusted(arg)).collect();
                    // A cast of a value the program has is that value (rd221).
                    match self.recast_of(f, args) {
                        Some(of) => {
                            for &ty in inputs.iter().chain([&results.expr_ty(expr)]) {
                                self.found.given_in(self.recognition, ty);
                            }
                            self.found.recast.push((of, results.expr_ty(expr)));
                        }
                        None => self.call(f, results.expr_ty(expr), &inputs),
                    }
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
            // What a field of a `dyn Fn` is written of: a literal's, an
            // assignment's, or lent mutably, any.
            hir::ExprKind::Struct(_, fields, _)
                if let ty::Adt(adt, _) = results.expr_ty(expr).kind()
                    && adt.is_struct() =>
            {
                self.found.built.insert(adt.did());
                for field in fields.iter() {
                    let def = adt.non_enum_variant().fields[results.field_index(field.hir_id)].did;
                    self.field_written(def, Some(field.expr));
                }
            }
            hir::ExprKind::Assign(place, value, _) if let Some(def) = self.field_of(place) => {
                self.field_written(def, Some(value));
            }
            hir::ExprKind::AddrOf(_, rustc_hir::Mutability::Mut, place) if let Some(def) = self.field_of(place) => {
                self.field_written(def, None);
            }
            // A library's static or constant.
            hir::ExprKind::Path(ref qpath) => {
                if let Res::Def(DefKind::Static { .. } | DefKind::Const { .. } | DefKind::AssocConst { .. }, id) =
                    results.qpath_res(qpath, expr.hir_id)
                    && !id.is_local()
                {
                    self.found.given_in(self.recognition, results.expr_ty(expr));
                    self.found.made_outside(results.expr_ty(expr));
                }
            }
            _ => {}
        }
        intravisit::walk_expr(self, expr);
    }
}

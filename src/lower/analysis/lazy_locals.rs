//! Which thread-locals are made where they're first used, as Rust makes
//! each (ADR 0037, amended): those whose initializer runs code of the
//! crate's that could tell when it runs, prints, may panic, or reads what
//! may change after its module loads. One that runs only what std's and
//! bindings' calls make, or what a function of the crate's that only makes
//! a value makes, React's `createCursor(null)`, is made as its module
//! loads, as JS writes it, `createContext([])`: the guide's semantics list
//! that difference. Of those, one whose maker is of a module that may load
//! after its own is made where it's first used too, which the pipeline
//! finds of the modules' imports (`made_by`).

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::TerminatorKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::fn_pointers::FnPointers;

use crate::lower::Body;
use crate::lower::recognition::{StdItem, is_std_def};

/// The thread-locals made where they're first used, and of those made as
/// their modules load, the crate's functions each one's initializer has
/// make its value (`makes_only`), by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    pub(in crate::lower) made_by: HashMap<LocalDefId, Vec<DefId>>,
}

/// The thread-locals, by their keys, whose initializer (`inits`, each
/// initializer's key) runs the crate's code, may panic, or reads what may
/// change after its module loads; a fn pointer's call runs what `pointers`
/// says it may be (ADR 0907). And those made so already,
/// `made_where_used`, which a lowering of the crate found make their value
/// by a function of a module that may load after theirs.
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
    // Each made as its module loads so far, what it reads of other
    // thread-locals, and its makers.
    let mut at_load = Vec::new();
    for body in bodies {
        let Some(init) = body.def_id.as_local() else { continue };
        let Some(&key) = inits.get(&init) else { continue };
        let mut initializer = Initializer {
            tcx,
            pointers,
            makers: Makers {
                tcx,
                bodies: &by_id,
                asked: Vec::new(),
                found: Vec::new(),
            },
            read: Vec::new(),
        };
        if made_where_used.contains(&key) || initializer.runs_the_crates(body) {
            found.lazy.insert(key);
        } else {
            at_load.push((key, initializer.read, initializer.makers.found));
        }
    }
    // One that reads another is made as its module loads only where that
    // one is made before it and holds what can't change after
    // (`made_ahead`): it reads then what Rust's reads where it's first used
    // (#376). As one is made where it's first used, so is each that reads it.
    loop {
        let lazy: Vec<LocalDefId> = (at_load.iter())
            .filter(|(key, read, _)| !read.iter().all(|&read| made_ahead(tcx, read, *key, &found.lazy)))
            .map(|&(key, ..)| key)
            .collect();
        if lazy.is_empty() {
            break;
        }
        at_load.retain(|(key, ..)| !lazy.contains(key));
        found.lazy.extend(lazy);
    }
    for (key, _, makers) in at_load {
        if !makers.is_empty() {
            found.made_by.insert(key, makers);
        }
    }
    found
}

/// Whether `read`, a thread-local `key`'s initializer reads, is made as its
/// module loads, not one of `lazy`, ahead of `key`, in its module and before
/// it, and holds what can't change after (`unchanging`), React's scheduler's
/// `localSetImmediate`: whenever `key`'s initializer runs, it reads the
/// same of it. A library's isn't asked, and is taken to change.
fn made_ahead(tcx: TyCtxt<'_>, read: DefId, key: LocalDefId, lazy: &HashSet<LocalDefId>) -> bool {
    let Some(read) = read.as_local() else { return false };
    let at = |key: LocalDefId| tcx.def_ident_span(key).unwrap_or_else(|| tcx.def_span(key));
    let ty = tcx.type_of(read).instantiate_identity().skip_normalization();
    !lazy.contains(&read)
        && tcx.parent_module_from_def_id(read) == tcx.parent_module_from_def_id(key)
        && at(read).lo() < at(key).lo()
        && matches!(ty.kind(), ty::Adt(_, held) if unchanging(tcx, held.type_at(0), &mut Vec::new()))
}

/// Whether a value of `ty` can't change once made: of numbers, text,
/// functions and fn pointers, by shared references, tuples, arrays, and
/// std's `Box`, `Vec`, `String`, `Rc` and `Arc` of those, and types of no
/// cell whose fields are; not a cell, an atomic, a raw pointer, a `dyn`,
/// which may hold a cell, nor a binding's JS value, which JS may change.
/// `seen`, the types being asked, which hold themselves.
fn unchanging<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>, seen: &mut Vec<ty::Ty<'tcx>>) -> bool {
    if seen.contains(&ty) {
        return true;
    }
    seen.push(ty);
    let unchanging = match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => true,
        ty::FnDef(..) | ty::FnPtr(..) => true,
        ty::Ref(_, inner, rustc_hir::Mutability::Not) | ty::Array(inner, _) | ty::Slice(inner) => {
            unchanging(tcx, *inner, seen)
        }
        ty::Tuple(parts) => parts.iter().all(|part| unchanging(tcx, part, seen)),
        ty::Adt(adt, args)
            if ty.is_box()
                || Some(adt.did()) == tcx.lang_items().string()
                || [StdItem::Vec, StdItem::Rc, StdItem::Arc]
                    .iter()
                    .any(|&item| is_std_def(tcx, adt.did(), item)) =>
        {
            args.types().all(|arg| unchanging(tcx, arg, seen))
        }
        ty::Adt(adt, args) => {
            ty.is_freeze(tcx, ty::TypingEnv::fully_monomorphized())
                && !crate::lower::bindings::is_binding(tcx, adt.did())
                && adt
                    .all_fields()
                    .all(|field| unchanging(tcx, field.ty(tcx, args).skip_normalization(), seen))
        }
        _ => false,
    };
    seen.pop();
    unchanging
}

/// What a thread-local's initializer is asked of what it runs: what it calls
/// of the crate's that only makes a value (`makers`), and the thread-locals
/// it reads, by their keys, `read`.
struct Initializer<'b, 'tcx> {
    tcx: TyCtxt<'tcx>,
    pointers: &'b FnPointers<'tcx>,
    makers: Makers<'b, 'tcx>,
    read: Vec<DefId>,
}

impl<'tcx> Initializer<'_, 'tcx> {
    /// Whether `body`, an initializer's or a closure's it gives std, runs
    /// code of the crate's, may panic, or reads a `static` that may change
    /// after its module loads (#376): a call of a body of the crate's, a
    /// function, a method of its impl or a closure, but a function that
    /// only makes a value (`Makers`, which keeps it), or of what rust-js
    /// can't see the body of; std's print; std's call given the crate's
    /// function or closure, which it may call (`given_run`); a fn pointer's
    /// call of what it may be (`pointers`), where any is the crate's or it
    /// may be anything; a drop of a value of the crate's with a destructor;
    /// a check that may fail; or a call that never returns, a panic's. What
    /// it reads of other thread-locals is kept, `read`.
    fn runs_the_crates(&mut self, body: &Body<'tcx>) -> bool {
        let tcx = self.tcx;
        let (_, changing, keys) = reads(tcx, body);
        self.read.extend(keys);
        if changing {
            return true;
        }
        let Some(mir) = &body.mir else { return true };
        let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
        let body = &mir.body;
        body.basic_blocks.iter().any(|data| match &data.terminator().kind {
            TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => true,
            TerminatorKind::Call { func, .. } => {
                let ty = func.ty(&body.local_decls, tcx);
                match crate::lower::fn_def(ty) {
                    Some((def_id, args)) => self.runs(typing_env, def_id, args),
                    // A fn pointer's runs what it may be (ADR 0907): of a
                    // binding's alone, `js::date::now`, none of the crate's.
                    None if ty.is_fn_ptr() => (self.pointers.may_be(tcx, ty))
                        .is_none_or(|may_be| may_be.iter().any(|&(def_id, args)| self.runs(typing_env, def_id, args))),
                    // A closure's or a `dyn Fn`'s, which may be the crate's.
                    None => true,
                }
            }
            // A drop of a value of the crate's that has a destructor runs it.
            TerminatorKind::Drop { place, .. } => drops_the_crates(tcx, place.ty(&body.local_decls, tcx).ty),
            _ => false,
        })
    }

    /// Whether calling `def_id`, of `args`, runs the crate's code.
    fn runs(&mut self, typing_env: ty::TypingEnv<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
        let tcx = self.tcx;
        if tcx.is_closure_like(def_id) {
            return def_id.is_local();
        }
        if crate::lower::bindings::is_binding(tcx, def_id) {
            return false;
        }
        match crate::lower::items::resolve_instance_in(tcx, typing_env, def_id, args) {
            Ok(Some(instance)) => match instance.def {
                ty::InstanceKind::Item(id) if id.is_local() => !self.makers.makes_only(id),
                ty::InstanceKind::Item(id) => {
                    crate::lower::recognition::print_of(tcx, id).is_some()
                        || (!std_makes_only(tcx, id) && self.given_run(instance.args))
                }
                // A shim of std's, a clone's or a drop's, runs what it's
                // of: the crate's where that is.
                _ => (instance.args.types()).any(|ty| ty.ty_adt_def().is_some_and(|adt| adt.did().is_local())),
            },
            _ => true,
        }
    }

    /// Whether what std's call is given of the crate's, a closure or a
    /// function among its type's `args`, runs the crate's code where std
    /// calls it, as a call of it there would: `Some(2).map(|n| { .. })`'s
    /// closure that prints, not `TRIES.with(|t| *t)`'s, which only reads
    /// what it's lent.
    fn given_run(&mut self, args: ty::GenericArgsRef<'tcx>) -> bool {
        let tcx = self.tcx;
        let given: Vec<DefId> = (args.types())
            .flat_map(|ty| ty.walk())
            .filter_map(|part| part.as_type())
            .filter_map(|ty| match *ty.kind() {
                ty::Closure(id, _) | ty::FnDef(id, _) if id.is_local() => Some(id),
                _ => None,
            })
            .collect();
        given.into_iter().any(|id| match tcx.is_closure_like(id) {
            true => (self.makers.bodies.get(&id).copied()).is_none_or(|body| self.runs_the_crates(body)),
            false => !self.makers.makes_only(id),
        })
    }
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
        let Some(&found) = self.bodies.get(&id) else {
            return false;
        };
        let Some(mir) = &found.mir else { return false };
        let body = &mir.body;
        let typing_env = ty::TypingEnv::post_analysis(tcx, id);
        if reads(tcx, found).0 {
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

/// What `body` reads of a `static` or a thread-local: whether it reads any;
/// whether one may change after its module loads, a `static mut`, or a
/// `static` of a cell, an atomic, but not one that can't, `BASE`, or a
/// binding's, React's `__DEV__`; and each thread-local it reads, by its
/// key, `TRIES` of `TRIES.get()`. Its closures are bodies of their own.
fn reads<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> (bool, bool, Vec<DefId>) {
    let (mut any, mut changing, mut keys) = (false, false, Vec::new());
    for expr in body.thir.exprs.iter() {
        match expr.kind {
            ExprKind::StaticRef { def_id, .. } => {
                let ty = tcx.type_of(def_id).instantiate_identity().skip_normalization();
                any = true;
                changing |= tcx.is_mutable_static(def_id) || !ty.is_freeze(tcx, ty::TypingEnv::fully_monomorphized());
            }
            ExprKind::ThreadLocalRef(_) => (any, changing) = (true, true),
            ExprKind::NamedConst { def_id, .. } if matches!(expr.ty.kind(), ty::Adt(key, _) if is_std_def(tcx, key.did(), StdItem::LocalKey)) =>
            {
                any = true;
                keys.push(def_id);
            }
            _ => {}
        }
    }
    (any, changing, keys)
}

/// Whether `id`, std's, only makes a value of what it's given: a `const
/// fn` of no trait, asking nothing of its type parameters but their size,
/// `RefCell::new`, `Vec::new`; not a `const` trait's method, `Iterator`'s,
/// nor one bound by a trait, whose impl may be the crate's, or a closure.
fn std_makes_only(tcx: TyCtxt<'_>, id: DefId) -> bool {
    // `Box::new(x)`, which is `x` (ADR 0069), React's
    // `createChildReconciler(true)`'s boxed closure.
    if crate::lower::recognition::pure_std(tcx, id) == Some(crate::lower::recognition::PureStd::BoxNew) {
        return true;
    }
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

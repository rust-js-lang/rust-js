//! Whether a call may keep a raw pointer to what a `&mut` it's given refers
//! to, past its call: the one answer (#1781, #1947), which lowering asks of
//! a `&mut` it gives generic code (`mir::assign::boxed_args`) and the MIR
//! intervals of a `Vec` they track given to the crate's code (#1966).
//! Beside `returned`, what a function gives back of its parameters:
//! pointer provenance, one home.

use super::*;
use crate::lower::library::Foreign;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// What `pointer_may_keep` asks of the crate: whether it's a library, whose
/// traits another crate may implement, and its libraries, whose functions
/// it can't look into.
#[derive(Clone, Copy)]
pub struct Libraries<'a, 'tcx> {
    pub(in crate::lower) library: bool,
    pub(in crate::lower) foreign: &'a Foreign<'a, 'tcx>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The crate's, as `pointer_may_keep` asks them.
    pub fn libraries(&self) -> Libraries<'a, 'tcx> {
        Libraries {
            library: self.krate.library,
            foreign: self.krate.foreign,
        }
    }
}

/// Whether `fn_id` may keep a raw pointer to what its `i`th parameter, a
/// `&mut`, refers to, past its call (#1781): a raw pointer of that type
/// among the locals of its body, of a closure it makes, or of a function
/// of the crate's it calls, however many calls down; or a call it can't
/// follow, through a function pointer or a `dyn`, of a trait's method it
/// can't resolve or of a library's function not std's, given anything
/// that may carry it: a reference or a raw pointer to it, a type
/// parameter, a closure that captures one. It fails closed: what it can't
/// see may keep it. std's own functions keep no `&mut` past their call
/// but where their signature says. Its signature can't say: a
/// raw pointer has no lifetime, so `keep(n: &mut u32)` that stores
/// `n as *mut u32` away borrows nothing past its call that the signature
/// names, and its caller's box copied back is a place the pointer no
/// longer writes.
///
/// `given` is the parameter as the call instantiates it, and each
/// function walked as the call instantiates it: of generic code, `&mut
/// impl GetMut<u32>`, a raw pointer to the `u32` it gives is one to it.
pub fn pointer_may_keep<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    libraries: &Libraries<'_, 'tcx>,
    call: (DefId, ty::GenericArgsRef<'tcx>),
    given: Ty<'tcx>,
) -> bool {
    may_keep(tcx, typing_env, libraries, Start::Call(call), given, None)
}

/// Whether a call of `call` that may run code not std's may keep, past
/// it, a pointer an argument of type `given` holds (#2112 B, what
/// `analysis::Effects` asks): a closure, or a `&mut` of one, what its
/// code keeps of each `&mut` it captures (`closure_may_keep`), and of
/// what else it captures, any that may write
/// (`holds_a_writing_pointer`); and the call keeps the closure, as
/// `pointer_may_keep` says, never std's. Of another, std's keeps none but
/// a `dyn`'s, whose code can't be seen; the crate's and a library's a
/// `&mut` as `pointer_may_keep` says, and anything else, may.
pub(in crate::lower) fn argument_may_keep<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    libraries: &Libraries<'_, 'tcx>,
    call: (DefId, ty::GenericArgsRef<'tcx>),
    given: Ty<'tcx>,
    holds_a_writing_pointer: &dyn Fn(Ty<'tcx>) -> bool,
) -> bool {
    let std = crate::lower::recognition::is_std_crate(tcx, call.0);
    let inner = match *given.kind() {
        ty::Ref(_, to, _) => to,
        _ => given,
    };
    if let ty::Closure(closure, closure_args) = *inner.kind() {
        let kept_by_closure = (closure_args.as_closure().upvar_tys().iter()).any(|upvar| match *upvar.kind() {
            ty::Ref(_, _, ty::Mutability::Mut) => {
                closure_may_keep(tcx, typing_env, libraries, (closure, closure_args), upvar)
            }
            _ => holds_a_writing_pointer(upvar),
        });
        let kept_by_call = !std
            && match *given.kind() {
                ty::Ref(_, _, ty::Mutability::Mut) => pointer_may_keep(tcx, typing_env, libraries, call, given),
                _ => true,
            };
        return kept_by_closure || kept_by_call;
    }
    match *given.kind() {
        _ if std => matches!(inner.kind(), ty::Dynamic(..)),
        ty::Ref(_, _, ty::Mutability::Mut) => pointer_may_keep(tcx, typing_env, libraries, call, given),
        _ => true,
    }
}

/// `pointer_may_keep` of a call given a `&mut dyn Fn*`, `given`, whose
/// code is known to be the closure `behind`: a call of it through what
/// it's given runs that closure, which is walked as the rest is, where
/// `pointer_may_keep` can't follow a `dyn` and fails closed (#1947). It
/// says whether the call may keep a pointer to the closure; what the
/// closure itself may keep is `closure_may_keep`'s.
pub(in crate::lower) fn pointer_may_keep_behind<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    libraries: &Libraries<'_, 'tcx>,
    call: (DefId, ty::GenericArgsRef<'tcx>),
    given: Ty<'tcx>,
    behind: (DefId, ty::GenericArgsRef<'tcx>),
) -> bool {
    may_keep(tcx, typing_env, libraries, Start::Call(call), given, Some(behind))
}

/// Whether the closure `closure`, of its maker's `typing_env`, may keep a
/// raw pointer to what a `&mut` it captures, `given`, refers to, past its
/// call: its body walked as `pointer_may_keep` walks a function's, what
/// it calls too (#1947).
pub(in crate::lower) fn closure_may_keep<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    libraries: &Libraries<'_, 'tcx>,
    closure: (DefId, ty::GenericArgsRef<'tcx>),
    given: Ty<'tcx>,
) -> bool {
    may_keep(tcx, typing_env, libraries, Start::Body(closure), given, None)
}

/// Where `may_keep` starts: a call, whose bodies are as it resolves, or a
/// body itself, a closure's.
enum Start<'tcx> {
    Call((DefId, ty::GenericArgsRef<'tcx>)),
    Body((DefId, ty::GenericArgsRef<'tcx>)),
}

fn may_keep<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    libraries: &Libraries<'_, 'tcx>,
    start: Start<'tcx>,
    given: Ty<'tcx>,
    behind: Option<(DefId, ty::GenericArgsRef<'tcx>)>,
) -> bool {
    let start_env = typing_env;
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
    let Some(pointee) = given.builtin_deref(true).map(erased) else {
        return false;
    };
    // Behind a `dyn` whose code isn't known, what its methods give out
    // may be any type, the `&mut Vec<u32>` of `metamorph_get_mut(v)` from a
    // `&mut dyn GetMut<Vec<u32>>`: a raw pointer to any type may be to it,
    // and a reference to any may carry it (#2053). Not of a `dyn` of the
    // `Fn` traits whose closure is known, `behind`: what that closure may
    // keep of what it captures is `closure_may_keep`'s question, and this
    // one asks only whether the call keeps a pointer to the closure.
    let opaque = matches!(pointee.kind(), ty::Dynamic(..)) && behind.is_none();
    // A raw pointer to it, or, of generic code, to what a type parameter
    // may be.
    let pointer_to = |t: Ty<'tcx>, or_param: bool| {
        crate::lower::pointers::raw_pointee(tcx, t).is_some_and(|(to, _)| {
            erased(to) == pointee || opaque || or_param && crate::lower::generic_facts::may_be_any_type(to)
        })
    };
    let pointer_to_it = |ty: Ty<'tcx>| ty.walk().filter_map(|part| part.as_type()).any(|t| pointer_to(t, true));
    // What a call it can't follow is given that may carry the reference:
    // a type parameter its caller leaves, too.
    let carries = |ty: Ty<'tcx>| {
        ty.walk().filter_map(|part| part.as_type()).any(|t| {
            crate::lower::generic_facts::may_be_any_type(t)
                || matches!(*t.kind(), ty::Ref(_, to, _) if opaque || erased(to) == pointee)
                || pointer_to(t, false)
        })
    };
    // The bodies a call runs, each as it instantiates it and in the
    // typing environment its types are of, or `None` where it can't
    // say. A trait's method generic code calls, or one through a `dyn`,
    // runs any of the program's impls: each impl's, as its own generic
    // code, and the trait's default. A program's are all known, and a
    // library's of a trait no other crate sees; another crate may
    // implement one it does, std's among them.
    let library = libraries.library;
    let all_known =
        |trait_id: DefId| !library || trait_id.is_local() && !crate::lower::library::escapes(tcx, library, trait_id);
    // A closure, a function and a function pointer are the `Fn` traits'
    // without an impl: what they run isn't known.
    let followed = |trait_id: DefId| {
        all_known(trait_id) && !tcx.is_fn_trait(trait_id) && tcx.async_fn_trait_kind_from_def_id(trait_id).is_none()
    };
    let own = |item: DefId| {
        (
            item,
            ty::GenericArgs::identity_for_item(tcx, item),
            ty::TypingEnv::post_analysis(tcx, item),
        )
    };
    let runs = |callee: DefId, callee_args: ty::GenericArgsRef<'tcx>, typing_env: ty::TypingEnv<'tcx>| {
        match crate::lower::traits::resolve(tcx, typing_env, callee, callee_args) {
            Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(_)) => {
                Some(vec![(instance.def_id(), instance.args, typing_env)])
            }
            Ok(
                None
                | Some(ty::Instance {
                    def: ty::InstanceKind::Virtual(..),
                    ..
                }),
            ) => {
                let trait_id = (tcx.trait_of_assoc(callee)).filter(|&trait_id| followed(trait_id))?;
                let impls = tcx
                    .all_impls(trait_id)
                    .filter_map(|impl_id| tcx.impl_item_implementor_ids(impl_id).get(&callee).copied());
                let default = tcx.defaultness(callee).has_value().then_some(callee);
                Some(impls.chain(default).map(own).collect())
            }
            _ => None,
        }
    };
    // Each method of a type of the crate's: its impls', and of a trait
    // of the crate's, its defaults.
    let methods = |adt: DefId| {
        let inherent = tcx.inherent_impls(adt).iter().copied();
        let traits = crate::lower::traits::declared_impls(tcx)
            .filter(|&(_, imp)| {
                matches!(*tcx.type_of(imp).instantiate_identity().skip_normalization().kind(), ty::Adt(of, _) if of.did() == adt)
            })
            .collect::<Vec<_>>();
        let defaults = traits
            .iter()
            .filter(|(trait_id, _)| trait_id.is_local())
            .flat_map(|&(trait_id, imp)| {
                let implemented = tcx.impl_item_implementor_ids(imp);
                (tcx.associated_item_def_ids(trait_id).iter().copied())
                    .filter(move |item| !implemented.contains_key(item) && tcx.defaultness(*item).has_value())
            });
        (inherent.chain(traits.iter().map(|&(_, imp)| imp)))
            .flat_map(|imp| tcx.associated_item_def_ids(imp).iter().copied())
            .chain(defaults)
            .filter(|&item| tcx.def_kind(item) == rustc_hir::def::DefKind::AssocFn)
            .collect::<Vec<_>>()
    };
    // A function of a library rust-js compiles, not std's.
    let library = |function: DefId| !function.is_local() && libraries.foreign.is_library(function.krate);
    // std's keep no `&mut` past their call, nor the crates std is built
    // of, hashbrown's `impl Extend` for its `HashSet`: a crate rust-js
    // compiles is the program's or a library's.
    let std_own = |function: DefId| {
        crate::lower::recognition::is_std_crate(tcx, function) || !function.is_local() && !library(function)
    };
    // A function pointer, or a `dyn` of the `Fn` traits, which may be any
    // code.
    let any_code = |ty: Ty<'tcx>| {
        // A closure, walked where it's held, is code it sees: its type's
        // signature, a function pointer's, no value is.
        let mut walker = ty.walk();
        while let Some(part) = walker.next() {
            let Some(t) = part.as_type() else {
                continue;
            };
            match *t.kind() {
                ty::Closure(..) | ty::CoroutineClosure(..) | ty::Coroutine(..) => walker.skip_current_subtree(),
                ty::FnPtr(..) => return true,
                ty::Dynamic(predicates, ..)
                    if predicates.principal_def_id().is_some_and(|trait_id| {
                        tcx.is_fn_trait(trait_id) || tcx.async_fn_trait_kind_from_def_id(trait_id).is_some()
                    }) =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    };
    let mut work = match start {
        Start::Call((fn_id, generic_args)) => match runs(fn_id, generic_args, typing_env) {
            Some(bodies) => bodies,
            None => return true,
        },
        Start::Body((body_id, args)) => vec![(body_id, args, typing_env)],
    };
    // A call of the `Fn` traits through what the call is given, `(*each)(at,
    // c)` of an `each: &mut dyn FnMut`, where the code behind it is known:
    // that closure's body, in its maker's terms.
    let behind_call = |callee: DefId, first: Option<Ty<'tcx>>| {
        behind.filter(|_| {
            tcx.trait_of_assoc(callee)
                .is_some_and(|trait_id| tcx.is_fn_trait(trait_id))
                && first.and_then(|ty| ty.builtin_deref(true)).map(erased) == Some(pointee)
        })
    };
    let mut seen: IndexSet<(DefId, ty::GenericArgsRef<'tcx>)> = IndexSet::default();
    while let Some((function, args, typing_env)) = work.pop() {
        if !seen.insert((function, args)) || std_own(function) {
            continue;
        }
        if !function.is_local() || !tcx.is_mir_available(function) {
            return true;
        }
        let instantiated = |ty: Ty<'tcx>| {
            tcx.try_instantiate_and_normalize_erasing_regions(args, typing_env, ty::EarlyBinder::bind(tcx, ty))
                .ok()
        };
        let body = tcx.optimized_mir(function);
        // What it holds, and gives the functions it calls.
        let mut held: IndexSet<Ty<'tcx>> = IndexSet::default();
        for decl in &body.local_decls {
            match instantiated(decl.ty) {
                Some(ty) if !pointer_to_it(ty) => held.insert(ty),
                _ => return true,
            };
        }
        for data in body.basic_blocks.iter() {
            if let rustc_middle::mir::TerminatorKind::Call {
                func, args: given_args, ..
            } = &data.terminator().kind
            {
                for arg in given_args {
                    match instantiated(arg.node.ty(body, tcx)) {
                        Some(ty) => held.insert(ty),
                        None => return true,
                    };
                }
                if let Some((callee, _)) = func.const_fn_def()
                    && let Some((closure, closure_args)) = behind_call(
                        callee,
                        given_args.first().and_then(|arg| instantiated(arg.node.ty(body, tcx))),
                    )
                {
                    work.push((closure, closure_args, start_env));
                    continue;
                }
                let bodies = func.const_fn_def().and_then(|(callee, callee_args)| {
                    let callee_args = instantiated_args(tcx, args, typing_env, callee_args)?;
                    runs(callee, callee_args, typing_env)
                });
                // A library's function, which it can't look into, may keep
                // what it's given that may carry it, and nothing else:
                // `js::get(error, "message")` keeps no `&mut String`. And
                // std's, given it and code that may be any, a function
                // pointer or a `dyn Fn`, may give that code it: a
                // `Box<dyn FnMut>`'s `call_mut`, `once(n).for_each(f)`.
                let given = |which: &dyn Fn(Ty<'tcx>) -> bool| {
                    given_args
                        .iter()
                        .any(|arg| instantiated(arg.node.ty(body, tcx)).is_none_or(which))
                };
                let given_it = || given(&carries);
                match bodies {
                    Some(bodies) => {
                        for (callee, callee_args, callee_env) in bodies {
                            if library(callee) {
                                if given_it() {
                                    return true;
                                }
                            } else if std_own(callee) {
                                if given_it() && given(&any_code) {
                                    return true;
                                }
                            } else {
                                work.push((callee, callee_args, callee_env));
                            }
                        }
                    }
                    None if given_it() => return true,
                    None => {}
                }
            }
        }
        // What it holds runs too, where it calls it or gives it to code
        // that does, std's `for_each`, and where it's dropped: a
        // closure, a function, each method of the crate's types, a
        // `Drop` and `Display` among them.
        // A closure's type is of what it captures, not of its signature, a
        // function pointer's type no value is.
        let mut types: Vec<Ty<'tcx>> = held.into_iter().collect();
        while let Some(ty) = types.pop() {
            let mut walker = ty.walk();
            while let Some(part) = walker.next() {
                let Some(t) = part.as_type() else {
                    continue;
                };
                let captured = match *t.kind() {
                    ty::Closure(_, args) => Some(args.as_closure().tupled_upvars_ty()),
                    ty::CoroutineClosure(_, args) => Some(args.as_coroutine_closure().tupled_upvars_ty()),
                    ty::Coroutine(_, args) => Some(args.as_coroutine().tupled_upvars_ty()),
                    _ => None,
                };
                if let Some(captured) = captured {
                    walker.skip_current_subtree();
                    types.push(captured);
                }
                match *t.kind() {
                    ty::Closure(def, args) | ty::CoroutineClosure(def, args) | ty::Coroutine(def, args) => {
                        work.push((def, args, typing_env))
                    }
                    ty::FnDef(def, args) => match args.no_bound_vars().and_then(|args| runs(def, args, typing_env)) {
                        Some(bodies) => work.extend(bodies),
                        None => return true,
                    },
                    ty::Adt(adt, _) if adt.did().is_local() => work.extend(methods(adt.did()).into_iter().map(own)),
                    _ => {}
                }
            }
        }
    }
    false
}

/// `of`, generic arguments of a body `args` instantiates, as it does.
fn instantiated_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    of: ty::GenericArgsRef<'tcx>,
) -> Option<ty::GenericArgsRef<'tcx>> {
    tcx.try_instantiate_and_normalize_erasing_regions(args, typing_env, ty::EarlyBinder::bind(tcx, of))
        .ok()
}

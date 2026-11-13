//! What the crate does with its closures that JS can't see in a function:
//! whether one holding a value with a destructor is made a `dyn Fn` (ADR
//! 0521), and which are cloned apart (ADR 0522); and, as a `dyn Fn`'s, of
//! a `dyn Any`, whether a value with a destructor is made one (ADR 0691).

use crate::collections::IndexSet;
use crate::lower::generic_facts::GivenADrop;
use crate::lower::passed_on::Taker;
use crate::lower::pointers::raw_pointee;
use crate::lower::recognition::{PureStd, any_trait, given_back, is_fn_dyn, pure_std, rc_pointee};
use crate::lower::{Body, bindings};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{AdtExprBase, ExprId, ExprKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;

/// Which `dyn Fn`s drop what they hold (ADR 0521, 0626): each that may
/// take what one a function holding a value with a destructor is made
/// takes, whatever its kind, `Fn`, `FnMut` or `FnOnce`, as one upcasts to
/// another, and whatever it gives; a type parameter in it may be any type
/// (`may_be`). Of every one, `all`, where a library
/// hands the crate a `dyn Fn` whose own may drop, as its contract says. One
/// `dyn Fn` dropping no longer makes every other drop (#954, #1003).
#[derive(Default)]
pub struct FnDynDrops<'tcx> {
    pub all: bool,
    pub made: Vec<Ty<'tcx>>,
}

impl<'tcx> FnDynDrops<'tcx> {
    /// Of the coercions `made` of a function holding what drops, and
    /// whether a library's contract says its own `dyn Fn`s may, `all`.
    pub(super) fn of(tcx: TyCtxt<'tcx>, all: bool, made: &[DroppingFnDyn<'tcx>]) -> Self {
        let made = (made.iter())
            .filter_map(|made| match made.to.kind() {
                ty::Dynamic(predicates, ..) => fn_dyn_args(tcx, predicates),
                _ => None,
            })
            .collect();
        FnDynDrops { all, made }
    }

    /// Whether any `dyn Fn` drops: what a library's contract says of the
    /// crate's (ADR 0626).
    pub fn any(&self) -> bool {
        self.all || !self.made.is_empty()
    }

    /// Whether a `dyn Fn` of `predicates` may drop what it holds.
    pub fn drops(&self, tcx: TyCtxt<'tcx>, predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool {
        if self.all {
            return true;
        }
        let Some(args) = fn_dyn_args(tcx, predicates) else {
            return self.any();
        };
        (self.made.iter()).any(|&made| crate::lower::analysis::rc_counts::may_be(tcx, made, args))
    }
}

/// What a `dyn Fn` takes, the tuple of its arguments, regions erased.
pub(in crate::lower) fn fn_dyn_args<'tcx>(
    tcx: TyCtxt<'tcx>,
    predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
) -> Option<Ty<'tcx>> {
    let principal = tcx.instantiate_bound_regions_with_erased(predicates.principal()?);
    Some(tcx.erase_and_anonymize_regions(principal.args.types().next()?))
}

/// What the crate does with its closures.
// Flags, not a state: whether its `dyn Any`s drop and whether its `dyn
// Error`s do are facts of different values, each found apart.
#[derive(Default)]
pub struct ClosureFacts<'tcx> {
    /// Which `dyn Fn`s drop what they hold: those a function holding a value
    /// with a destructor is made, by their signature, or every one, where a
    /// library hands the crate one whose own may, as its contract says (ADR
    /// 0626, amended), or makes one of a type parameter's.
    pub fn_dyns_drop: FnDynDrops<'tcx>,
    /// Whether a value with a destructor is made a `dyn Any`, which then
    /// drops it: what unwound, a panic's message or what JS threw, drops
    /// nothing, nor a `dyn Any` of anything else (ADR 0691).
    pub any_dyns_drop: bool,
    /// Whether a `dyn Error` the crate has may hold a value with a
    /// destructor: one it makes of one, by an unsizing or a std conversion
    /// that boxes it, or one a library makes (ADR 0462, amended).
    pub error_dyns_drop: bool,
    /// Whether a `Box<dyn Iterator>` the crate has may own a value with a
    /// destructor: one it boxes so, or a library does (ADR 0521, #1420).
    pub iterator_dyns_drop: bool,
    /// The closures made a `dyn Fn`, each carrying its drop.
    made_fn_dyns: IndexSet<DefId>,
    /// The closures made a `dyn Fn` only to hand it to JS, which Rust never
    /// drops, so none carries its drop (ADR 0873).
    handed_fn_dyns: IndexSet<DefId>,
    /// The closures cloned, `Clone::clone`'s or a `T: Clone`'s.
    cloned: IndexSet<DefId>,
    /// Of them, those a clone of which isn't the closure itself, each made
    /// by a function of what it holds, which its clone is made by again.
    pub(super) cloned_apart: IndexSet<DefId>,
    /// The closures rustc infers `Fn` or `FnMut` made where they're given
    /// to a binding that calls what it's given once, `js::set_timeout(move
    /// || { let m = &n; .. }, ms)`'s, of a union bound (ADR 1020): each
    /// called as its `call_once` runs it, dropping what it took.
    called_once: IndexSet<DefId>,
    /// Whether the crate is a library, whose callers' closures may be.
    library: bool,
}

impl<'tcx> ClosureFacts<'tcx> {
    /// What `all_bodies` make `dyn Fn`s of, and of the types the crate
    /// clones, `cloned`, the closures, whose drop is a library's where
    /// `library`.
    pub(super) fn new(
        tcx: TyCtxt<'tcx>,
        all_bodies: &[&Body<'tcx>],
        cloned: &IndexSet<Ty<'tcx>>,
        library: bool,
    ) -> ClosureFacts<'tcx> {
        let closure_of = |ty: Ty<'tcx>| match ty.kind() {
            ty::Closure(id, _) => Some(*id),
            _ => None,
        };
        let (handed, made): (Vec<_>, Vec<_>) = (all_bodies.iter())
            .flat_map(|body| fn_dyn_sources(tcx, body))
            .partition(|&(_, _, handed, _)| handed);
        let made_fn_dyns: IndexSet<DefId> = made.into_iter().filter_map(|(_, ty, _, _)| closure_of(ty)).collect();
        let called_once = (all_bodies.iter())
            .flat_map(|body| called_once_by_js(tcx, &body.thir))
            .collect();
        let handed_fn_dyns = (handed.into_iter())
            .filter_map(|(_, ty, _, _)| closure_of(ty))
            .filter(|id| !made_fn_dyns.contains(id))
            .collect();
        let cloned = (cloned.iter())
            .flat_map(|ty| {
                ty.walk()
                    .filter_map(|part| part.as_type())
                    .filter_map(closure_of)
                    .collect::<Vec<_>>()
            })
            .collect();
        ClosureFacts {
            fn_dyns_drop: FnDynDrops::default(),
            any_dyns_drop: false,
            error_dyns_drop: false,
            iterator_dyns_drop: false,
            made_fn_dyns,
            handed_fn_dyns,
            cloned,
            cloned_apart: IndexSet::default(),
            called_once,
            library,
        }
    }

    /// Of the closures cloned, those a clone of which `shares` says isn't
    /// the closure itself: `cloned_apart`'s, to set.
    pub(super) fn find_cloned_apart(&self, shares: impl Fn(DefId) -> bool) -> IndexSet<DefId> {
        self.cloned.iter().copied().filter(|&id| !shares(id)).collect()
    }

    /// Whether a clone of the closure `closure` is the closure itself, as
    /// the crate decided of each it clones (`cloned`, ADR 0522): `None` of
    /// one it doesn't, whose clone it can't make.
    pub(in crate::lower) fn clone_is_itself(&self, closure: DefId) -> Option<bool> {
        (self.cloned.contains(&closure)).then(|| !self.cloned_apart.contains(&closure))
    }

    /// Each closure the crate clones, or, a library, gives a consumer to
    /// clone behind an `impl .. + Clone`, with whether it's cloned apart:
    /// of its own, what its library's contract says, which its consumers'
    /// clones of each ask (`Foreign::closure_cloned_apart`, #1202).
    pub(in crate::lower) fn clones(&self) -> Vec<(DefId, bool)> {
        (self.cloned.iter())
            .map(|&id| (id, self.cloned_apart.contains(&id)))
            .collect()
    }

    /// Whether the closure `closure` carries its own drop, `$dropping`'s:
    /// one its maker gives back, the type its `impl Fn` hides (ADR 0484),
    /// one made a `dyn Fn` (ADR 0521), or one cloned apart (ADR 0522),
    /// each dropped where its maker's variables it took aren't seen.
    pub(in crate::lower) fn carries_its_drop(&self, tcx: TyCtxt<'_>, closure: DefId) -> bool {
        self.made_fn_dyns.contains(&closure) || self.cloned_apart.contains(&closure) || given_back(tcx, closure)
    }

    /// Whether the closure `closure`, which rustc infers `Fn` or `FnMut`, is
    /// made where it's given to a binding that calls it once, which then
    /// drops what it took: its body is its `call_once`'s, `Mir::called_once`
    /// (ADR 1020).
    pub(in crate::lower) fn called_once(&self, closure: DefId) -> bool {
        self.called_once.contains(&closure)
    }

    /// Whether the closure `closure` is made a `dyn Fn` only to hand it to
    /// JS, which Rust then never drops (ADR 0873): it carries no drop, nor
    /// is given one where it's made a `dyn Fn`.
    pub(in crate::lower) fn handed_to_js(&self, closure: DefId) -> bool {
        self.handed_fn_dyns.contains(&closure)
    }

    /// Whether the closure `closure` is cloned apart: made by a function of
    /// what it holds, `$cloning(f, clone)`, which `$clone(f)` makes again of
    /// a clone of it (ADR 0522).
    pub(in crate::lower) fn clones_apart(&self, closure: DefId) -> bool {
        self.cloned_apart.contains(&closure)
    }

    /// Whether a function a type parameter is given may be cloned apart,
    /// so its clone is its `Clone` dictionary's, not the function itself
    /// (ADRs 0246, 0522): where the crate clones one apart, or its callers
    /// may.
    pub(in crate::lower) fn functions_clone_apart(&self) -> bool {
        self.library || !self.cloned_apart.is_empty()
    }
}

/// Whether a clone of the closure of type `closure` can be the closure
/// itself, the one JS function: one nothing could tell from it, that never
/// writes what it took, nor moves it out, an `FnOnce`'s, for what it's
/// given to to change, holds no `Cell`, and holds nothing a clone of which
/// counts or drops, as `drops` says (ADRs 0522, 0525).
pub(in crate::lower) fn clone_shares<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    closure: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let ty::Closure(id, args) = *closure.kind() else {
        return true;
    };
    let closure = args.as_closure();
    let writes = writes_what_it_took(tcx, id, args) || closure.kind() == ty::ClosureKind::FnOnce;
    // Nor one of what it owns, a `Box<Cell<i32>>`'s, which a clone copies
    // apart (#989).
    !writes
        && (closure.upvar_tys().iter())
            .all(|t| !crate::lower::recognition::owns_interior_mutability(tcx, typing_env, t) && !drops(t))
}

/// Whether the closure `id` writes what it took, by value or through a
/// `&mut`: a `FnMut` may be one only by the bound it's given to, `map(|x|
/// x * 2)`, which writes nothing. Of another crate's, whose captures are
/// its compile's to see, any that isn't `Fn`.
pub(in crate::lower) fn writes_what_it_took<'tcx>(
    tcx: TyCtxt<'tcx>,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    match id.as_local() {
        Some(local) => tcx
            .closure_captures(local)
            .iter()
            .any(|captured| match captured.info.capture_kind {
                ty::UpvarCapture::ByRef(kind) => kind != ty::BorrowKind::Immutable,
                ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse => captured.mutability == ty::Mutability::Mut,
            }),
        None => args.as_closure().kind() != ty::ClosureKind::Fn,
    }
}

/// Of each capture of `closure`, by its index among them, whether it
/// borrows a binding that holds one value for as long as it's in scope:
/// one that isn't `mut`, `self` of a method among them, read through
/// shared references alone, which nothing can give another value (ADRs
/// 0846, 0879, amended, rd515). None of a closure of another crate's.
pub(in crate::lower) fn borrowed_fixed_bindings(tcx: TyCtxt<'_>, closure: DefId) -> Vec<bool> {
    use rustc_hir::{BindingMode, ByRef, Mutability, Node, PatKind};
    let Some(local) = closure.as_local().filter(|_| tcx.is_closure_like(closure)) else {
        return Vec::new();
    };
    let fixed = |captured: &&ty::CapturedPlace<'_>| {
        let ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) = captured.info.capture_kind else {
            return false;
        };
        let root = captured.get_root_variable();
        let Node::Pat(pat) = tcx.hir_node(root) else {
            return false;
        };
        let place = &captured.place;
        matches!(pat.kind, PatKind::Binding(BindingMode(ByRef::No, Mutability::Not), ..))
            && (0..place.projections.len()).all(|i| {
                matches!(
                    place.projections[i].kind,
                    rustc_middle::hir::place::ProjectionKind::Deref
                ) && matches!(place.ty_before_projection(i).kind(), ty::Ref(_, _, Mutability::Not))
            })
    };
    tcx.closure_captures(local).iter().map(fixed).collect()
}

/// Of each capture of `closure`, by its index among them, whether it takes
/// its value, a `move` closure's or one that consumes it, not a borrow: what
/// a `bind` may bind (`passed_on::binds`, ADR 0605, amended). None of a
/// closure of another crate's.
pub(in crate::lower) fn taken_by_value(tcx: TyCtxt<'_>, closure: DefId) -> Vec<bool> {
    let Some(local) = closure.as_local().filter(|_| tcx.is_closure_like(closure)) else {
        return Vec::new();
    };
    (tcx.closure_captures(local).iter())
        .map(|captured| {
            matches!(
                captured.info.capture_kind,
                ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse
            )
        })
        .collect()
}

/// A `dyn Fn` the crate makes that drops what it holds: where it's made,
/// and what it holds that drops, which a refusal names (ADR 0626, amended).
pub(super) struct DroppingFnDyn<'tcx> {
    /// The coercion to the `dyn Fn`.
    pub(super) span: Span,
    /// The function made one, a closure, a `fn` or a type parameter's.
    pub(super) function: Ty<'tcx>,
    /// What it holds that drops: a closure's capture, or the function
    /// itself, a type parameter's whose drop only its callers know.
    pub(super) held: Ty<'tcx>,
    /// The `dyn Fn` it's made.
    pub(super) to: Ty<'tcx>,
}

/// Each of `all_bodies`' coercions of a function to a `dyn Fn` that
/// `drops` says has something to drop, or that holds a value of a type
/// parameter a caller gives one whose drop runs, `given` by its function
/// and index, `drop_params`' answer, where `params_may_drop`; or of one of
/// a type parameter's where `foreign_drops`, as a library's function may
/// be given it (ADR 0521, amended by #580). Of a library's, whose callers
/// it can't see, `given` is `None`: one of a type parameter's may be any
/// closure it makes, given a closure: where one has, or, where
/// `foreign_drops`, one a library makes may.
pub(super) fn dropping_fn_dyns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, given, foreign_drops): (bool, Option<&GivenADrop<'_>>, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Vec<DroppingFnDyn<'tcx>> {
    // A library's type parameter's: in Rust a function is a closure, a
    // `fn` or a `dyn Fn`, whose own coercion answers for it.
    let closure_drops = || {
        foreign_drops
            || all_bodies.iter().any(|body| {
                let id = body.def_id;
                tcx.is_closure_like(id) && {
                    let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
                    drops(ty) || params_may_drop && holds_a_param(tcx, ty)
                }
            })
    };
    (all_bodies.iter())
        .flat_map(|body| {
            let owner = tcx.typeck_root_def_id(body.def_id);
            fn_dyn_sources(tcx, body).into_iter().map(move |source| (owner, source))
        })
        .filter_map(|(owner, (span, function, handed, to))| {
            if handed {
                return None;
            }
            if drops(function) {
                // The capture that drops, of a closure.
                let held = match function.kind() {
                    ty::Closure(_, args) => args.as_closure().upvar_tys().iter().find(|&t| drops(t)),
                    _ => None,
                };
                return Some(DroppingFnDyn {
                    span,
                    function,
                    held: held.unwrap_or(function),
                    to,
                });
            }
            // A type parameter's, or a closure holding one: in Rust a
            // function is a closure, a `fn` or a `dyn Fn`, of what a caller
            // gives it, which `given` answers for.
            let held = match given {
                Some(given) => {
                    foreign_drops && holds_a_param(tcx, function)
                        || params_may_drop
                            && owns_a_param(tcx, function, &|index| given(owner, index), &mut IndexSet::default())
                }
                None => holds_a_param(tcx, function) && closure_drops(),
            };
            held.then_some(DroppingFnDyn {
                span,
                function,
                held: function,
                to,
            })
        })
        .collect()
}

/// Refuses the crate, whose `dyn Fn`s drop, for `library`, whose drop
/// nothing (ADR 0626, amended): naming, where the crate makes one, `made`,
/// what it holds that drops, and of a closure's handle on what owns it, a
/// cell counted as the call it's given to doesn't own the closure, that
/// call (ADR 0317, amended).
pub(super) fn refuse_dropping_fn_dyn<'tcx>(
    tcx: TyCtxt<'tcx>,
    library: &str,
    made: Option<&DroppingFnDyn<'tcx>>,
    self_handles: &super::SelfHandles<'tcx>,
) {
    let message = format!(
        "rust-js: this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `{library}`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them, and would drop nothing of it (ADR 0626)"
    );
    let Some(made) = made else {
        tcx.dcx().err(message);
        return;
    };
    let mut error = (tcx.dcx().struct_span_err(made.span, message)).with_span_label(
        made.span,
        format!("this `dyn Fn` holds a `{}`, whose drop runs code", made.held),
    );
    if let ty::Closure(closure, _) = *made.function.kind()
        && let Some(unowned) = self_handles.unowned(closure, made.held)
    {
        let parameter = match tcx.fn_arg_idents(unowned.callee).get(unowned.given as usize) {
            Some(Some(ident)) => format!("`{ident}`"),
            _ => format!("its parameter {}", unowned.given),
        };
        error = error.with_note(format!(
            "its cell is the `let` of the `Rc` the call it's given to returns only where that return alone owns the closure (ADR 0317, amended), and `{}`'s doesn't own {parameter}: the field it's moved to must be private to its crate, of a struct that isn't `Clone`, and read only to call through it; a library's contract says so of its function (`owned_by_return`)",
            tcx.def_path_str(unowned.callee)
        ));
    }
    error.emit();
}

/// Whether a function of type `ty` may be one of a type parameter's,
/// whose drop only its callers know: a type parameter's own, `F`, or a
/// closure that captures one. Not a closure of a generic function's that
/// holds none, `move |resolve| listeners.borrow_mut().push(resolve)` in
/// `chain_thenable_value<T>`, whose type names `T` among its parent's
/// generic arguments but holds what it captures, which `drops` answers
/// for; nor a `dyn Fn` made another, `&dyn Fn(S, A) -> S`, a function
/// already, whose own coercion answers (ADR 0521, amended).
fn holds_a_param<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    owns_a_param(tcx, ty, &|_| true, &mut IndexSet::default())
}

/// Whether dropping a value of `ty` may drop a value of a type parameter:
/// one it owns, in its fields, a `Box`'s, or what std's own types own by
/// the `PhantomData` they mark it with, an `Rc`'s, a `Vec`'s. Not one that
/// only a reference, a raw pointer (`raw_pointee`'s, a `NonNull` too) or a
/// function's signature names, `&K`, `Box<dyn Fn(&K)>`, a `dyn` whose own
/// coercion answers for it (ADR 0521, amended). Of the type parameters,
/// those `given` says by index, a caller gives one whose drop runs (#580).
/// An associated type or a coroutine may hold one.
fn owns_a_param<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    given: &dyn Fn(u32) -> bool,
    seen: &mut IndexSet<Ty<'tcx>>,
) -> bool {
    if !ty.has_non_region_param() || !seen.insert(ty) || raw_pointee(tcx, ty).is_some() {
        return false;
    }
    match ty.kind() {
        ty::Ref(..) | ty::FnPtr(..) | ty::FnDef(..) | ty::Dynamic(..) => false,
        ty::Param(param) => given(param.index),
        ty::Tuple(items) => items.iter().any(|item| owns_a_param(tcx, item, given, seen)),
        ty::Array(item, _) | ty::Slice(item) => owns_a_param(tcx, *item, given, seen),
        ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|t| owns_a_param(tcx, t, given, seen)),
        ty::Adt(adt, args) if adt.is_box() || adt.is_phantom_data() => owns_a_param(tcx, args.type_at(0), given, seen),
        ty::Adt(adt, args) => adt
            .all_fields()
            .any(|field| owns_a_param(tcx, field.ty(tcx, args).skip_normalization(), given, seen)),
        _ => true,
    }
}

/// Whether any of `all_bodies` makes a value a `dyn Any` that `drops` says
/// has something to drop, or a type parameter's, where `params_may_drop`;
/// or, where `foreign_drops`, a library made one, as its contract says
/// (ADR 0691, amended).
/// Only the crate makes one: std's of its own is a panic's message.
pub(super) fn any_dyns_drop<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, foreign_drops): (bool, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let Some(any) = any_trait(tcx) else {
        return false;
    };
    let of_any = |predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>| predicates.principal_def_id() == Some(any);
    foreign_drops
        || (all_bodies.iter())
            .flat_map(|body| dyn_sources(tcx, &body.thir, of_any))
            .map(|(_, from)| from)
            .any(|from| drops(from) || params_may_drop && from.has_non_region_param())
}

/// Whether any of `all_bodies` boxes a value a `dyn Iterator` that `drops`
/// says has something to drop, or a type parameter's, where
/// `params_may_drop`; or, where `foreign_drops`, a library does, as its
/// contract says (ADR 0521, #1420). A `Box`'s alone: a `&dyn Iterator` owns
/// nothing to drop, and an `Rc` of one of what drops is refused.
pub(super) fn iterator_dyns_drop<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, foreign_drops): (bool, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    foreign_drops
        || all_bodies.iter().any(|body| {
            dyn_sources(tcx, &body.thir, |_| true)
                .filter(|&(at, _)| {
                    let to = body.thir[at].ty;
                    to.is_box() && crate::lower::recognition::is_dyn_iter(tcx, to)
                })
                .any(|(_, from)| drops(from) || params_may_drop && from.has_non_region_param())
        })
}

/// Whether `all_bodies` make a `dyn Error` of what `drops`, or of a type
/// parameter's value where `params_may_drop`, or a library does,
/// `foreign_drops` (ADR 0462, amended): an unsizing the crate writes, or a
/// std conversion it calls that boxes its error, `From<E> for Box<dyn
/// Error>`'s, `?`'s or `.into()`'s, and `io::Error::new` and `other`'s, as a
/// `dyn Any`'s are found (ADR 0691). Else a `dyn Error`, and the `io::Error`
/// that holds one, drop nothing, whatever else in the crate does.
pub(super) fn error_dyns_drop<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, foreign_drops): (bool, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    use crate::lower::recognition::{Std, StdItem, is_std_def};
    use crate::lower::std_types::io_error::IoErrorOp;
    let tcx = recognition.tcx;
    let is_error = |id: DefId| is_std_def(tcx, id, StdItem::Error);
    let of_error = move |predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>| {
        predicates.principal_def_id().is_some_and(is_error)
    };
    let boxed_error = |ty: Ty<'tcx>| {
        ty.boxed_ty()
            .is_some_and(|inner| matches!(inner.kind(), ty::Dynamic(predicates, ..) if of_error(predicates)))
    };
    let error_of = |result: Ty<'tcx>| match result.kind() {
        ty::Adt(_, args) if args.len() == 2 => args.types().nth(1),
        _ => None,
    };
    let dropping =
        |from: Ty<'tcx>| !boxed_error(from) && (drops(from) || params_may_drop && from.has_non_region_param());
    let mut boxed = Vec::new();
    for body in all_bodies {
        boxed.extend(dyn_sources(tcx, &body.thir, of_error).map(|(_, from)| from));
        for (callee, args, _) in crate::lower::instances::calls_of(body) {
            let types: Vec<Ty<'tcx>> = args.types().collect();
            // Asked only of `io::Error`'s own functions, as the one catalogue
            // of them says which make one of an error (`classify`).
            let of_io_error = tcx.inherent_impl_of_assoc(callee).is_some_and(|imp| {
                recognition.is_io_error(tcx.type_of(imp).instantiate_identity().skip_normalization())
            });
            let made = of_io_error.then(|| recognition.classify(callee, args)).flatten();
            let from = match (made, tcx.trait_of_assoc(callee)) {
                (Some(Std::IoError(IoErrorOp::New | IoErrorOp::Other)), _) => types.last().copied(),
                (_, Some(tr)) if is_std_def(tcx, tr, StdItem::From) && types.len() == 2 && boxed_error(types[0]) => {
                    Some(types[1])
                }
                (_, Some(tr)) if is_std_def(tcx, tr, StdItem::Into) && types.len() == 2 && boxed_error(types[1]) => {
                    Some(types[0])
                }
                _ if tcx.is_lang_item(callee, LangItem::TryTraitFromResidual)
                    && types.len() == 2
                    && error_of(types[0]).is_some_and(boxed_error) =>
                {
                    error_of(types[1])
                }
                _ => None,
            };
            boxed.extend(from);
        }
    }
    foreign_drops || boxed.into_iter().any(dropping)
}

/// The first of `all_bodies`' coercions to a `dyn` of the trait `tr`, of
/// what `drops`, or of a type parameter's value where `params_may_drop`:
/// where a program its tests use drops nothing of one (#597), what it would
/// drop nothing of.
pub(super) fn dropping_dyn_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (tr, params_may_drop): (DefId, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Option<(Span, Ty<'tcx>)> {
    (all_bodies.iter()).find_map(|body| {
        dyn_sources(tcx, &body.thir, move |predicates| {
            predicates.principal_def_id() == Some(tr)
        })
        .find(|&(_, from)| drops(from) || params_may_drop && from.has_non_region_param())
        .map(|(at, from)| (body.thir[at].span, from))
    })
}

/// Each closure `all_bodies` make a `dyn Fn` of only to hand it to JS: what
/// a drop of a `dyn Fn` of the crate's may drop beside what `drop_runs`
/// says, which `dropping_fn_dyn` leaves them out of (`may_run`). Each other
/// function made one whose drop runs code makes every `dyn Fn`'s drop run.
pub(super) fn handed_fn_dyns<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Vec<Ty<'tcx>> {
    let mut functions: Vec<Ty<'tcx>> = (all_bodies.iter())
        .flat_map(|body| fn_dyn_sources(tcx, body))
        .filter_map(|(_, function, handed, _)| handed.then_some(function))
        .collect();
    functions.dedup();
    functions
}

/// The functions `thir` makes `dyn Fn`s of, `Box<F>` to `Box<dyn Fn()>`'s
/// `F`, each with where it's made one and whether it's a closure made one
/// only to hand it to JS.
/// And of a library's function the crate compiles itself, of MIR alone
/// (ADR 0914), each of its own coercions, none handed to JS: what it's
/// given, a closure that holds a value with a destructor, may be made a
/// `dyn Fn` there, `node`'s `Box::new(run)` of its `F` (#1051).
fn fn_dyn_sources<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<(Span, Ty<'tcx>, bool, Ty<'tcx>)> {
    if body.root().is_none()
        && !crate::lower::recognition::is_std_crate(tcx, body.def_id)
        && let Some(mir) = &body.mir
    {
        return mir_fn_dyn_sources(tcx, &mir.body);
    }
    let thir = &body.thir;
    let handed = handed_to_js(tcx, thir);
    dyn_sources(tcx, thir, move |predicates| is_fn_dyn(tcx, predicates))
        .map(move |(at, from)| {
            let handed = matches!(from.kind(), ty::Closure(..)) && handed.contains(&at);
            let to = pointee(tcx, thir[at].ty).expect("a `dyn`'s pointer");
            (thir[at].span, from, handed, to)
        })
        .collect()
}

/// The functions a body of MIR alone makes `dyn Fn`s of, as `dyn_sources`
/// finds them in THIR: each unsizing to a pointer to one.
fn mir_fn_dyn_sources<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
) -> Vec<(Span, Ty<'tcx>, bool, Ty<'tcx>)> {
    use rustc_middle::mir::{CastKind, Rvalue, StatementKind};
    (body.basic_blocks.iter())
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| {
            let StatementKind::Assign(assign) = &statement.kind else {
                return None;
            };
            let Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize, _), operand, to) = &assign.1 else {
                return None;
            };
            let to = pointee(tcx, *to)?;
            let ty::Dynamic(predicates, ..) = to.kind() else {
                return None;
            };
            is_fn_dyn(tcx, predicates).then_some(())?;
            let from = pointee(tcx, operand.ty(&body.local_decls, tcx))?;
            Some((statement.source_info.span, from, false, to))
        })
        .collect()
}

/// The coercions to a `dyn Fn` in `thir` handed straight to JS (ADR 0873):
/// of a closure boxed where it's made, a binding's argument, or the `Some`
/// of one, that nothing evaluated after it, before the call, can unwind
/// past, of a binding that gives back no function, which might be the one
/// it's given, as `widen`'s `this` does. JS keeps what it's given, and has
/// no drop: Rust never drops it again.
fn handed_to_js(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> IndexSet<ExprId> {
    let through = |mut e: ExprId| {
        while let ExprKind::Scope { value: inner, .. } | ExprKind::ValueExpr { source: inner } = thir[e].kind {
            e = inner;
        }
        e
    };
    let coercion = |arg: ExprId| {
        let mut e = through(arg);
        if let ExprKind::Adt(adt) = &thir[e].kind
            && tcx.is_lang_item(adt.adt_def.did(), LangItem::Option)
            && let [field] = &adt.fields[..]
            && matches!(adt.base, AdtExprBase::None)
        {
            e = through(field.expr);
        }
        // Of a closure boxed where it's made, `Box::new(move || ..)`: nothing
        // else holds it before.
        let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = thir[e].kind
        else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[through(source)].kind else {
            return None;
        };
        let boxed = matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if pure_std(tcx, id) == Some(PureStd::BoxNew));
        (boxed && matches!(&args[..], [made] if matches!(thir[through(*made)].kind, ExprKind::Closure(_)))).then_some(e)
    };
    let mut handed = IndexSet::default();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let ty::FnDef(id, _) = *thir[fun].ty.kind() else {
            continue;
        };
        if !bindings::is_binding(tcx, id) || gives_back_a_function(tcx, id) {
            continue;
        }
        for (i, &arg) in args.iter().enumerate() {
            if let Some(e) = coercion(arg)
                && args[i + 1..].iter().all(|&after| cannot_unwind(thir, after))
            {
                handed.insert(e);
            }
        }
    }
    handed
}

/// The closures in `thir` rustc infers `Fn` or `FnMut`, made where they're
/// given to a binding's parameter whose callback JS calls once
/// (`Taker::parameter`), a type parameter of a union whose `Function`
/// member is called by `FnOnce`, `js::set_timeout`'s `H: IntoTimerHandler`
/// (ADR 1020): JS calls it once, or never, and keeps it, as Rust's
/// `call_once` takes it, so what it took is dropped as its call ends.
fn called_once_by_js(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> Vec<DefId> {
    let mut once = Vec::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let ty::FnDef(id, _) = *thir[fun].ty.kind() else {
            continue;
        };
        if !bindings::is_binding(tcx, id) {
            continue;
        }
        for (i, &arg) in args.iter().enumerate() {
            let mut e = arg;
            while let ExprKind::Scope { value: inner, .. } | ExprKind::ValueExpr { source: inner } = thir[e].kind {
                e = inner;
            }
            if let ExprKind::Closure(_) = thir[e].kind
                && let ty::Closure(closure, closure_args) = *thir[e].ty.kind()
                && closure_args.as_closure().kind() != ty::ClosureKind::FnOnce
                && Taker::parameter(tcx, id, i) == Some(Taker::OnceCallback)
            {
                once.push(closure);
            }
        }
    }
    once
}

/// Whether the binding `id` may give a function back: what it returns, or
/// writes through a `&mut` it's given, may hold one, a `dyn Fn`, a closure,
/// an `impl Fn` or a type parameter's, by any pointer.
fn gives_back_a_function(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let holds = |given: Ty<'_>| {
        given
            .walk()
            .filter_map(|part| part.as_type())
            .any(|ty| match ty.kind() {
                ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),
                ty::Closure(..) | ty::FnPtr(..) | ty::Param(_) => true,
                ty::Alias(_, alias) => matches!(alias.kind, ty::Opaque { .. }),
                _ => false,
            })
    };
    let sig = tcx.fn_sig(id).skip_binder().skip_binder();
    holds(sig.output())
        || (sig.inputs().iter()).any(|input| {
            input
                .walk()
                .filter_map(|part| part.as_type())
                .any(|ty| matches!(ty.kind(), ty::Ref(_, written, ty::Mutability::Mut) if holds(*written)))
        })
}

/// Whether evaluating `e` can't unwind: a literal, a constant, a variable
/// or a place of one, borrowed, and what's made of those alone.
fn cannot_unwind(thir: &Thir<'_>, e: ExprId) -> bool {
    match &thir[e].kind {
        ExprKind::Scope { value: inner, .. }
        | ExprKind::ValueExpr { source: inner }
        | ExprKind::Field { lhs: inner, .. }
        | ExprKind::Borrow { arg: inner, .. }
        | ExprKind::PointerCoercion { source: inner, .. } => cannot_unwind(thir, *inner),
        ExprKind::Literal { .. }
        | ExprKind::NonHirLiteral { .. }
        | ExprKind::ZstLiteral { .. }
        | ExprKind::NamedConst { .. }
        | ExprKind::ConstParam { .. }
        | ExprKind::StaticRef { .. }
        | ExprKind::VarRef { .. }
        | ExprKind::UpvarRef { .. } => true,
        ExprKind::Adt(adt) => {
            matches!(adt.base, AdtExprBase::None) && adt.fields.iter().all(|field| cannot_unwind(thir, field.expr))
        }
        ExprKind::Tuple { fields } | ExprKind::Array { fields } => {
            fields.iter().all(|&field| cannot_unwind(thir, field))
        }
        _ => false,
    }
}

/// The values `thir` makes `dyn`s of that `of` says are its, `Box<T>` to
/// `Box<dyn Trait>`'s `T`.
fn dyn_sources<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    of: impl Fn(&'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool + 'a,
) -> impl Iterator<Item = (ExprId, Ty<'tcx>)> + 'a {
    thir.exprs.iter_enumerated().filter_map(move |(at, expr)| {
        let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = expr.kind
        else {
            return None;
        };
        let to = pointee(tcx, expr.ty)?;
        let ty::Dynamic(predicates, ..) = to.kind() else {
            return None;
        };
        of(predicates).then_some(())?;
        Some((at, pointee(tcx, thir[source].ty)?))
    })
}

/// What a `&`, a `Box` or an `Rc` points at.
fn pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Ref(_, inner, _) => Some(*inner),
        _ if ty.is_box() => Some(ty.boxed_ty()?),
        _ => rc_pointee(tcx, ty),
    }
}

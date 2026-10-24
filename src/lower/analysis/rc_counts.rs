//! The `Rc`s whose counts the crate reads, or that point at a value with a
//! destructor, which the last of them drops (ADR 0320): only those are
//! `{ value, strong, weak }`; every other is the value it points at (ADR 0023).

use crate::lower::recognition::{is_std_crate, known_by_bounds, rc_pointee, reads_rc_count};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// What an `Rc` points at, of those counted: each type, or every one.
#[derive(Default)]
pub struct Counted<'tcx> {
    /// Every `Rc` is: one counted is of a type a generic function knows only
    /// as its parameter, or a generic `Rc` might be one counted.
    pub all: bool,
    pub types: HashSet<Ty<'tcx>>,
}

impl<'tcx> Counted<'tcx> {
    /// Does the crate count any `Rc`? Then dropping one runs (ADR 0098).
    pub fn any(&self) -> bool {
        self.all || !self.types.is_empty()
    }

    /// Is an `Rc` of `pointee` counted?
    pub fn counts(&self, tcx: TyCtxt<'tcx>, pointee: Ty<'tcx>) -> bool {
        self.all || self.types.contains(&tcx.erase_and_anonymize_regions(pointee))
    }
}

/// Whom a crate's `Rc`s are shared with, which it can't see (ADRs 0523,
/// 0844).
#[derive(Clone, Copy)]
pub(super) struct Shared {
    /// It's a library: its consumers share its `Rc`s, each as it has them,
    /// so it counts every one, or none.
    pub(super) library: bool,
    /// A library that counts its `Rc`s hands it one.
    pub(super) counted: bool,
}

/// The types whose `Rc`s the crate reads a count of, `strong_count`,
/// `get_mut`, `try_unwrap` and the rest, or asks a `Weak` of whether it's
/// alive, `upgrade` or a count (ADR 0843), and those
/// whose drop runs a destructor, `runs`, which the last of them drops: one of
/// those is counted as Rust counts it, where it's made, cloned and dropped.
/// What a type drops depends on which `Rc`s are counted, so the second to a
/// fixed point.
/// And an `Rc` of a value of a type parameter's, `Rc<RefCell<Buffer<I>>>`,
/// where `params_may_drop`, a type may have a destructor, and the
/// parameter may be one, not `T: Copy`, where a body holds it: what it's
/// made of may be one whose last clone drops it, so every `Rc` is (ADRs
/// 0494, 0844).
/// And every `Rc` is where `shared` says: a library that counts any, or a
/// crate a library that counts its `Rc`s hands one (ADRs 0523, 0844).
pub(super) fn counted_rcs<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, shared): (bool, Shared),
    runs: impl Fn(&Counted<'tcx>, Ty<'tcx>) -> bool,
) -> Counted<'tcx> {
    let mut types: HashSet<Ty<'tcx>> = HashSet::new();
    let mut generic_rc = false;
    // Whether an `Rc` a body holds holds a type parameter's value itself,
    // not one it points at, of a parameter that may have a destructor.
    let mut holds_param = false;
    // What each `Rc` of a known type points at.
    let mut pointees: HashSet<Ty<'tcx>> = HashSet::new();
    // Each `Rc` in `ty`, of a body's whose typing is `env`, through the
    // fields of the crates' types it holds, `seen` once each: where it's
    // what a body may drop, by `env`'s bounds; or in a field, `None`.
    let mut found = |ty: Ty<'tcx>, env: Option<ty::TypingEnv<'tcx>>, seen: &mut HashSet<Ty<'tcx>>| {
        let mut held = vec![ty];
        while let Some(ty) = held.pop() {
            if !seen.insert(ty) {
                continue;
            }
            for part in ty.walk().filter_map(|part| part.as_type()) {
                match rc_pointee(tcx, part) {
                    Some(pointee) if pointee.has_non_region_param() => {
                        generic_rc = true;
                        holds_param |= env.is_some_and(|env| owns_dropping_param(tcx, env, pointee));
                    }
                    Some(pointee) => {
                        pointees.insert(tcx.erase_and_anonymize_regions(pointee));
                    }
                    None => {}
                }
                // What a body holds of a type, its fields hold.
                if env.is_some()
                    && let ty::Adt(adt, args) = *part.kind()
                    && !is_std_crate(tcx, adt.did())
                {
                    held.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));
                }
            }
        }
    };
    // Each `Rc` made another's, `Rc<RefCell<Logger>>` an
    // `Rc<RefCell<dyn Listener>>`: the two share their counts.
    let mut made_others: HashSet<(Ty<'tcx>, Ty<'tcx>)> = HashSet::new();
    for body in all_bodies {
        let env = ty::TypingEnv::non_body_analysis(tcx, tcx.typeck_root_def_id(body.def_id));
        let mut seen = HashSet::new();
        // A parameter nothing reads is dropped where no expression is of it.
        for param in body.thir.params.iter() {
            found(param.ty, Some(env), &mut seen);
        }
        for expr in body.thir.exprs.iter() {
            found(expr.ty, Some(env), &mut seen);
            if let ExprKind::PointerCoercion { source, .. } = expr.kind
                && let (Some(from), Some(to)) = (rc_pointee(tcx, body.thir[source].ty), rc_pointee(tcx, expr.ty))
            {
                made_others.insert((
                    tcx.erase_and_anonymize_regions(from),
                    tcx.erase_and_anonymize_regions(to),
                ));
            }
            // A count read by a call, or by the function given as a value,
            // `.map(Rc::strong_count)`: either is an expression of its type.
            if let Some((def_id, args)) = fn_def(expr.ty)
                && reads_rc_count(tcx, def_id, args.types().next())
            {
                types.insert(tcx.erase_and_anonymize_regions(args.type_at(0)));
            }
        }
    }
    // A field's `Rc`, of a value another crate may make, where no
    // expression names it: what it points at, and whether it's generic.
    let mut seen = HashSet::new();
    for id in tcx.hir_crate_items(()).definitions() {
        if matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for field in tcx.adt_def(id).all_fields() {
                found(
                    tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                    None,
                    &mut seen,
                );
            }
        }
    }
    let mut counted = Counted { all: false, types };
    loop {
        let mut dropping: Vec<Ty<'tcx>> = (pointees.iter())
            .filter(|&&pointee| !counted.types.contains(&pointee) && runs(&counted, pointee))
            .copied()
            .collect();
        // An `Rc` made a counted one is counted: its counts are the other's.
        dropping.extend(
            (made_others.iter())
                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))
                .map(|&(from, _)| from),
        );
        if dropping.is_empty() {
            break;
        }
        counted.types.extend(dropping);
    }
    counted.all = shared.counted
        || counted.types.iter().any(|t| t.has_non_region_param())
        || !counted.types.is_empty() && generic_rc
        || params_may_drop && holds_param;
    // A library's consumers share each `Rc` as it has it: every one, or none.
    counted.all |= shared.library && counted.any();
    counted
}

/// Whether `ty` holds a value of a type parameter, or of an associated type
/// of one, itself, not behind a reference, which drops nothing, that may
/// have a destructor where `env` types it: not a `T: Copy`.
fn owns_dropping_param<'tcx>(tcx: TyCtxt<'tcx>, env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let mut walk = ty.walk();
    while let Some(part) = walk.next() {
        match part.as_type() {
            Some(t) if matches!(t.kind(), ty::Ref(..)) => walk.skip_current_subtree(),
            Some(t) if known_by_bounds(t) && t.needs_drop(tcx, env) => return true,
            _ => {}
        }
    }
    false
}

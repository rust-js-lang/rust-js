//! The `Rc`s whose counts the crate reads, or that point at a value with a
//! destructor, which the last of them drops (ADR 0320): only those are
//! `{ value, strong, weak }`; every other is the value it points at (ADR 0023).
//! Each decided by what it points at, a type and every type that may be it
//! (ADR 0852).

use crate::lower::pointers::raw_pointee;
use crate::lower::recognition::{
    compares_fn_identity, is_fn_dyn, is_rc_new, is_std_crate, known_by_bounds, rc_pointee, reads_rc_count, weak_pointee,
};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// What an `Rc` points at, of those counted: each type, and every type that
/// may be one of them, or every one.
#[derive(Default)]
pub struct Counted<'tcx> {
    /// Every `Rc` is: one counted is of a type parameter's value, which may
    /// be any type.
    pub all: bool,
    /// What each counted `Rc` points at, a generic one's of its
    /// parameters: `Slot<T>`'s counts every `Slot`'s.
    pub types: HashSet<Ty<'tcx>>,
}

impl<'tcx> Counted<'tcx> {
    /// Does the crate count any `Rc`? Then dropping one runs (ADR 0098).
    pub fn any(&self) -> bool {
        self.all || !self.types.is_empty()
    }

    /// Is an `Rc` of `pointee` counted? Where it's one counted, or may be:
    /// one generic code holds as `Rc<Slot<T>>` is any `Rc<Slot<_>>`, so
    /// the two are one representation (ADR 0852).
    pub fn counts(&self, tcx: TyCtxt<'tcx>, pointee: Ty<'tcx>) -> bool {
        let pointee = tcx.erase_and_anonymize_regions(pointee);
        self.all || self.types.contains(&pointee) || self.types.iter().any(|&t| may_be(tcx, t, pointee))
    }
}

/// What makes a type, its constructor, compared with another's to say
/// whether the two may be one type (ADR 0852), and named by a library's
/// contract.
#[derive(PartialEq, Eq)]
pub(in crate::lower) enum Head {
    /// An item's: an ADT's, `dyn` a trait's, a function's or a closure's,
    /// each kind its word.
    Def(&'static str, DefId),
    /// A primitive's, a reference's, a tuple's and the others' of no item.
    Named(String),
}

/// `ty`'s constructor and the types it's made of, where it has one: a type
/// parameter or a projection, which may be any type, has none. What it's
/// made of that can't tell two types apart, a `dyn`'s or a function
/// pointer's, an array's length, isn't compared: it may be.
pub(in crate::lower) fn head<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Head, Vec<Ty<'tcx>>)> {
    let named = |name: &str, args: Vec<Ty<'tcx>>| Some((Head::Named(name.to_string()), args));
    if let Some((t, mutability)) = raw_pointee(tcx, ty) {
        return named(if mutability.is_mut() { "*mut" } else { "*const" }, vec![t]);
    }
    match *ty.kind() {
        ty::Adt(adt, args) => Some((Head::Def("", adt.did()), args.types().collect())),
        ty::Ref(_, t, mutability) => named(if mutability.is_mut() { "&mut" } else { "&" }, vec![t]),
        ty::Slice(t) => named("[]", vec![t]),
        ty::Array(t, _) => named("[;]", vec![t]),
        ty::Tuple(ts) => named("()", ts.to_vec()),
        ty::Dynamic(predicates, ..) => match predicates.principal_def_id() {
            Some(id) => Some((Head::Def("dyn ", id), Vec::new())),
            None => named("dyn", Vec::new()),
        },
        ty::FnDef(id, _) => Some((Head::Def("fn ", id), Vec::new())),
        ty::Closure(id, _) | ty::CoroutineClosure(id, _) | ty::Coroutine(id, _) => {
            Some((Head::Def("closure ", id), Vec::new()))
        }
        ty::Foreign(id) => Some((Head::Def("extern ", id), Vec::new())),
        ty::FnPtr(..) => named("fn", Vec::new()),
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => {
            named(&tcx.erase_and_anonymize_regions(ty).to_string(), Vec::new())
        }
        _ => None,
    }
}

/// May `a` and `b` be one type, a type parameter of either any type? Where
/// either has none to compare, or both one constructor, each of what it's
/// made of may be the other's. More than rustc's unification says may,
/// never less: one counted, the other is too (ADR 0852).
pub(in crate::lower) fn may_be<'tcx>(tcx: TyCtxt<'tcx>, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
    match (head(tcx, a), head(tcx, b)) {
        (Some((a, of_a)), Some((b, of_b))) => {
            a == b && of_a.len() == of_b.len() && of_a.iter().zip(&of_b).all(|(&a, &b)| may_be(tcx, a, b))
        }
        _ => true,
    }
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
/// parameter is given one, `drop_given`'s, by a caller of its function,
/// where a body owns it: what it's made of may be one whose last clone
/// drops it (ADRs 0494, 0844). Not of one only its crate's callers give,
/// none of them a value whose drop runs a destructor, as each of them has
/// it, `read_context::<Option<Rc<Cache>>>`'s: a consumer that gives one
/// compiles its own (ADR 0914, amended).
/// And each that a library counts and hands the crate, `handed` (ADRs 0523,
/// 0852).
/// Each counted, every `Rc` the crate has of what may be the same type is:
/// generic code's `Rc<Slot<T>>` is any `Slot`'s, and an `Rc<T>`'s every
/// one (ADR 0852).
/// And an `Rc` of a `dyn Fn` whose `ptr_eq` the crate asks, where one that
/// may be it may be made of a function made apart from it, which another
/// `Rc` may be made of too: a function item's, a type parameter's, or a
/// closure copied or held before. A library's is counted anyway, as its
/// `dyn Fn`s may drop, and its consumer counts what it hands (ADR 0959).
pub(super) fn counted_rcs<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, handed, drop_given): (bool, &[Ty<'tcx>], &dyn Fn(DefId, u32) -> bool),
    runs: impl Fn(&Counted<'tcx>, Ty<'tcx>) -> bool,
) -> Counted<'tcx> {
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
    let mut counted: HashSet<Ty<'tcx>> = handed.iter().map(|&ty| erased(ty)).collect();
    // What each `Rc` and `Weak` the crate has points at.
    let mut pointees: HashSet<Ty<'tcx>> = counted.clone();
    // Each `Rc` in `ty`, through the fields of the crates' types a body
    // holds, `seen` once each.
    let mut found = |ty: Ty<'tcx>, fields: bool, seen: &mut HashSet<Ty<'tcx>>| {
        let mut held = vec![ty];
        while let Some(ty) = held.pop() {
            if !seen.insert(ty) {
                continue;
            }
            for part in ty.walk().filter_map(|part| part.as_type()) {
                if let Some(pointee) = rc_pointee(tcx, part).or_else(|| weak_pointee(tcx, part)) {
                    pointees.insert(erased(pointee));
                }
                // What a body holds of a type, its fields hold.
                if fields
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
    // What each `ptr_eq` of an `Rc<dyn Fn>` compares, and the closures
    // given `Rc::new` as they're made, and those given it made before, a
    // copy or a variable (ADR 0959).
    let mut asked: HashSet<Ty<'tcx>> = HashSet::new();
    let (mut made_with, mut made_apart) = (HashSet::new(), HashSet::new());
    for body in all_bodies {
        let owner = tcx.typeck_root_def_id(body.def_id);
        let env = ty::TypingEnv::non_body_analysis(tcx, owner);
        let given = |index| drop_given(owner, index);
        let mut seen = HashSet::new();
        let mut owned_seen = HashSet::new();
        // A parameter nothing reads is dropped where no expression is of it.
        for param in body.thir.params.iter() {
            found(param.ty, true, &mut seen);
            if params_may_drop {
                owned_dropping(tcx, (env, &given), param.ty, &mut owned_seen, &mut counted);
            }
        }
        for (id, expr) in body.thir.exprs.iter_enumerated() {
            found(expr.ty, true, &mut seen);
            if params_may_drop && !behind_pointer(&body.thir, id) {
                owned_dropping(tcx, (env, &given), expr.ty, &mut owned_seen, &mut counted);
            }
            if let ExprKind::PointerCoercion { source, .. } = expr.kind
                && let (Some(from), Some(to)) = (rc_pointee(tcx, body.thir[source].ty), rc_pointee(tcx, expr.ty))
            {
                made_others.insert((erased(from), erased(to)));
            }
            // A count read by a call, or by the function given as a value,
            // `.map(Rc::strong_count)`: either is an expression of its type.
            if let Some((def_id, args)) = fn_def(expr.ty) {
                if reads_rc_count(tcx, def_id, args.types().next()) {
                    counted.insert(erased(args.type_at(0)));
                } else if compares_fn_identity(tcx, def_id, args.types().next()) {
                    asked.insert(erased(args.type_at(0)));
                }
            }
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && fn_def(body.thir[fun].ty).is_some_and(|(def_id, _)| is_rc_new(tcx, def_id))
                && let [given] = args[..]
                && let closure = body.thir[given].ty
                && let ty::Closure(..) = closure.kind()
            {
                let closure = erased(closure);
                match made_where_given(&body.thir, given) {
                    true => made_with.insert(closure),
                    false => made_apart.insert(closure),
                };
            }
        }
    }
    // A field's `Rc`, of a value another crate may make, where no
    // expression names it.
    let mut seen = HashSet::new();
    for id in tcx.hir_crate_items(()).definitions() {
        if matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for field in tcx.adt_def(id).all_fields() {
                found(
                    tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                    false,
                    &mut seen,
                );
            }
        }
    }
    // An `Rc` of a `dyn Fn` compared by identity is counted where one that
    // may be it may be made of a function made apart from it (ADR 0959).
    // One made of another `dyn Fn`'s, its lifetime's, is that one, which
    // its own making answers for.
    let made_with_it = |from: Ty<'tcx>| match *from.kind() {
        ty::Closure(id, _) => id.is_local() && made_with.contains(&from) && !made_apart.contains(&from),
        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),
        _ => false,
    };
    for &pointee in &asked {
        if (made_others.iter()).any(|&(from, to)| !made_with_it(from) && may_be(tcx, to, pointee)) {
            counted.insert(pointee);
        }
    }
    pointees.extend(counted.iter().copied());
    let mut counted = Counted {
        all: false,
        types: counted,
    };
    loop {
        let mut more: Vec<Ty<'tcx>> = (pointees.iter())
            .filter(|&&pointee| !counted.types.contains(&pointee))
            .filter(|&&pointee| {
                // What may be one counted is: generic code's of its
                // parameter's, and each it's of (ADR 0852).
                counted.types.iter().any(|&t| may_be(tcx, t, pointee))
                    || !pointee.has_non_region_param() && runs(&counted, pointee)
            })
            .copied()
            .collect();
        // An `Rc` made a counted one is counted: its counts are the other's.
        more.extend(
            (made_others.iter())
                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))
                .map(|&(from, _)| from),
        );
        if more.is_empty() {
            break;
        }
        counted.types.extend(more);
    }
    // An `Rc` of a type parameter's value may be of any type.
    counted.all = counted.types.iter().any(|&t| head(tcx, t).is_none());
    counted
}

/// Is `given`, what `Rc::new` is given, a closure made there, `Rc::new(move
/// || ..)`, a JS function no other value is (ADR 0959)?
fn made_where_given(thir: &Thir<'_>, mut given: ExprId) -> bool {
    while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[given].kind {
        given = inner;
    }
    matches!(thir[given].kind, ExprKind::Closure(_))
}

/// Each `Rc` a body owns in `ty`, not behind a reference, a pointer or a
/// `Weak`, nor of a function's type, which names its arguments' types and
/// holds no value of them, through the fields of the crates' types, of a value of a type parameter
/// a caller gives one with a destructor, `given` by its index, `seen` once each: what
/// its last clone drops may be one (ADRs 0494, 0844), so it's counted.
fn owned_dropping<'tcx>(
    tcx: TyCtxt<'tcx>,
    (env, given): (ty::TypingEnv<'tcx>, &dyn Fn(u32) -> bool),
    ty: Ty<'tcx>,
    seen: &mut HashSet<Ty<'tcx>>,
    counted: &mut HashSet<Ty<'tcx>>,
) {
    let mut held = vec![ty];
    while let Some(ty) = held.pop() {
        if !seen.insert(ty) {
            continue;
        }
        let mut walk = ty.walk();
        while let Some(part) = walk.next() {
            let Some(part) = part.as_type() else { continue };
            if matches!(part.kind(), ty::Ref(..) | ty::FnDef(..))
                || raw_pointee(tcx, part).is_some()
                || weak_pointee(tcx, part).is_some()
            {
                walk.skip_current_subtree();
            } else if let Some(pointee) = rc_pointee(tcx, part)
                && pointee.has_non_region_param()
                && owns_dropping_param(tcx, (env, given), pointee)
            {
                counted.insert(tcx.erase_and_anonymize_regions(pointee));
            } else if let ty::Adt(adt, args) = *part.kind()
                && !is_std_crate(tcx, adt.did())
            {
                held.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));
                walk.skip_current_subtree();
            }
        }
    }
}

/// Is expression `id` a place behind a pointer, `*context` or its field,
/// which the body reads or borrows but doesn't own, and so drops nothing of?
fn behind_pointer(thir: &Thir<'_>, id: ExprId) -> bool {
    match thir[id].kind {
        ExprKind::Deref { .. } => true,
        ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } => behind_pointer(thir, lhs),
        ExprKind::Scope { value, .. } => behind_pointer(thir, value),
        _ => false,
    }
}

/// Whether `ty` holds a value of a type parameter a caller gives one with a
/// destructor, `given` by its index, or of an associated type of one that
/// may have a destructor where `env` types it, itself, not behind a
/// reference or a `Weak`, which drop nothing of it, through the fields of
/// the crates' types: not a `T: Copy`.
fn owns_dropping_param<'tcx>(
    tcx: TyCtxt<'tcx>,
    (env, given): (ty::TypingEnv<'tcx>, &dyn Fn(u32) -> bool),
    ty: Ty<'tcx>,
) -> bool {
    let mut held = vec![ty];
    let mut seen = HashSet::new();
    while let Some(ty) = held.pop() {
        if !seen.insert(ty) {
            continue;
        }
        let mut walk = ty.walk();
        while let Some(part) = walk.next() {
            match part.as_type() {
                Some(t) if matches!(t.kind(), ty::Ref(..)) || weak_pointee(tcx, t).is_some() => {
                    walk.skip_current_subtree()
                }
                Some(t) if let ty::Param(param) = t.kind() => {
                    if given(param.index) && t.needs_drop(tcx, env) {
                        return true;
                    }
                }
                Some(t) if known_by_bounds(t) && t.needs_drop(tcx, env) => return true,
                Some(t)
                    if let ty::Adt(adt, args) = *t.kind()
                        && !is_std_crate(tcx, adt.did()) =>
                {
                    held.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));
                    walk.skip_current_subtree();
                }
                _ => {}
            }
        }
    }
    false
}

//! The `Rc`s whose counts the crate reads, or that point at a value with a
//! destructor, which the last of them drops (ADR 0320): only those are
//! `{ value, strong, weak }`; every other is the value it points at (ADR 0023).
//! Each decided by what it points at, a type and every type that may be it
//! (ADR 0852).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::pointers::raw_pointee;
use crate::lower::recognition::{
    StdItem, compares_fn_identity, is_fn_dyn, is_rc_new, is_std_crate, is_std_def, known_by_bounds, rc_pointee,
    reads_rc_count, weak_pointee,
};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// What an `Rc` points at, of those counted: each type, and every type that
/// may be one of them, or every one.
#[derive(Default)]
pub struct Counted<'tcx> {
    /// Every `Rc` is: one counted is of a type parameter's value, which may
    /// be any type.
    pub all: bool,
    /// What each counted `Rc` points at, a generic one's of its
    /// parameters: `Slot<T>`'s counts every `Slot`'s.
    pub types: IndexSet<Ty<'tcx>>,
    /// Whether the crate compares an `Rc<dyn Fn>` it doesn't count,
    /// `Rc::ptr_eq` of one whose pointee is in `asked` and not counted:
    /// its identity is its JS function's (ADR 0959), so a closure made an
    /// `Rc`'s stays a function of its own (`passed_on`'s
    /// `counted_closure`, rd313).
    pub compared_uncounted: bool,
    /// Whether the crate holds a raw pointer to a `dyn Fn`, what `ptr::eq`,
    /// `ptr::addr_eq`, `Rc::as_ptr` and `Rc::into_raw` give or take, whose
    /// address, compared, hashed or cast, is its JS function's identity:
    /// then a closure that passes on to a scope's method is a function of
    /// its own, as two such are two to Rust (#500, the user's decision). A
    /// closure whose own address is taken is a box of its own already.
    pub points_at_functions: bool,
    /// Each `Box`, `Rc` and `Arc` made a `dyn Fn` of a function another may
    /// hold too, a `fn` pointer or item, a closure copied or cloned, a type
    /// parameter's or a `&dyn Fn`, where the crate reads a `dyn Fn`'s
    /// address, a raw pointer to a `dyn Fn` of its arguments
    /// (`points_at_functions`): two are two allocations to Rust,
    /// apart, so each is a function of its own where it's unsized
    /// (`own_function`, #1053), by what owns it. A `Box` of a value of no
    /// size too, which allocates nothing, whose address Rust promises
    /// nothing of (the user's decision, ADR 0262's second case). Not an
    /// `Rc`'s or an `Arc`'s closure, which is made one where it's given
    /// `Rc::new`, as the `Rc` may be cloned before it's unsized
    /// (`rc_new_apart`).
    pub apart: IndexSet<Ty<'tcx>>,
    /// The closures given `Rc::new` or `Arc::new` that another may be too,
    /// where the crate reads a `dyn Fn`'s address, of an `Rc` made a `dyn
    /// Fn`: each a function of its own where it's given it (#1053).
    pub rc_new_apart: IndexSet<Ty<'tcx>>,
    /// Of `apart`, each `Rc` or `Arc` of what's no closure, a `fn` pointer
    /// or item, whose `Rc` the crate may clone, or upgrade a `Weak` to,
    /// before it's unsized: two of one allocation would be two functions,
    /// so it's refused (the user's decision, #1053).
    pub shared_apart: IndexSet<Ty<'tcx>>,
    /// The functions that make an `Rc<dyn Fn>` it doesn't count of what
    /// another may be made of too, a closure cloned or a function item,
    /// which nothing here compares: the two are one JS function, which a
    /// library's consumer that compares them compiles its own of (ADR
    /// 0959, amended; #1052).
    pub merges: IndexSet<DefId>,
}

/// What `ty`, an expression's, reads of a function's identity, as
/// `counted_rcs` asks: `Rc::ptr_eq` of an `Rc<dyn Fn>`, by what it points
/// at (ADR 0959), or a raw pointer to a `dyn Fn`, whose address is its JS
/// function's (#500).
enum IdentityRead<'tcx> {
    Compared(Ty<'tcx>),
    /// Of the `dyn Fn` pointed at.
    Pointed(Ty<'tcx>),
}

fn identity_read<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<IdentityRead<'tcx>> {
    if let Some((pointee, _)) = raw_pointee(tcx, ty)
        && matches!(pointee.kind(), ty::Dynamic(predicates, ..) if is_fn_dyn(tcx, predicates))
    {
        return Some(IdentityRead::Pointed(pointee));
    }
    let (def_id, args) = fn_def(ty)?;
    compares_fn_identity(tcx, def_id, args.types().next()).then(|| IdentityRead::Compared(args.type_at(0)))
}

/// Does the crate read a function's identity, as `counted_rcs` asks it
/// (`identity_read`)? Then a library function whose JS may give one
/// function where Rust makes two, which its library compared none of, is
/// compiled here (ADR 0914, amended; #1052).
pub(in crate::lower) fn reads_fn_identity<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> bool {
    (all_bodies.iter()).any(|body| body.thir.exprs.iter().any(|expr| identity_read(tcx, expr.ty).is_some()))
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

    /// Is `owner`, a `Box`, an `Rc` or an `Arc` unsized to a `dyn Fn`, a
    /// function of its own, apart from every other's (`apart`, #1053)?
    pub fn own_function(&self, tcx: TyCtxt<'tcx>, owner: Ty<'tcx>) -> bool {
        self.apart.contains(&tcx.erase_and_anonymize_regions(owner))
    }

    /// Is `closure`, given `Rc::new` or `Arc::new`, a function of its own
    /// there (`rc_new_apart`, #1053)?
    pub fn own_function_given(&self, tcx: TyCtxt<'tcx>, closure: Ty<'tcx>) -> bool {
        self.rc_new_apart.contains(&tcx.erase_and_anonymize_regions(closure))
    }

    /// Is `owner`, an `Rc` or an `Arc` unsized to a `dyn Fn` a function of
    /// its own, one the crate may share before (`shared_apart`, #1053)?
    pub fn shared_before_apart(&self, tcx: TyCtxt<'tcx>, owner: Ty<'tcx>) -> bool {
        self.shared_apart.contains(&tcx.erase_and_anonymize_regions(owner))
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
        // A `dyn Fn`'s, by what it takes, as one may be another only where
        // each takes what the other does (#1003); its kind isn't compared,
        // as one upcasts to another.
        ty::Dynamic(predicates, ..) if is_fn_dyn(tcx, predicates) => {
            let args = super::closures::fn_dyn_args(tcx, predicates).into_iter().collect();
            named("dyn Fn", args)
        }
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

/// Whether a function's type parameter, by its owner and index, is given a
/// value whose drop runs, asked of a generic `Rc`'s pointee.
pub(super) type DropGiven<'a, 'tcx> = dyn Fn(DefId, u32, Ty<'tcx>) -> bool + 'a;

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
/// asked with what the `Rc` points at, where a body owns it: what it's made of may be one whose last clone
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
    (all_bodies, self_handles): (&[&Body<'tcx>], &super::SelfHandles<'tcx>),
    (params_may_drop, handed, drop_given): (bool, &[Ty<'tcx>], &DropGiven<'_, 'tcx>),
    runs: impl Fn(&Counted<'tcx>, Ty<'tcx>) -> bool,
) -> Counted<'tcx> {
    let erased = |ty: Ty<'tcx>| tcx.erase_and_anonymize_regions(ty);
    let mut counted: IndexSet<Ty<'tcx>> = handed.iter().map(|&ty| erased(ty)).collect();
    // What each `Rc` and `Weak` the crate has points at.
    let mut pointees: IndexSet<Ty<'tcx>> = counted.clone();
    // Each `Rc` in `ty`, through the fields of the crates' types a body
    // holds, `seen` once each.
    // What holds an `Rc` of a type parameter's value: the function and the
    // parameter's index, or `None` of a field's or an impl's, which any
    // caller may give (#1001).
    let mut holders: IndexSet<Option<(DefId, u32)>> = IndexSet::default();
    let mut found = |ty: Ty<'tcx>, holder: Option<DefId>, fields: bool, seen: &mut IndexSet<Ty<'tcx>>| {
        let mut held = vec![ty];
        while let Some(ty) = held.pop() {
            if !seen.insert(ty) {
                continue;
            }
            for part in ty.walk().filter_map(|part| part.as_type()) {
                if let Some(pointee) = rc_pointee(tcx, part).or_else(|| weak_pointee(tcx, part)) {
                    pointees.insert(erased(pointee));
                    if let ty::Param(param) = *pointee.kind() {
                        holders.insert(holder.map(|holder| (holder, param.index)));
                    }
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
    let mut made_others: IndexSet<(Ty<'tcx>, Ty<'tcx>)> = IndexSet::default();
    // What each `ptr_eq` of an `Rc<dyn Fn>` compares, and the closures
    // given `Rc::new` as they're made, and those given it made before, a
    // copy or a variable (ADR 0959).
    let mut asked: IndexSet<Ty<'tcx>> = IndexSet::default();
    let (mut made_with, mut made_apart) = (IndexSet::default(), IndexSet::default());
    // The closures made an `Rc<dyn Fn>` that are `Copy`, and those a body
    // clones: what else may be one JS function given it twice.
    let (mut copied, mut cloned): (IndexSet<Ty<'tcx>>, IndexSet<Ty<'tcx>>) = (IndexSet::default(), IndexSet::default());
    let mut points_at_functions = false;
    let fn_dyn = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Dynamic(predicates, ..) if is_fn_dyn(tcx, predicates));
    // Each `Box`, `Rc` and `Arc` made a `dyn Fn`, and what it's made of.
    let mut unsized_fns: IndexSet<(Ty<'tcx>, Ty<'tcx>, Ty<'tcx>)> = IndexSet::default();
    // Each `dyn Fn` whose address the crate reads, a raw pointer's pointee.
    let mut pointed: IndexSet<Ty<'tcx>> = IndexSet::default();
    // And those unsized elsewhere than where they're made.
    let mut unsized_later: IndexSet<Ty<'tcx>> = IndexSet::default();
    // And the function that makes each, and what it's made of.
    let mut allocated_by: Vec<(DefId, Ty<'tcx>)> = Vec::new();
    // What each `Weak` a body has points at: an `Rc` of it may be upgraded
    // to another of one allocation.
    let mut weaks: IndexSet<Ty<'tcx>> = IndexSet::default();
    // Each count read of a type parameter's `Rc`, by the function and the
    // parameter's index, and the parameter.
    let mut of_params: IndexSet<(DefId, u32, Ty<'tcx>)> = IndexSet::default();
    for body in all_bodies {
        let owner = tcx.typeck_root_def_id(body.def_id);
        let env = ty::TypingEnv::non_body_analysis(tcx, owner);
        let given = |index, pointee| drop_given(owner, index, pointee);
        let mut seen = IndexSet::default();
        let mut owned_seen = IndexSet::default();
        // A parameter nothing reads is dropped where no expression is of it.
        for param in body.thir.params.iter() {
            found(param.ty, Some(owner), true, &mut seen);
            if params_may_drop {
                owned_dropping(tcx, (env, &given), param.ty, &mut owned_seen, &mut counted);
            }
        }
        // A library's function the crate compiles itself, of MIR alone
        // (ADR 0914): each `Rc` it makes another's, its made of a closure
        // of another crate's, which may be one JS function given twice
        // (#1052).
        if body.root().is_none()
            && !is_std_crate(tcx, body.def_id)
            && let Some(mir) = &body.mir
        {
            for (from_owner, to_owner) in owners_made_others(tcx, &mir.body) {
                if let (Some(from), Some(to)) = (rc_pointee(tcx, from_owner), rc_pointee(tcx, to_owner)) {
                    made_others.insert((erased(from), erased(to)));
                }
                // And each `Box` and `Rc` it makes a `dyn Fn`, which may be
                // shared before (#1053).
                if let Some(from) = from_owner.boxed_ty().or_else(|| rc_pointee(tcx, from_owner))
                    && let Some(to) = to_owner.boxed_ty().or_else(|| rc_pointee(tcx, to_owner))
                    && fn_dyn(to)
                {
                    if tcx.type_is_copy_modulo_regions(env, from) {
                        copied.insert(erased(from));
                    }
                    unsized_fns.insert((erased(from_owner), erased(from), erased(to)));
                    unsized_later.insert(erased(from_owner));
                    allocated_by.push((owner, erased(from)));
                }
            }
            // And each value it clones, a closure given twice.
            cloned.extend(clones_of(tcx, &mir.body).into_iter().map(erased));
        }
        for (id, expr) in body.thir.exprs.iter_enumerated() {
            found(expr.ty, Some(owner), true, &mut seen);
            if let Some(pointee) = weak_pointee(tcx, expr.ty) {
                weaks.insert(erased(pointee));
            }
            match identity_read(tcx, expr.ty) {
                Some(IdentityRead::Pointed(pointee)) => {
                    points_at_functions = true;
                    pointed.insert(erased(pointee));
                }
                Some(IdentityRead::Compared(pointee)) => _ = asked.insert(erased(pointee)),
                None => {}
            }
            if params_may_drop && !behind_pointer(&body.thir, id) {
                owned_dropping(tcx, (env, &given), expr.ty, &mut owned_seen, &mut counted);
            }
            if let ExprKind::PointerCoercion { source, .. } = expr.kind
                && let (Some(from), Some(to)) = (rc_pointee(tcx, body.thir[source].ty), rc_pointee(tcx, expr.ty))
            {
                // An `Rc` of a `&dyn Fn` made right there of a function made
                // right there, `Rc::from(function::bind(..))` (rd231).
                if let ty::Ref(_, function, _) = from.kind()
                    && let ty::Dynamic(predicates, ..) = function.kind()
                    && is_fn_dyn(tcx, predicates)
                {
                    match made_of_a_new_function(tcx, &body.thir, source) {
                        true => made_with.insert(erased(from)),
                        false => made_apart.insert(erased(from)),
                    };
                }
                made_others.insert((erased(from), erased(to)));
            }
            // A `Box`, an `Rc` or an `Arc` made a `dyn Fn` of a function, by
            // what owns it and what it owns: one of what another may hold
            // too is a function of its own (#1053).
            if let ExprKind::PointerCoercion { source, .. } = expr.kind
                && let owner_ty = body.thir[source].ty
                && let Some(from) = owner_ty.boxed_ty().or_else(|| rc_pointee(tcx, owner_ty))
                && let Some(to) = expr.ty.boxed_ty().or_else(|| rc_pointee(tcx, expr.ty))
                && fn_dyn(to)
            {
                if tcx.type_is_copy_modulo_regions(env, from) {
                    copied.insert(erased(from));
                }
                unsized_fns.insert((erased(owner_ty), erased(from), erased(to)));
                // One unsized where it's made, `Rc::new(f)`'s, no clone
                // of it before; any other may be shared.
                if !made_where_unsized(tcx, &body.thir, source) {
                    unsized_later.insert(erased(owner_ty));
                }
                allocated_by.push((owner, erased(from)));
            }
            // A count read by a call, or by the function given as a value,
            // `.map(Rc::strong_count)`: either is an expression of its type.
            if let Some((def_id, args)) = fn_def(expr.ty) {
                if tcx.is_lang_item(def_id, rustc_hir::attrs::lang_items::LangItem::CloneFn)
                    && let Some(of) = args.types().next()
                {
                    cloned.insert(erased(of));
                }
                // A closure's upgrade of the handle on itself always gives
                // its `Rc`, and asks no count (ADR 0843, amended).
                if reads_rc_count(tcx, def_id, args.types().next())
                    && !self_handles.asks_no_count(body.def_id, expr.span)
                {
                    // Of a type parameter's value, what the function's
                    // callers give it, where they're seen (#1001).
                    match *args.type_at(0).kind() {
                        ty::Param(param) => _ = of_params.insert((owner, param.index, erased(args.type_at(0)))),
                        _ => _ = counted.insert(erased(args.type_at(0))),
                    }
                }
            }
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && fn_def(body.thir[fun].ty)
                    .is_some_and(|(def_id, _)| is_rc_new(tcx, def_id) || is_std_def(tcx, def_id, StdItem::BoxNew))
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
    // expression names it; and an impl's, `impl<T> StableDeref for Rc<T>`,
    // whose dictionary holds `Rc<T>`'s `Deref` and drop, though no body of
    // it names it: of a `T` that may be one counted, every `Rc` given it is
    // counted, as generic code's is (ADR 0852, #949).
    let mut seen = IndexSet::default();
    for id in tcx.hir_crate_items(()).definitions() {
        match tcx.def_kind(id) {
            DefKind::Struct | DefKind::Enum | DefKind::Union => {
                for field in tcx.adt_def(id).all_fields() {
                    found(
                        tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                        None,
                        false,
                        &mut seen,
                    );
                }
            }
            DefKind::Impl { .. } => found(
                tcx.type_of(id).instantiate_identity().skip_normalization(),
                None,
                false,
                &mut seen,
            ),
            _ => {}
        }
    }
    // An `Rc` of a `dyn Fn` compared by identity is counted where one that
    // may be it may be made of a function made apart from it (ADR 0959).
    // One made of another `dyn Fn`'s, its lifetime's, is that one, which
    // its own making answers for.
    // One neither `Copy` nor cloned is given `Rc::new` once, moved, a JS
    // function of its own wherever it was made: React's
    // `subscribe(Rc::new(handle_store_change))` (ADR 0959, amended).
    // One of a `&dyn Fn`, a JS function already, is that function where
    // each is made of a function made with it, `Rc::from(function::bind(..))`,
    // which no other is; one made of a reference given it may share it
    // with another (rd231).
    let made_with_it = |from: Ty<'tcx>| match *from.kind() {
        ty::Ref(_, function, _) if matches!(function.kind(), ty::Dynamic(predicates, ..) if is_fn_dyn(tcx, predicates)) => {
            made_with.contains(&from) && !made_apart.contains(&from)
        }
        ty::Closure(id, _) => {
            id.is_local()
                && (made_with.contains(&from) && !made_apart.contains(&from)
                    || !copied.contains(&from) && !cloned.contains(&from))
        }
        ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),
        _ => false,
    };
    // Of an allocation's address, a closure of another crate's too, which a
    // body of MIR alone makes, its copies and clones read there (#1053).
    let its_own =
        |from: Ty<'tcx>| made_with_it(from) || from.is_closure() && !copied.contains(&from) && !cloned.contains(&from);
    for &pointee in &asked {
        if (made_others.iter()).any(|&(from, to)| !made_with_it(from) && may_be(tcx, to, pointee)) {
            counted.insert(pointee);
        }
    }
    // A type parameter's count read: the parameter, which settles as any
    // other a function holds does, below.
    for &(owner, index, param) in &of_params {
        counted.insert(param);
        holders.insert(Some((owner, index)));
    }
    pointees.extend(counted.iter().copied());
    // Where a `dyn Fn`'s address is read, each allocation made one of what
    // another may hold too is a function of its own (#1053).
    // An `Rc`'s closure, which has no identity but its address, is one where
    // it's given `Rc::new`, as its clones before it's unsized share it; what
    // else an `Rc` holds, a `fn` pointer whose value may be compared too,
    // where it's unsized, refused where it may be shared before.
    // Only one whose address may be read: a `dyn Fn` that may be one a raw
    // pointer points at, which takes what it takes, as no coercion of a
    // `dyn Fn` changes that (`may_be`).
    let address_read = |to: Ty<'tcx>| pointed.iter().any(|&read| may_be(tcx, read, to));
    let of_its_own = (unsized_fns.iter())
        .filter(|&&(_, from, to)| address_read(to) && !its_own(from))
        .map(|&(owner, from, _)| (owner, from))
        .collect::<Vec<_>>();
    let (rc_closures, unsized_apart): (Vec<_>, Vec<_>) = of_its_own
        .iter()
        .partition(|&&(owner, from)| !owner.is_box() && from.is_closure());
    let rc_new_apart = rc_closures.into_iter().map(|&(_, from)| from).collect();
    let apart: IndexSet<Ty<'tcx>> = unsized_apart.iter().map(|&&(owner, _)| owner).collect();
    let shared_apart = (unsized_apart.iter())
        .filter(|&&&(owner, from)| {
            !owner.is_box()
                && unsized_later.contains(&owner)
                && (cloned.iter().any(|&clone| may_be(tcx, clone, owner))
                    || weaks.iter().any(|&weak| may_be(tcx, weak, from)))
        })
        .map(|&&(owner, _)| owner)
        .collect();
    let mut counted = Counted {
        all: false,
        compared_uncounted: false,
        points_at_functions,
        apart,
        rc_new_apart,
        shared_apart,
        types: counted,
        merges: IndexSet::default(),
    };
    // Settled to a fixed point; of a type parameter's resolved, `bare`
    // false, never the parameter itself, which `may_be` would take for
    // every type.
    let settle = |counted: &mut Counted<'tcx>, bare: bool| loop {
        let mut more: Vec<Ty<'tcx>> = (pointees.iter())
            .filter(|&&pointee| !counted.types.contains(&pointee))
            .filter(|&&pointee| bare || head(tcx, pointee).is_some())
            .filter(|&&pointee| {
                // What may be one counted is: generic code's of its
                // parameter's, and each it's of (ADR 0852).
                counted.types.iter().any(|&t| may_be(tcx, t, pointee))
                    || !pointee.has_non_region_param() && runs(counted, pointee)
            })
            .copied()
            .collect();
        // An `Rc` made a counted one is counted, and one a counted one is
        // made, `Rc<{closure}>`'s `Rc<dyn Fn>`: the two are one allocation,
        // whose counts each reads, so one representation (#458).
        more.extend(
            (made_others.iter())
                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))
                .map(|&(from, _)| from),
        );
        more.extend(
            (made_others.iter())
                .filter(|(from, to)| counted.types.contains(from) && !counted.types.contains(to))
                .map(|&(_, to)| to),
        );
        if more.is_empty() {
            break;
        }
        counted.types.extend(more);
    };
    // What's counted before a type parameter's stands for every type.
    let before = counted.types.clone();
    settle(&mut counted, true);
    // An `Rc` of a type parameter's value may be of any type; but where each
    // function that holds one is seen given its types, those types alone,
    // as what's given it may be no other (#1001).
    if counted.types.iter().any(|&t| head(tcx, t).is_none()) {
        let callers = callers_of(tcx, all_bodies);
        let given: Option<Vec<IndexSet<Ty<'tcx>>>> = (holders.iter())
            .map(|&holder| holder.and_then(|held| given_types(tcx, &callers, held, &mut IndexSet::default())))
            .collect();
        match given {
            Some(given) => {
                counted.types = before.into_iter().filter(|&t| head(tcx, t).is_some()).collect();
                counted.types.extend(given.into_iter().flatten());
                settle(&mut counted, false);
            }
            None => counted.all = true,
        }
    }
    counted.compared_uncounted = !counted.all && asked.iter().any(|pointee| !counted.types.contains(pointee));
    // A `Box`, an `Rc` or an `Arc` made a `dyn Fn` of what another may hold
    // too, one JS function here, which a consumer that compares the two,
    // `Rc::ptr_eq` (#1052) or their addresses (#1053), makes apart in its
    // own compile; a counted `Rc`'s box too, whose function is shared.
    counted.merges = (allocated_by.iter())
        .filter(|&&(_, from)| !its_own(from))
        .map(|&(owner, _)| owner)
        .collect();
    counted
}

/// What a body of MIR alone clones, `Clone::clone`'s `Self`.
fn clones_of<'tcx>(tcx: TyCtxt<'tcx>, body: &rustc_middle::mir::Body<'tcx>) -> Vec<Ty<'tcx>> {
    use rustc_middle::mir::TerminatorKind;
    (body.basic_blocks.iter())
        .filter_map(|block| match &block.terminator().kind {
            TerminatorKind::Call { func, .. } => fn_def(func.ty(&body.local_decls, tcx)),
            _ => None,
        })
        .filter(|&(id, _)| tcx.is_lang_item(id, rustc_hir::attrs::lang_items::LangItem::CloneFn))
        .filter_map(|(_, args)| args.types().next())
        .collect()
}

/// Each `Box` or `Rc` a body of MIR alone makes another's, `Rc<{closure}>`
/// an `Rc<dyn Fn()>`: what it's made of, and what it's made, each whole.
fn owners_made_others<'tcx>(tcx: TyCtxt<'tcx>, body: &rustc_middle::mir::Body<'tcx>) -> Vec<(Ty<'tcx>, Ty<'tcx>)> {
    use rustc_middle::mir::{CastKind, Rvalue, StatementKind};
    let owns = |ty: Ty<'tcx>| ty.is_box() || rc_pointee(tcx, ty).is_some();
    (body.basic_blocks.iter())
        .flat_map(|block| block.statements.iter())
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign) => match &assign.1 {
                Rvalue::Cast(CastKind::PointerCoercion(..), operand, to) => {
                    let from = operand.ty(&body.local_decls, tcx);
                    (owns(from) && owns(*to)).then_some((from, *to))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Each function the crate names, called or as a value, and each naming of
/// it: by what, its body's owner, and its generic arguments there.
type Callers<'tcx> = IndexMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>)>>;

fn callers_of<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Callers<'tcx> {
    let mut callers: Callers<'tcx> = IndexMap::default();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id);
        for (callee, args, _) in crate::lower::instances::calls_of(body) {
            callers.entry(callee).or_default().push((caller, args));
        }
    }
    callers
}

/// Each type the crate's namings of `owner` give its type parameter of
/// `index`, of a caller's own parameter what its callers give that, in
/// turn: `None` where one can't be seen, so the parameter may be any type.
/// A trait's or an impl's method is called through a dictionary; a `pub`
/// function by JS, or a library's by its consumers; and one given what
/// holds a type parameter, not one, is given what's resolved no further.
fn given_types<'tcx>(
    tcx: TyCtxt<'tcx>,
    callers: &Callers<'tcx>,
    (owner, index): (DefId, u32),
    seen: &mut IndexSet<(DefId, u32)>,
) -> Option<IndexSet<Ty<'tcx>>> {
    // Met again, a caller's own: what reaches it is what its other callers
    // give, which they answer for.
    if !seen.insert((owner, index)) {
        return Some(IndexSet::default());
    }
    if tcx.trait_of_assoc(owner).is_some()
        || tcx.trait_impl_of_assoc(owner).is_some()
        || tcx.visibility(owner).is_public()
    {
        return None;
    }
    let mut given = IndexSet::default();
    for &(caller, args) in callers.get(&owner).into_iter().flatten() {
        let arg = tcx.erase_and_anonymize_regions(args.get(index as usize)?.as_type()?);
        match *arg.kind() {
            ty::Param(param) => given.extend(given_types(tcx, callers, (caller, param.index), seen)?),
            _ if arg.has_non_region_param() => return None,
            _ => _ = given.insert(arg),
        }
    }
    Some(given)
}

/// Is `rc`, an `Rc` unsized, one made there of a JS function made there,
/// `Rc::from(function::bind(..))` or `Rc::new(function::with_this(..))`,
/// so a function no other `Rc` holds (rd231)?
fn made_of_a_new_function(tcx: TyCtxt<'_>, thir: &Thir<'_>, rc: ExprId) -> bool {
    let through = |mut e: ExprId| {
        while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
            e = inner;
        }
        e
    };
    let ExprKind::Call { fun, ref args, .. } = thir[through(rc)].kind else {
        return false;
    };
    let [given] = args[..] else { return false };
    let makes_rc = fn_def(thir[fun].ty).is_some_and(|(id, _)| {
        is_rc_new(tcx, id)
            || tcx.trait_of_assoc(id).is_some_and(|t| {
                crate::lower::recognition::is_std_def(tcx, t, crate::lower::recognition::StdItem::From)
            })
    });
    let ExprKind::Call { fun: made, .. } = thir[through(given)].kind else {
        return false;
    };
    makes_rc
        && fn_def(thir[made].ty).is_some_and(|(id, _)| {
            crate::lower::recognition::is_function_bind(tcx, id)
                || matches!(
                    crate::lower::bindings::js_form(tcx, id),
                    crate::lower::bindings::JsForm::WithThis
                ) && crate::lower::bindings::is_binding(tcx, id)
        })
}

/// Is `owner`, an `Rc` or an `Arc` unsized, made right there,
/// `Rc::new(f)`'s, so no clone of it is made before (#1053)?
fn made_where_unsized(tcx: TyCtxt<'_>, thir: &Thir<'_>, mut owner: ExprId) -> bool {
    while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[owner].kind {
        owner = inner;
    }
    let ExprKind::Call { fun, .. } = thir[owner].kind else {
        return false;
    };
    fn_def(thir[fun].ty).is_some_and(|(id, _)| is_rc_new(tcx, id))
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
/// a caller gives one with a destructor, `given` by its index and the `Rc`'s pointee, `seen` once each: what
/// its last clone drops may be one (ADRs 0494, 0844), so it's counted.
fn owned_dropping<'tcx>(
    tcx: TyCtxt<'tcx>,
    (env, given): (ty::TypingEnv<'tcx>, &dyn Fn(u32, Ty<'tcx>) -> bool),
    ty: Ty<'tcx>,
    seen: &mut IndexSet<Ty<'tcx>>,
    counted: &mut IndexSet<Ty<'tcx>>,
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
                && owns_dropping_param(tcx, (env, &|index| given(index, pointee)), pointee)
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
/// the crates' types and what a closure captures: not a `T: Copy`, nor a
/// closure that captures none of it (ADR 0626, amended).
fn owns_dropping_param<'tcx>(
    tcx: TyCtxt<'tcx>,
    (env, given): (ty::TypingEnv<'tcx>, &dyn Fn(u32) -> bool),
    ty: Ty<'tcx>,
) -> bool {
    let mut held = vec![ty];
    let mut seen = IndexSet::default();
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
                // A closure holds what it captures, not its function's type
                // arguments nor its signature's types, which its type names:
                // a generic function's `Rc::new(|_| {})` holds nothing (rd597).
                Some(t) if let ty::Closure(_, args) = *t.kind() => {
                    held.extend(args.as_closure().upvar_tys());
                    walk.skip_current_subtree();
                }
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

//! Which thread-locals are made as their module loads, as JS's `const x =
//! f();` and `export const` are, and which where they're first used, as
//! Rust makes each (the user's decisions, ADR 1006, amended): one is made
//! as its module loads unless that would show (#554). What its initializer
//! runs, `may_run`'s answer, is asked: whether it prints or panics, reads
//! state something other than the loading code writes, or writes state
//! something else touches (`Shows`). React DOM's `const
//! isScrollEndEventSupported = isEventSupported("scrollend")`, which calls
//! bindings alone, is made as its module loads. A private one that would
//! show is made where it's first used, `$local(() => ..)`; a `pub` one is
//! refused, as JS's importers read an `export const`. The pipeline asks
//! too, of the modules' imports, where its module's load can't make one
//! first (`load_cycles::out_of_order`): one that runs what rust-js can't
//! follow, reads itself, or runs a module in an import cycle with its own,
//! made where it's first used, the crate lowered again so
//! (`made_where_used`), or refused.

use std::collections::{HashMap, HashSet};

use rustc_hir as hir;
use rustc_middle::mir::TerminatorKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::may_run::Runs;
use crate::lower::Body;
use crate::lower::body_queries::strip;
use crate::lower::recognition::{is_local_with, local_key_access, local_key_sets, print_of};

/// The thread-locals made where they're first used, and each other one,
/// made as its module loads, by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    /// Each made as its module loads, as JS's `const x = f();` is, and what
    /// its initializer may run and read there, all `may_run` finds, the
    /// one rule; `None` where it may run what rust-js can't follow. The pipeline asks, of the
    /// modules' imports, where its module's load can't make it
    /// (`load_cycles::out_of_order`, ADR 1006, amended).
    pub(in crate::lower) at_load: HashMap<LocalDefId, Option<Vec<DefId>>>,
}

/// The thread-locals, by their keys, of `inits`, each initializer's key:
/// those made where they're first used already, `made_where_used`, which a
/// lowering of the crate found their module's load can't make; each whose
/// initializer would show it's made as its module loads (`shows`), made
/// where it's first used, a `pub` one refused; and each other, made as its
/// module loads, and what it may run and read there, as `runs` finds of
/// `bodies`.
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    (inits, made_where_used): (&HashMap<LocalDefId, LocalDefId>, &HashSet<LocalDefId>),
    (runs, bodies): (&Runs<'_, 'tcx>, &[&Body<'tcx>]),
) -> LazyLocals {
    let mut found = LazyLocals {
        lazy: made_where_used.clone(),
        at_load: HashMap::new(),
    };
    // What each initializer may run and read as its module loads, all
    // `may_run` finds, which follows each thread-local it reads to its
    // own initializer: the one rule (the user's decision, rd120).
    let mut keyed: Vec<(LocalDefId, LocalDefId)> = (inits.iter())
        .filter(|&(&init, key)| !made_where_used.contains(key) && runs.body(init.to_def_id()).is_some())
        .map(|(&init, &key)| (init, key))
        .collect();
    keyed.sort_by_key(|&(_, key)| tcx.def_span(key));
    for &(init, key) in &keyed {
        found.at_load.insert(key, runs.of(init.to_def_id()));
    }
    // Each that would show it's made as its module loads, made where it's
    // first used, until none more is: a body only it ran runs outside the
    // load then, and what that touches may make another show (#554).
    let touches = Touches::of(tcx, bodies);
    // Each function only calls reach, by its calls (ADR 0577).
    let only_called = super::only_called::only_called(tcx);
    loop {
        let loading = loading(tcx, &keyed, &found.lazy, (bodies, &only_called));
        let outside = Outside::of(&touches, &loading);
        // A namespace thread-local (ADR 1061) is its module's exports, which
        // only JS reads, so its first use is its module's load: made then,
        // it shows nothing Rust would order otherwise.
        let shown: Vec<(LocalDefId, Shows)> = (keyed.iter())
            .filter(|(_, key)| !found.lazy.contains(key))
            .filter(|(_, key)| !crate::lower::classes::is_value_namespace(tcx, key.to_def_id()))
            .filter_map(|&(init, key)| {
                let reached = found.at_load[&key].as_deref()?;
                Some((key, shows(&touches, &outside, init.to_def_id(), reached)?))
            })
            .collect();
        if shown.is_empty() {
            break;
        }
        for (key, why) in shown {
            found.lazy.insert(key);
            found.at_load.remove(&key);
            if tcx.visibility(key).is_public() {
                refuse_exported(tcx, key, why);
            }
        }
    }
    found
}

/// What a body touches of the crate's state, a thread-local of what may
/// change, a `static mut` or a `static` of interior mutability (`changes`),
/// and whether it prints or panics, which Rust orders.
struct Touched {
    /// What it reads or writes, each by its name.
    reads: Vec<DefId>,
    /// What it may write: a thread-local it `set`s, `take`s, `replace`s or
    /// lends, by `with_borrow_mut` or `with`, or hands on; a `static mut` it
    /// writes or takes a pointer to; any `static` of interior mutability.
    writes: Vec<DefId>,
    /// Whether it prints, or calls what never returns, a `panic!`'s.
    ordered: Option<Ordered>,
}

/// What a body does that Rust orders, where it's run.
#[derive(Clone, Copy)]
enum Ordered {
    /// `println!` and the like, std's `_print` or `_eprint`.
    Prints,
    /// A call of what never returns: a `panic!`, an `assert!`'s failure,
    /// `unreachable!()`, `process::exit`.
    Panics,
}

/// Each body's `Touched`, by its id, and the bodies in the crate's order.
struct Touches {
    order: Vec<DefId>,
    of: HashMap<DefId, Touched>,
}

impl Touches {
    fn of<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> Self {
        let mut state: HashMap<DefId, bool> = HashMap::new();
        let mut changing = |id: DefId| *state.entry(id).or_insert_with(|| changes(tcx, id));
        let mut found = Touches {
            order: bodies.iter().map(|body| body.def_id).collect(),
            of: HashMap::new(),
        };
        for body in bodies {
            let thir = &body.thir;
            let mut touched = Touched {
                reads: Vec::new(),
                writes: Vec::new(),
                ordered: None,
            };
            // Each thread-local read where it's only read, `KEY` of
            // `KEY.get()` or `KEY.with_borrow(f)`, and each written, by a
            // method that sets it or lends its cell.
            let mut read_only = HashSet::new();
            for expr in thir.exprs.iter() {
                if let ExprKind::Call { fun, ref args, .. } = expr.kind
                    && let Some((id, _)) = crate::lower::fn_def(thir[fun].ty)
                    && (local_key_access(tcx, id).is_some() || is_local_with(tcx, id))
                    && !local_key_sets(tcx, id)
                    && let Some(&first) = args.first()
                    && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                {
                    read_only.insert(strip(thir, arg));
                }
            }
            for (at, expr) in thir.exprs.iter_enumerated() {
                let written = match expr.kind {
                    ExprKind::NamedConst { def_id, .. } | ExprKind::StaticRef { def_id, .. }
                        if def_id.is_local() && changing(def_id) =>
                    {
                        touched.reads.push(def_id);
                        // A `static mut` is written where it's assigned,
                        // below; anything else that isn't only read may be.
                        if !read_only.contains(&at) && !tcx.is_mutable_static(def_id) {
                            touched.writes.push(def_id);
                        }
                        continue;
                    }
                    ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => lhs,
                    ExprKind::RawBorrow { arg, .. } => arg,
                    ExprKind::Borrow {
                        borrow_kind: rustc_middle::mir::BorrowKind::Mut { .. },
                        arg,
                    } => arg,
                    _ => continue,
                };
                if let Some(def_id) = super::validation::static_of(thir, written)
                    && def_id.is_local()
                    && changing(def_id)
                {
                    touched.writes.push(def_id);
                }
            }
            if let Some(mir) = &body.mir {
                for block in mir.body.basic_blocks.iter().filter(|block| !block.is_cleanup) {
                    let TerminatorKind::Call { func, target, .. } = &block.terminator().kind else {
                        continue;
                    };
                    let printed = func.const_fn_def().is_some_and(|(f, _)| print_of(tcx, f).is_some());
                    touched.ordered = touched.ordered.or(match () {
                        _ if printed => Some(Ordered::Prints),
                        _ if target.is_none() => Some(Ordered::Panics),
                        _ => None,
                    });
                }
            }
            found.of.insert(body.def_id, touched);
        }
        found
    }
}

/// Whether `id`, a thread-local's key or a `static`, is state that may
/// change: a `static mut`; a `static` or a thread-local of what has a
/// cell, through the fields of the crate's types, the type arguments of
/// std's, and what a reference or a pointer points to; a mutable raw
/// pointer. Not a function, a closure or a `dyn`, which holds no state
/// that's named, nor a JS value, a binding's type, `&'static
/// Class<fn(Rc<Fiber>)>`, of what its own fields hold: its JS object a
/// binding's call may change, as what bindings do as a module loads is (the
/// guide's difference 25).
fn changes(tcx: TyCtxt<'_>, id: DefId) -> bool {
    use rustc_hir::def::DefKind;
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let held = match tcx.def_kind(id) {
        DefKind::Static { .. } if tcx.is_mutable_static(id) => return true,
        DefKind::Static { .. } => tcx.type_of(id).instantiate_identity().skip_normalization(),
        DefKind::Const { .. } => {
            let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
            match ty.kind() {
                ty::Adt(key, args)
                    if crate::lower::recognition::is_std_def(
                        tcx,
                        key.did(),
                        crate::lower::recognition::StdItem::LocalKey,
                    ) =>
                {
                    args.type_at(0)
                }
                _ => return false,
            }
        }
        _ => return false,
    };
    let mut todo: Vec<Ty<'_>> = vec![held];
    let mut seen = HashSet::new();
    while let Some(ty) = todo.pop() {
        if !seen.insert(ty) {
            continue;
        }
        if let Some((pointee, mutability)) = crate::lower::pointers::raw_pointee(tcx, ty) {
            if mutability.is_mut() {
                return true;
            }
            todo.push(pointee);
            continue;
        }
        match *ty.kind() {
            ty::Adt(adt, args) if adt.did().is_local() => {
                todo.extend(adt.all_fields().map(|field| field.ty(tcx, args).skip_normalization()));
            }
            ty::Adt(..) if !ty.is_freeze(tcx, typing_env) => return true,
            ty::Adt(adt, args) if crate::lower::recognition::is_std_crate(tcx, adt.did()) => todo.extend(args.types()),
            ty::Ref(_, pointee, _) | ty::Array(pointee, _) | ty::Slice(pointee) => todo.push(pointee),
            ty::Tuple(items) => todo.extend(items.iter()),
            _ => {}
        }
    }
    false
}

/// The bodies that run only as the modules load: each thread-local's
/// initializer made then, of `keyed`'s whose key isn't `lazy`, each
/// `js::on_load!`'s; a function only calls reach, of `only_called`, each
/// of whose calls is in one of those; and a closure one of those gives std
/// to run there (`run_in_place`).
fn loading<'tcx>(
    tcx: TyCtxt<'tcx>,
    keyed: &[(LocalDefId, LocalDefId)],
    lazy: &HashSet<LocalDefId>,
    (bodies, only_called): (&[&Body<'tcx>], &HashMap<DefId, Vec<hir::HirId>>),
) -> HashSet<DefId> {
    let mut loading: HashSet<DefId> = (keyed.iter())
        .filter(|(_, key)| !lazy.contains(key))
        .map(|(init, _)| init.to_def_id())
        .chain(
            (bodies.iter())
                .map(|body| body.def_id)
                .filter(|&id| crate::lower::bindings::is_on_load(tcx, id)),
        )
        .collect();
    let in_place: Vec<(DefId, DefId)> = (bodies.iter())
        .flat_map(|body| {
            run_in_place(tcx, body)
                .into_iter()
                .map(|closure| (closure, body.def_id))
        })
        .collect();
    loop {
        let before = loading.len();
        for (&f, calls) in only_called {
            if !loading.contains(&f)
                && (calls.iter()).all(|&call| loading.contains(&tcx.hir_enclosing_body_owner(call).to_def_id()))
            {
                loading.insert(f);
            }
        }
        for &(closure, at) in &in_place {
            if loading.contains(&at) {
                loading.insert(closure);
            }
        }
        if loading.len() == before {
            return loading;
        }
    }
}

/// Each closure `body` gives std's call, made there, that the call runs
/// before it returns, or never, keeping none of it: one whose type is in
/// neither what the call gives back nor what else it's given, as a
/// thread-local's `with(|x| ..)`'s or `Option::map(|x| ..)`'s is, where an
/// iterator's `map` keeps it in what it gives, and a `Vec`'s `push` in what
/// it's given. Std runs it, if at all, where `body` calls it.
fn run_in_place<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<DefId> {
    let thir = &body.thir;
    let mut found = Vec::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let Some((id, fn_args)) = crate::lower::fn_def(thir[fun].ty) else {
            continue;
        };
        if !crate::lower::recognition::is_std_crate(tcx, id) {
            continue;
        }
        let sig =
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate(tcx, fn_args).skip_normalization());
        for (at, &arg) in args.iter().enumerate() {
            let ExprKind::Closure(ref closure) = thir[strip(thir, arg)].kind else {
                continue;
            };
            let closure = closure.closure_id.to_def_id();
            let holds = |ty: Ty<'tcx>| {
                ty.walk().any(
                    |part| matches!(part.as_type().map(|part| *part.kind()), Some(ty::Closure(c, _)) if c == closure),
                )
            };
            let kept = holds(sig.output())
                || (sig.inputs().iter().enumerate()).any(|(other, &input)| other != at && holds(input));
            if !kept {
                found.push(closure);
            }
        }
    }
    found
}

/// What the bodies that may run outside the modules' load touch: each
/// state they may write, by the first that does, and each they read or
/// write, by the first that does.
struct Outside {
    writes: HashMap<DefId, DefId>,
    touches: HashMap<DefId, DefId>,
}

impl Outside {
    fn of(touches: &Touches, loading: &HashSet<DefId>) -> Self {
        let mut outside = Outside {
            writes: HashMap::new(),
            touches: HashMap::new(),
        };
        for id in touches.order.iter().filter(|id| !loading.contains(id)) {
            let touched = &touches.of[id];
            for &state in &touched.writes {
                outside.writes.entry(state).or_insert(*id);
            }
            for &state in touched.reads.iter().chain(&touched.writes) {
                outside.touches.entry(state).or_insert(*id);
            }
        }
        outside
    }
}

/// Why making a thread-local as its module loads would show, where Rust
/// makes it at its first use.
enum Shows {
    /// Its initializer runs a body, the first, that prints or panics.
    Ordered(DefId, Ordered),
    /// It reads state, the first, that a body that may run after the
    /// modules load, the second, writes.
    Reads(DefId, DefId),
    /// It writes state, the first, that a body that may run after the
    /// modules load, the second, reads or writes.
    Writes(DefId, DefId),
}

/// Whether making `init`'s thread-local as its module loads would show:
/// of what it runs and reads there, `reached`, a body that prints or
/// panics; state it reads that what may run outside the load, `outside`,
/// writes; or state it writes that that touches (the user's decision,
/// #554).
fn shows(touches: &Touches, outside: &Outside, init: DefId, reached: &[DefId]) -> Option<Shows> {
    let ran = || {
        std::iter::once(&init)
            .chain(reached)
            .filter_map(|id| Some((*id, touches.of.get(id)?)))
    };
    if let Some((id, ordered)) = ran().find_map(|(id, touched)| Some((id, touched.ordered?))) {
        return Some(Shows::Ordered(id, ordered));
    }
    let read = ran().flat_map(|(_, touched)| touched.reads.iter());
    if let Some((&state, &by)) = read.clone().find_map(|state| Some((state, outside.writes.get(state)?))) {
        return Some(Shows::Reads(state, by));
    }
    let written = ran().flat_map(|(_, touched)| touched.writes.iter());
    written
        .clone()
        .find_map(|state| Some((state, outside.touches.get(state)?)))
        .map(|(&state, &by)| Shows::Writes(state, by))
}

/// A `pub` thread-local whose initializer would show it's made as its
/// module loads, `why`: refused, as JS's importers read it as an `export
/// const`, made then (ADR 1006, amended, #554).
fn refuse_exported(tcx: TyCtxt<'_>, key: LocalDefId, why: Shows) {
    let span = tcx.def_ident_span(key).unwrap_or_else(|| tcx.def_span(key));
    let name = |id: DefId| tcx.def_path_str(tcx.typeck_root_def_id(id));
    let what = match why {
        Shows::Ordered(by, Ordered::Prints) => {
            format!("a `pub` thread-local whose initializer prints, in `{}`", name(by))
        }
        Shows::Ordered(by, Ordered::Panics) => {
            format!("a `pub` thread-local whose initializer may panic, in `{}`", name(by))
        }
        Shows::Reads(state, by) => format!(
            "a `pub` thread-local whose initializer reads `{}`, which `{}` may change after the modules load",
            name(state),
            name(by)
        ),
        Shows::Writes(state, by) => format!(
            "a `pub` thread-local whose initializer changes `{}`, which `{}` may read after the modules load",
            name(state),
            name(by)
        ),
    };
    let why = "JS's importers read it as an `export const`, made as its module loads, where Rust makes it at its first use: make it `pub(crate)`, made where it's first used, and export a `pub fn` that reads it (ADR 1006)";
    crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(&what, why));
}

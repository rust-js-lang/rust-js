//! What each of the crate's bodies may borrow, of its cells (ADR 0580): a
//! `RefCell`, a `Mutex` or an `RwLock`, by the thread-local it's held in
//! or by its type, or anything, of code rust-js can't see. A borrow asks
//! whether its cell is borrowed only of one it may be, so a guard held
//! while the crate's own functions run is checked and not counted where
//! none of them borrows its cell (ADR 0328): `TIMER_QUEUE`'s while
//! `TASK_QUEUE`'s is taken, or the heap's `push` runs.

use crate::lower::Body;
use crate::lower::bindings::{JsForm, is_binding, js_form, runs_nothing, runs_only_given};
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{Recognition, Std, StdItem, is_js_object_deref};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// A cell a borrow is of: a thread-local's, by its key, or any of a type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::lower) enum Cell<'tcx> {
    Key(DefId),
    Type(Ty<'tcx>),
}

/// A borrow of a cell: which, and whether mutably. A shared borrow beside
/// a shared one asks only whether it's mutably borrowed, which neither
/// changes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::lower) struct Borrow<'tcx> {
    pub(in crate::lower) cell: Cell<'tcx>,
    pub(in crate::lower) mutable: bool,
}

/// What running a body may borrow: its cells, and the values of its type
/// parameters' it may drop, whose destructors are its caller's to say.
#[derive(Clone, PartialEq, Default, Debug)]
struct Summary<'tcx> {
    cells: HashSet<Borrow<'tcx>>,
    drops: HashSet<Ty<'tcx>>,
}

/// What each body of the crate may borrow, its calls' and its closures'
/// with it; `None`, anything.
#[derive(Default)]
pub(in crate::lower) struct CellBorrows<'tcx> {
    of: HashMap<DefId, Option<Summary<'tcx>>>,
    /// Each of the crate's types' impls of a trait, by the type: what std's
    /// code bounding one by the trait may run of it.
    impls: Impls,
    /// What each fn pointer it calls may be (ADR 0907).
    pointers: super::fn_pointers::FnPointers<'tcx>,
    /// The type parameters whose `Clone` bound takes its dictionary, whose
    /// clone may run code of the crate's (ADR 0914, amended).
    clones: HashSet<(DefId, u32)>,
    /// What code rust-js can't see may borrow of a thread-local of the
    /// crate's that only its code reaches (`closed`): what the crate's
    /// bodies borrow but what its modules run as they load, which runs
    /// once, before JS can call into it (ADR 0580, amended).
    later: HashSet<Borrow<'tcx>>,
    /// The crate's thread-locals no code but the crate's reaches: not
    /// visible outside it, each use a read or a write of what it holds,
    /// never its cell lent, `with`'s, or handed on.
    closed: HashSet<DefId>,
}

/// Each of the crate's ADTs' trait impls: the trait, and the impl.
type Impls = HashMap<DefId, Vec<(DefId, DefId)>>;

/// What a body does itself: what it borrows and drops, the bodies of the
/// crate's it runs, of their generic arguments, and whether it may run what
/// rust-js can't see.
#[derive(Default)]
struct Direct<'tcx> {
    own: Summary<'tcx>,
    runs: Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>,
    any: bool,
}

impl<'tcx> CellBorrows<'tcx> {
    /// Each body's, until what each runs holds; `drops` says whether
    /// dropping a type runs a destructor (ADR 0320).
    pub(in crate::lower) fn of_crate(
        tcx: TyCtxt<'tcx>,
        recognition: &Recognition<'_, 'tcx>,
        (bodies, clones): (&[&Body<'tcx>], &HashSet<(DefId, u32)>),
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> Self {
        let mut impls: Impls = HashMap::new();
        for (&trait_id, of_trait) in tcx.all_local_trait_impls(()) {
            for &imp in of_trait {
                if let ty::Adt(adt, _) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
                    impls.entry(adt.did()).or_default().push((trait_id, imp.to_def_id()));
                }
            }
        }
        // What each fn pointer the crate calls may be (ADR 0907).
        let pointers = super::fn_pointers::FnPointers::of_crate(recognition);
        let mut found = CellBorrows {
            impls,
            pointers,
            clones: clones.clone(),
            ..CellBorrows::default()
        };
        let direct: HashMap<DefId, Direct<'tcx>> = (bodies.iter())
            .filter(|body| body.root().is_some())
            .map(|body| {
                let recognition = Recognition {
                    typing_env: ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body.def_id)),
                    ..*recognition
                };
                let itself = |ty| found.clones_itself(&recognition, body.def_id, ty);
                (
                    body.def_id,
                    direct(
                        &recognition,
                        (&found.impls, &found.pointers),
                        &body.thir,
                        (drops, &itself),
                    ),
                )
            })
            .collect();
        let mut of: HashMap<DefId, Option<Summary<'tcx>>> = (direct.iter())
            .map(|(&id, d)| (id, (!d.any).then(|| d.own.clone())))
            .collect();
        loop {
            let mut changed = false;
            for (&id, d) in &direct {
                let Some(mut next) = of[&id].clone() else { continue };
                let any = (d.runs.iter()).any(|&(run, args)| match of.get(&run) {
                    Some(Some(theirs)) => match instantiated(tcx, theirs, args, drops) {
                        Some(theirs) => {
                            next.cells.extend(theirs.cells);
                            next.drops.extend(theirs.drops);
                            false
                        }
                        None => true,
                    },
                    // What isn't the crate's own, or may borrow anything.
                    _ => true,
                });
                let next = (!any).then_some(next);
                if next != of[&id] {
                    of.insert(id, next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        found.of = of;
        // What may run after the crate's modules have loaded: every body
        // but what a module runs as it loads, `js::on_load!`'s own, whose
        // closures are bodies of their own.
        found.later = (direct.iter())
            .filter(|&(&id, _)| !crate::lower::bindings::is_on_load(tcx, id))
            .flat_map(|(_, d)| d.own.cells.iter().copied())
            .collect();
        found.closed = closed_keys(tcx, bodies);
        found
    }

    /// Whether code rust-js can't see, JS's or a library's, may ask of
    /// `held`: of a thread-local only the crate's code reaches (`closed`),
    /// only where a body that may run after its modules load borrows it so
    /// (ADR 0580, amended), by its key: a borrow of a cell of its type
    /// through a reference is never of its own, which none reaches (ADR
    /// 0960, amended); of any other cell, it may.
    fn anything_asks(&self, tcx: TyCtxt<'tcx>, held: Borrow<'tcx>) -> bool {
        match held.cell {
            Cell::Key(key) if self.closed.contains(&key) => {
                (self.later.iter()).any(|&theirs| asks(tcx, &self.closed, theirs, held))
            }
            _ => true,
        }
    }

    /// Whether `key`'s thread-local is one only its key reaches (`closed`):
    /// its own cell, which no `with` lends, is no `&RefCell` of its type
    /// anywhere, so a borrow of one is never of it (ADR 0960, amended).
    pub(in crate::lower) fn reached_only_by_key(&self, key: DefId) -> bool {
        self.closed.contains(&key)
    }

    /// Whether `Clone::clone` of `ty` in `owner`'s body is the value itself,
    /// running nothing: a type parameter's whose bound takes no dictionary
    /// (`clones_by_itself`, ADR 0914, amended).
    fn clones_itself(&self, recognition: &Recognition<'_, 'tcx>, owner: DefId, ty: Ty<'tcx>) -> bool {
        let tcx = recognition.tcx;
        matches!(ty.kind(), ty::Param(param)
            if super::clones_by_itself(tcx, recognition.foreign, &self.clones, (owner, param.index))
                && super::mutation::clone_bounded(tcx, tcx.typeck_root_def_id(owner)).contains(&param.index))
    }

    /// Whether running `body`, a closure's, may ask of `held`, a borrow.
    pub(in crate::lower) fn body_asks(&self, tcx: TyCtxt<'tcx>, body: DefId, held: Borrow<'tcx>) -> bool {
        match self.of.get(&body) {
            Some(Some(summary)) => summary.asks(tcx, &self.closed, held),
            _ => self.anything_asks(tcx, held),
        }
    }

    /// Whether calling `def_id`, of `args`, may ask of `held`: what a body of
    /// the crate's it runs may, or std's borrow of a cell, its receiver;
    /// anything else, which may run code of the crate's, may. `drops` says
    /// whether dropping a type runs a destructor.
    pub(in crate::lower) fn call_asks(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (owner, def_id, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
        receiver: Option<(&Thir<'tcx>, ExprId)>,
        held: Borrow<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        let tcx = recognition.tcx;
        let itself = |ty| self.clones_itself(recognition, owner, ty);
        match called(recognition, def_id, args, receiver, &itself) {
            Called::Nothing => false,
            Called::Cell(theirs) => asks(tcx, &self.closed, theirs, held),
            Called::Body(body, args) => self.runs_asks(tcx, body, Some(args), held, drops),
            Called::Std(id, args) => {
                let given = std_given(recognition, &self.impls, id, args);
                (given.any && self.anything_asks(tcx, held))
                    || given.cells.iter().any(|&theirs| asks(tcx, &self.closed, theirs, held))
                    || (given.bodies.iter()).any(|&(body, args)| self.runs_asks(tcx, body, args, held, drops))
            }
            Called::Any => self.anything_asks(tcx, held),
        }
    }

    /// Whether `cell`'s own impls may ask of it (ADR 0863): a `RefCell<T>`'s
    /// `Clone`, `PartialEq`, `PartialOrd`, `Ord` and `Debug` hold a borrow
    /// of it while `T`'s run, which may borrow it mutably, or run what
    /// rust-js can't see. Only those it has: a `RefCell<Vec<&dyn Fn()>>`
    /// has no `PartialEq` nor `Debug`, as a `dyn Fn()` hasn't, so none of
    /// them runs, nor what a `dyn Fn()` would run of them.
    pub(in crate::lower) fn impls_ask(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        cell: Ty<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        let tcx = recognition.tcx;
        let ty::Adt(_, args) = cell.kind() else {
            return true;
        };
        let items = tcx.lang_items();
        let traits = [
            items.clone_trait(),
            items.eq_trait(),
            items.partial_ord_trait(),
            Some(crate::lower::recognition::std_item(tcx, StdItem::Ord)),
            Some(crate::lower::recognition::std_item(tcx, StdItem::Debug)),
        ];
        // Whether `cell` has `trait_id`'s impl, `PartialEq<Self>`'s of
        // `PartialEq`: where what it is isn't known, of a type parameter,
        // it may.
        let has = |trait_id: DefId| {
            if cell.has_param() || cell.has_aliases() {
                return true;
            }
            let of: Vec<ty::GenericArg<'tcx>> = match tcx.generics_of(trait_id).count() {
                1 => vec![cell.into()],
                _ => vec![cell.into(), cell.into()],
            };
            let tr = ty::TraitRef::new(tcx, trait_id, of);
            !matches!(
                tcx.codegen_select_candidate(recognition.typing_env.as_query_input(tr)),
                Err(rustc_middle::traits::CodegenObligationError::Unimplemented)
            )
        };
        let mut given = Given::default();
        let mut seen = HashSet::new();
        for trait_id in traits.into_iter().flatten().filter(|&t| has(t)) {
            bound_runs(
                recognition,
                &self.impls,
                (args.type_at(0), trait_id),
                &mut seen,
                &mut given,
            );
        }
        let held = Borrow {
            cell: Cell::Type(cell),
            mutable: false,
        };
        given.any
            || given.cells.iter().any(|&theirs| asks(tcx, &self.closed, theirs, held))
            || (given.bodies.iter()).any(|&(body, args)| self.runs_asks(tcx, body, args, held, drops))
    }

    /// Whether calling `def_id`, of `args`, may run code rust-js can't see
    /// (ADR 0863): a library's, JS's, or a function's it's given as a
    /// value; a body of the crate's that may, or std's that may run one.
    pub(in crate::lower) fn call_runs_unseen(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (owner, def_id, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    ) -> bool {
        let summarized = |body: &DefId| matches!(self.of.get(body), Some(Some(_)));
        let itself = |ty| self.clones_itself(recognition, owner, ty);
        // A cell's borrow, `RefCell::borrow_mut`, a lock's, runs nothing but
        // its own check, whichever cell it is (ADR 0863, amended): which,
        // `called` asks of its receiver, a borrow's question, not this one.
        if matches!(
            recognition.classify(def_id, args),
            Some(crate::lower::Std::Borrow { .. } | crate::lower::Std::TryBorrow { .. })
        ) {
            return false;
        }
        match called(recognition, def_id, args, None, &itself) {
            Called::Nothing | Called::Cell(_) => false,
            Called::Body(body, _) => !summarized(&body),
            Called::Std(id, args) => {
                let given = std_given(recognition, &self.impls, id, args);
                given.any || !given.bodies.iter().all(|(body, _)| summarized(body))
            }
            Called::Any => true,
        }
    }

    /// What each fn pointer the crate calls may be (ADR 0907), its owner's
    /// answers, for another analysis to ask.
    pub(in crate::lower) fn pointers(&self) -> &super::fn_pointers::FnPointers<'tcx> {
        &self.pointers
    }

    /// Whether calling a fn pointer of type `ptr` may ask of `held` (ADR
    /// 0907): what calling any function or closure it may be may.
    pub(in crate::lower) fn pointer_call_asks(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        ptr: Ty<'tcx>,
        held: Borrow<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        let tcx = recognition.tcx;
        let Some(may_be) = self.pointers.may_be(tcx, ptr) else {
            return self.anything_asks(tcx, held);
        };
        may_be.iter().any(|&(f, args)| match tcx.is_closure_like(f) {
            true => self.runs_asks(tcx, f, Some(args), held, drops),
            false => self.call_asks(recognition, (f, f, args), None, held, drops),
        })
    }

    /// Whether calling a fn pointer of type `ptr` may run code rust-js
    /// can't see (ADR 0907): what it may be may, or it may be anything.
    pub(in crate::lower) fn pointer_call_runs_unseen(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        ptr: Ty<'tcx>,
    ) -> bool {
        let tcx = recognition.tcx;
        let Some(may_be) = self.pointers.may_be(tcx, ptr) else {
            return true;
        };
        may_be.iter().any(|&(f, args)| match tcx.is_closure_like(f) {
            true => !matches!(self.of.get(&f), Some(Some(_))),
            false => self.call_runs_unseen(recognition, (f, f, args)),
        })
    }

    /// Whether running `body` of the crate's, of `args`, may ask of `held`.
    fn runs_asks(
        &self,
        tcx: TyCtxt<'tcx>,
        body: DefId,
        args: Option<ty::GenericArgsRef<'tcx>>,
        held: Borrow<'tcx>,
        drops: &dyn Fn(Ty<'tcx>) -> bool,
    ) -> bool {
        match self.of.get(&body) {
            Some(Some(theirs)) => match instantiated(tcx, theirs, args, drops) {
                Some(theirs) => theirs.asks(tcx, &self.closed, held),
                None => self.anything_asks(tcx, held),
            },
            _ => self.anything_asks(tcx, held),
        }
    }
}

impl<'tcx> Summary<'tcx> {
    /// Whether it may ask of `held`: one of its borrows may, or a drop its
    /// caller says may run anything.
    fn asks(&self, tcx: TyCtxt<'tcx>, closed: &HashSet<DefId>, held: Borrow<'tcx>) -> bool {
        !self.drops.is_empty() || self.cells.iter().any(|&theirs| asks(tcx, closed, theirs, held))
    }
}

/// A body's summary as a call of `args` runs it: its type parameters'
/// cells and drops of the call's types; `None` where a drop known now runs
/// a destructor, which may run anything.
fn instantiated<'tcx>(
    tcx: TyCtxt<'tcx>,
    summary: &Summary<'tcx>,
    args: Option<ty::GenericArgsRef<'tcx>>,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> Option<Summary<'tcx>> {
    // Of a body run of no args a type says, an impl's method std's code
    // runs, whose types are of its own parameters: not known (#240).
    let typed = |ty: &Ty<'tcx>| ty.has_param();
    if args.is_none()
        && (summary.drops.iter().any(typed)
            || summary
                .cells
                .iter()
                .any(|borrow| matches!(borrow.cell, Cell::Type(ty) if ty.has_param())))
    {
        return None;
    }
    let of = |ty: Ty<'tcx>| match args {
        Some(args) if ty.has_param() => tcx.erase_and_anonymize_regions(
            ty::EarlyBinder::bind(tcx, ty)
                .instantiate(tcx, args)
                .skip_normalization(),
        ),
        _ => ty,
    };
    let cells = (summary.cells.iter())
        .map(|&borrow| Borrow {
            cell: match borrow.cell {
                Cell::Type(ty) => Cell::Type(of(ty)),
                key => key,
            },
            ..borrow
        })
        .collect();
    let mut kept = HashSet::new();
    for &ty in &summary.drops {
        let ty = of(ty);
        match ty.has_param() {
            true => _ = kept.insert(ty),
            false if drops(ty) => return None,
            false => {}
        }
    }
    Some(Summary { cells, drops: kept })
}

/// The borrow `call`, a borrow's call, is: `c.borrow_mut()`'s of `c`, a
/// thread-local's by its key.
pub(in crate::lower) fn borrowed<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    thir: &Thir<'tcx>,
    call: ExprId,
) -> Option<Borrow<'tcx>> {
    let ExprKind::Call { fun, ref args, .. } = thir[call].kind else {
        return None;
    };
    let (id, generic_args) = fn_def(thir[fun].ty)?;
    match called(recognition, id, generic_args, args.first().map(|&a| (thir, a)), &|_| {
        false
    }) {
        Called::Cell(cell) => Some(cell),
        _ => None,
    }
}

/// What a call runs, as it borrows a cell.
enum Called<'tcx> {
    /// Nothing that borrows: std's, a binding that makes a value or reads
    /// one.
    Nothing,
    /// A borrow of a cell, std's.
    Cell(Borrow<'tcx>),
    /// A body of the crate's, of these generic arguments.
    Body(DefId, ty::GenericArgsRef<'tcx>),
    /// A function of std's, of these generic arguments, which runs what its
    /// bounds say of them (`std_given`).
    Std(DefId, ty::GenericArgsRef<'tcx>),
    /// What rust-js can't see.
    Any,
}

/// Whether a binding runs no code of the crate's: what makes a JSX element
/// or an object, or casts a value, a component given called when React
/// renders it; a read of a property, `node.sortIndex`, `in` or
/// `instanceof`, but by a JS getter that calls back into Rust; whether the
/// host has a global, `typeof X !== "undefined"`; JS's `null`, `js::null()`;
/// and one that says it runs nothing, `#[rust_js::runs_nothing]` (ADR 0867).
fn binding_runs_nothing(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_binding(tcx, id)
        && (matches!(
            js_form(tcx, id),
            JsForm::Jsx(_)
                | JsForm::Prop(_)
                | JsForm::Object(_)
                | JsForm::This
                | JsForm::Get(_)
                | JsForm::GetIndex
                | JsForm::In
                | JsForm::InstanceOf(_)
                | JsForm::InstanceOfGiven
                | JsForm::Truthy
                | JsForm::StrictEquals
                | JsForm::Defined
                | JsForm::Function
                | JsForm::TypeofObject
                | JsForm::Null
        ) || runs_nothing(tcx, id))
}

/// Whether `def_id`, a binding that runs no code but its own and what it's
/// given, `#[rust_js::runs_only_given]` (ADR 0898), is given, of `args`,
/// only what runs nothing as JS reads it: numbers, `bool`s, `char`s, text,
/// and `Option`s, tuples, arrays, slices and `Vec`s of them, which
/// `String(x)` and `+x` read with no code of anyone's. Not an object of the
/// crate's, whose getter or `toString` may be its code, nor a JS value,
/// which may be a function or a proxy of the crate's.
fn given_what_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    let tcx = recognition.tcx;
    if !is_binding(tcx, def_id) || !runs_only_given(tcx, def_id) {
        return false;
    }
    fn runs_nothing<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => true,
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => runs_nothing(recognition, *inner),
            ty::Tuple(items) => items.iter().all(|item| runs_nothing(recognition, item)),
            ty::Adt(_, _) if recognition.is_lang_adt(ty, LangItem::String) => true,
            ty::Adt(_, items)
                if recognition.is_lang_adt(ty, LangItem::Option) || recognition.is_std_type(ty, StdItem::Vec) =>
            {
                runs_nothing(recognition, items.type_at(0))
            }
            _ => false,
        }
    }
    let sig = tcx.fn_sig(def_id).instantiate(tcx, args).skip_normalization();
    (sig.skip_binder().inputs().iter()).all(|&input| runs_nothing(recognition, input))
}

/// Whether a call of `def_id`, a binding, of `args`, runs none of the
/// crate's code: one that runs nothing (`binding_runs_nothing`), or only
/// what it's given, given only what runs nothing
/// (`given_what_runs_nothing`). So a borrow held across it meets no other,
/// and it sets none of the crate's variables, which lowering marks its
/// call so (`Known::RunsNothing`, ADR 0277, amended).
pub(in crate::lower) fn binding_call_runs_nothing<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    binding_runs_nothing(recognition.tcx, def_id) || given_what_runs_nothing(recognition, def_id, args)
}

/// What calling `def_id`, of `args`, runs; `receiver` its first argument,
/// the cell a borrow is of.
fn called<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
    receiver: Option<(&Thir<'tcx>, ExprId)>,
    clones_itself: &dyn Fn(Ty<'tcx>) -> bool,
) -> Called<'tcx> {
    let tcx = recognition.tcx;
    // A clone of what's its own clone, a type parameter's whose `Clone`
    // bound takes no dictionary, is the value: it runs nothing (ADR 0914,
    // amended).
    if tcx.is_lang_item(def_id, LangItem::CloneFn) && args.types().next().is_some_and(clones_itself) {
        return Called::Nothing;
    }
    // A trait's binding, of a type parameter its bound reads (ADR 0307).
    if !tcx.is_closure_like(def_id) && binding_call_runs_nothing(recognition, def_id, args) {
        return Called::Nothing;
    }
    let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, recognition.typing_env, def_id, args) else {
        return Called::Any;
    };
    // An intrinsic, the compiler's own, runs only what it's given: a
    // function by a bound, `const_eval_select`'s, as std's code does; and
    // a pointer to one or a `dyn`, `catch_unwind`'s, which may be any.
    // `vec![x]`'s `write_box_via_move` moves `x` into its box.
    if let ty::InstanceKind::Intrinsic(id) = instance.def {
        let sig = tcx.fn_sig(id).instantiate(tcx, instance.args).skip_normalization();
        let given_code = (sig.skip_binder().inputs().iter())
            .flat_map(|input| input.walk().filter_map(|part| part.as_type()))
            .any(|part| matches!(part.kind(), ty::FnPtr(..) | ty::Dynamic(..)));
        return match given_code {
            true => Called::Any,
            false => Called::Std(id, instance.args),
        };
    }
    let ty::InstanceKind::Item(id) = instance.def else {
        return Called::Any;
    };
    // A closure of the crate's, called by its `Fn` trait's method.
    if tcx.is_closure_like(id) {
        return match id.is_local() {
            true => Called::Body(id, instance.args),
            false => Called::Any,
        };
    }
    // A borrow, of the cell its receiver is: a thread-local's by its key.
    // A thread-local's own, but its `with_borrow`, may change it.
    let refcell = receiver.is_some_and(|(thir, r)| recognition.is_std_type(thir[r].ty.peel_refs(), StdItem::RefCell));
    let mutable = match recognition.classify(id, instance.args) {
        Some(Std::Borrow { mutable, .. } | Std::TryBorrow { mutable, .. } | Std::LocalBorrow { mutable }) => {
            Some(mutable)
        }
        // A `RefCell`'s own write borrows it mutably while it runs. A
        // `swap`'s other cell is lent, a cell whose borrows are counted.
        Some(Std::CellReplace | Std::CellTake | Std::CellSwap | Std::CellReplaceWith) if refcell => Some(true),
        _ if recognition.asks_borrows(id) => Some(true),
        _ => None,
    };
    if let Some(mutable) = mutable {
        let Some((thir, receiver)) = receiver else {
            return Called::Any;
        };
        let held = thir[receiver].ty.peel_refs();
        let cell = match thir[strip(thir, receiver)].kind {
            ExprKind::Borrow { arg, .. }
                if let ExprKind::NamedConst { def_id: key, .. } = thir[strip(thir, arg)].kind =>
            {
                Cell::Key(key)
            }
            _ => match held.kind() {
                ty::Adt(_, key_args) if recognition.is_std_type(held, StdItem::LocalKey) => {
                    match key_args.types().next() {
                        Some(cell) => Cell::Type(tcx.erase_and_anonymize_regions(cell)),
                        None => return Called::Any,
                    }
                }
                _ if recognition.is_borrowed_cell(held) => Cell::Type(tcx.erase_and_anonymize_regions(held)),
                _ => return Called::Any,
            },
        };
        return Called::Cell(Borrow { cell, mutable });
    }
    // A JS object type's `Deref` to the class it extends is the object
    // itself, `AggregateError`'s to `Error`.
    if binding_runs_nothing(tcx, id) || is_js_object_deref(tcx, tcx.parent(id)) {
        return Called::Nothing;
    }
    // A method std derives, `Clone::clone` of `#[derive(Clone)]`, runs what
    // it reads of its fields by its trait, as std's code does (ADR 0868).
    if id.is_local() && derived_method(tcx, id).is_some() {
        return Called::Std(id, instance.args);
    }
    match id.is_local() && !is_binding(tcx, id) {
        true => Called::Body(id, instance.args),
        false if recognition.is_std(id) => Called::Std(id, instance.args),
        false => Called::Any,
    }
}

/// What std's code may run of the crate's, called of some generic
/// arguments, by the bounds it's called under: it calls what it's given
/// only through them (ADR 0328).
#[derive(Default)]
struct Given<'tcx> {
    /// The cells it borrows, a `RefCell<T>`'s by `RefCell<T>: Clone`, say.
    cells: Vec<Borrow<'tcx>>,
    /// The crate's bodies it may run, and of what args where its type says:
    /// a closure or function it's given, an impl's method, whose args no
    /// type says (#240).
    bodies: Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>,
    /// What rust-js can't see: a type parameter's, a `dyn`'s or a
    /// library's type's impl.
    any: bool,
}

/// What `id`, std's, of `args`, may run of the crate's code (`Given`): for
/// each trait with methods that bounds a type it's given, `F: FnMut(&T) ->
/// bool` or `T: Clone`, the closure or function, the crate's impl of it,
/// through what a derive reads of its fields, the cell whose impl borrows
/// it, or what rust-js can't see.
fn std_given<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    impls: &Impls,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Given<'tcx> {
    let tcx = recognition.tcx;
    let mut given = Given::default();
    let mut seen = HashSet::new();
    // A derived method reads its type's fields by its trait.
    if let Some(imp) = derived_method(tcx, id) {
        let own = tcx.mk_args(&args[..tcx.generics_of(imp).count().min(args.len())]);
        let ty = tcx.type_of(imp).instantiate(tcx, own).skip_normalization();
        let trait_id = tcx
            .impl_trait_ref(imp)
            .instantiate_identity()
            .skip_normalization()
            .def_id;
        bound_runs(recognition, impls, (ty, trait_id), &mut seen, &mut given);
        return given;
    }
    for (clause, _) in tcx.clauses_of(id).instantiate(tcx, args) {
        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
            continue;
        };
        let bound = bound.skip_binder();
        bound_runs(
            recognition,
            impls,
            (bound.self_ty(), bound.def_id()),
            &mut seen,
            &mut given,
        );
    }
    // A cell's own trait impl's method borrows it: `RefCell`'s
    // `PartialEq::eq` borrows both it's given, its `Clone::clone` the one
    // it clones (ADR 0863).
    if let Some(imp) = tcx.trait_impl_of_assoc(id) {
        let own = tcx.mk_args(&args[..tcx.generics_of(imp).count().min(args.len())]);
        let cell = tcx.type_of(imp).instantiate(tcx, own).skip_normalization();
        if recognition.is_borrowed_cell(cell) {
            given.cells.push(Borrow {
                cell: Cell::Type(tcx.erase_and_anonymize_regions(cell)),
                mutable: !recognition.is_std_type(cell, StdItem::RefCell),
            });
        }
    }
    given
}

/// The impl std derives that `id`, the crate's, is a method of:
/// `#[derive(Clone)]`'s `clone`.
fn derived_method(tcx: TyCtxt<'_>, id: DefId) -> Option<DefId> {
    tcx.trait_impl_of_assoc(id)
        .filter(|&imp| crate::lower::recognition::known_derive(tcx, imp))
}

/// Whether a trait or one it extends has a method: what a bound by it may
/// run. `Sized`, `Copy` and `Send` have none.
fn has_methods(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    rustc_middle::ty::elaborate::supertrait_def_ids(tcx, trait_id).any(|t| {
        // A trait alias, `ptr::Thin`, may stand for any.
        tcx.def_kind(t) != DefKind::Trait
            || (tcx.associated_item_def_ids(t).iter()).any(|&item| matches!(tcx.def_kind(item), DefKind::AssocFn))
    })
}

/// What std's code may run of `ty` by its bound `trait_id` (`std_given`).
fn bound_runs<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    impls: &Impls,
    (ty, trait_id): (Ty<'tcx>, DefId),
    seen: &mut HashSet<(Ty<'tcx>, DefId)>,
    given: &mut Given<'tcx>,
) {
    let tcx = recognition.tcx;
    if given.any || !seen.insert((ty, trait_id)) || !has_methods(tcx, trait_id) {
        return;
    }
    let traits: Vec<DefId> = rustc_middle::ty::elaborate::supertrait_def_ids(tcx, trait_id).collect();
    let mut walk = ty.walk();
    while let Some(part) = walk.next() {
        let Some(part) = part.as_type() else { continue };
        // A raw pointer's, which std's code reads nothing of.
        if crate::lower::pointers::raw_pointee(tcx, part).is_some() {
            continue;
        }
        // A type std's impls are of runs of what it's made of what those
        // impls' bounds say, as std's functions do (ADR 0868): `Rc<T>`'s
        // `Clone` clones no `T`, nor a `&T`'s, which copies it.
        if let Some(bounds) = std_impl_bounds(recognition, part, &traits) {
            walk.skip_current_subtree();
            for bound in bounds {
                bound_runs(recognition, impls, bound, seen, given);
            }
            continue;
        }
        match *part.kind() {
            // Called, by an `Fn` trait; or, by another, `F: Clone`, what it
            // took is. Its signature, among its arguments, isn't given.
            ty::Closure(closure, closure_args) => {
                walk.skip_current_subtree();
                if !closure.is_local() {
                    given.any = true;
                    continue;
                }
                given.bodies.push((closure, Some(closure_args)));
                if tcx.fn_trait_kind_from_def_id(trait_id).is_none() {
                    for upvar in closure_args.as_closure().upvar_tys() {
                        bound_runs(recognition, impls, (upvar, trait_id), seen, given);
                    }
                }
            }
            ty::CoroutineClosure(closure, args) | ty::Coroutine(closure, args) => {
                walk.skip_current_subtree();
                match closure.is_local() {
                    true => given.bodies.push((closure, Some(args))),
                    false => given.any = true,
                }
            }
            ty::FnDef(f, args) if f.is_local() && !is_binding(tcx, f) => {
                given.bodies.push((f, Some(args.skip_binder())))
            }
            ty::Adt(..) if recognition.is_borrowed_cell(part) => given.cells.push(Borrow {
                cell: Cell::Type(tcx.erase_and_anonymize_regions(part)),
                mutable: !recognition.is_std_type(part, StdItem::RefCell),
            }),
            ty::Adt(adt, adt_args) if adt.did().is_local() => {
                let all = impls.get(&adt.did()).into_iter().flatten();
                let of: Vec<DefId> = (all.clone())
                    .filter(|(t, _)| traits.contains(t))
                    .map(|&(_, imp)| imp)
                    .collect();
                // One a blanket impl of std's meets by another it has,
                // `ToString` by its `Display`: any of its impls but its
                // `Drop`, which runs where it drops. Not one std derives,
                // `Clone` or `Debug`, which only its own impl is.
                let items = tcx.lang_items();
                let derived = [
                    items.clone_trait(),
                    items.eq_trait(),
                    items.partial_ord_trait(),
                    Some(crate::lower::recognition::std_item(tcx, StdItem::Ord)),
                    Some(crate::lower::recognition::std_item(tcx, StdItem::Debug)),
                ];
                let of = match of.is_empty() && !derived.contains(&Some(trait_id)) {
                    true => all
                        .filter(|(t, _)| Some(*t) != items.drop_trait())
                        .map(|&(_, imp)| imp)
                        .collect(),
                    false => of,
                };
                for imp in of {
                    // A derive reads its fields by the trait it derives.
                    if crate::lower::recognition::known_derive(tcx, imp) {
                        for field in adt.all_fields() {
                            let field_ty = field.ty(tcx, adt_args).skip_normalization();
                            for &t in &traits {
                                bound_runs(recognition, impls, (field_ty, t), seen, given);
                            }
                        }
                        continue;
                    }
                    given.bodies.extend(
                        (tcx.associated_item_def_ids(imp).iter())
                            .filter(|&&item| matches!(tcx.def_kind(item), DefKind::AssocFn))
                            .map(|&item| (item, None)),
                    );
                }
            }
            // A library's, whose impls' bodies the crate can't see: what
            // it implements of `traits` may run anything, but what it's
            // equal only to itself by, `==` and `Hash`, its address's, as
            // its contract says (ADR 0864, amended). What it has no impl of,
            // its own or a blanket one outside std's, runs nothing.
            ty::Adt(adt, _) if recognition.foreign.in_library(adt.did()) => {
                let identity = recognition.foreign.equal_only_to_itself(adt.did());
                let by_address = |t: DefId| {
                    tcx.is_lang_item(t, rustc_hir::attrs::lang_items::LangItem::PartialEq)
                        || crate::lower::recognition::is_std_def(tcx, t, StdItem::Eq)
                        || crate::lower::recognition::is_std_def(tcx, t, StdItem::Hash)
                };
                let implemented = |t: DefId| {
                    let impls = tcx.trait_impls_of(t);
                    let own = rustc_middle::ty::fast_reject::SimplifiedType::Adt(adt.did());
                    impls.non_blanket_impls().contains_key(&own)
                        || (impls.blanket_impls().iter()).any(|&imp| !recognition.in_sysroot(imp))
                };
                if traits.iter().any(|&t| implemented(t) && !(identity && by_address(t))) {
                    given.any = true;
                }
            }
            ty::Adt(..)
            | ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::Array(..)
            | ty::Slice(_)
            | ty::Ref(..)
            | ty::Tuple(_)
            | ty::Foreign(_)
            | ty::FnDef(..) => {}
            // A type parameter's, or a projection's, by a trait no impl of
            // which is the program's, each std's: what std's code runs,
            // `H: Hasher`'s, which runs nothing of the crate's.
            ty::Param(_) | ty::Alias(..)
                if traits.iter().all(|&t| {
                    tcx.all_impls(t)
                        .all(|imp| !imp.is_local() && recognition.in_sysroot(imp))
                }) => {}
            // A type parameter's, a `dyn`'s, a function pointer's.
            _ => given.any = true,
        }
    }
}

/// The bounds std's impls of `traits` for `ty` are under, each a type and
/// the trait it's bound by, `T: Clone` of `Option<T>`'s `Clone`: what
/// they may run of what `ty` is made of (ADR 0868). `None` where `ty` is
/// the crate's, a cell, which its own impl borrows, or has a trait of
/// `traits` that isn't std's impl, a built-in one or one not selected.
fn std_impl_bounds<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    ty: Ty<'tcx>,
    traits: &[DefId],
) -> Option<Vec<(Ty<'tcx>, DefId)>> {
    let tcx = recognition.tcx;
    let ours = matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local());
    if ours || recognition.is_borrowed_cell(ty) || ty.has_param() || ty.has_aliases() {
        return None;
    }
    // A trait alias, `ptr::Thin`, which may stand for any, and has no impls.
    if traits.iter().any(|&t| tcx.def_kind(t) != DefKind::Trait) {
        return None;
    }
    let ty = tcx.erase_and_anonymize_regions(ty);
    // A function pointer's impls, a copy, a comparison of where it points,
    // never call it; only an `Fn` trait's does.
    if ty.is_fn_ptr() {
        let calls = traits.iter().any(|&t| tcx.fn_trait_kind_from_def_id(t).is_some());
        return (!calls).then(Vec::new);
    }
    let own_methods =
        |t: DefId| (tcx.associated_item_def_ids(t).iter()).any(|&item| matches!(tcx.def_kind(item), DefKind::AssocFn));
    let mut bounds = Vec::new();
    for &t in traits.iter().filter(|&&t| own_methods(t)) {
        // A trait of arguments, `Fn(&T)`'s, `PartialEq<Rhs>`'s, which a
        // bound here doesn't say.
        if tcx.generics_of(t).count() != 1 {
            return None;
        }
        let tr = ty::TraitRef::new(tcx, t, [ty]);
        let (imp, args) = crate::lower::traits::evidence_impl(tcx, recognition.typing_env, tr)?;
        if !recognition.in_sysroot(imp) {
            return None;
        }
        for (clause, _) in tcx.clauses_of(imp).instantiate(tcx, args) {
            if let Some(bound) = clause.skip_normalization().as_trait_clause() {
                let bound = bound.skip_binder();
                bounds.push((bound.self_ty(), bound.def_id()));
            }
        }
    }
    Some(bounds)
}

/// What a body borrows and runs, by its own expressions: each call's, each
/// closure it makes, which whatever it's given to may run, and each value
/// it may drop, whose destructor runs.
fn direct<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (impls, pointers): (&Impls, &super::fn_pointers::FnPointers<'tcx>),
    thir: &Thir<'tcx>,
    (drops, clones_itself): (&dyn Fn(Ty<'tcx>) -> bool, &dyn Fn(Ty<'tcx>) -> bool),
) -> Direct<'tcx> {
    let tcx = recognition.tcx;
    let mut found = Direct::default();
    let runs = |found: &mut Direct<'tcx>, called: Called<'tcx>| match called {
        Called::Nothing => {}
        Called::Cell(cell) => _ = found.own.cells.insert(cell),
        Called::Body(body, args) => found.runs.push((body, Some(args))),
        Called::Std(id, args) => {
            let given = std_given(recognition, impls, id, args);
            found.any |= given.any;
            found.own.cells.extend(given.cells);
            found.runs.extend(given.bodies);
        }
        Called::Any => found.any = true,
    };
    for e in thir.exprs.iter() {
        match e.kind {
            ExprKind::Call { fun, ref args, .. } => match fn_def(thir[fun].ty) {
                Some((id, generic_args)) => {
                    let called = called(
                        recognition,
                        id,
                        generic_args,
                        args.first().map(|&a| (thir, a)),
                        clones_itself,
                    );
                    runs(&mut found, called);
                }
                // A fn pointer's, which runs what it may be (ADR 0907): a
                // closure of the crate's, or what a call of a function does.
                None => match pointers.may_be(tcx, thir[fun].ty) {
                    Some(may_be) => {
                        for &(f, args) in may_be {
                            let called = match tcx.is_closure_like(f) {
                                true => Called::Body(f, args),
                                false => called(recognition, f, args, None, &|_| false),
                            };
                            runs(&mut found, called);
                        }
                    }
                    None => found.any = true,
                },
            },
            // Of the body's own parameters, as it is.
            ExprKind::Closure(ref closure) => {
                let id = closure.closure_id.to_def_id();
                found.runs.push((id, Some(ty::GenericArgs::identity_for_item(tcx, id))));
            }
            ExprKind::InlineAsm(_) | ExprKind::Yield { .. } | ExprKind::Become { .. } => found.any = true,
            _ => {}
        }
        // A value whose drop runs a destructor, which may run anything; of a
        // type parameter's, its caller's to say. A guard's releases its
        // borrow.
        let ty = tcx.erase_and_anonymize_regions(e.ty);
        // Nor what's behind a reference, `*state`, which no body owns.
        let behind_ref = matches!(e.kind, ExprKind::Deref { arg } if thir[arg].ty.is_ref());
        if behind_ref || recognition.is_guard(ty) || !ty.needs_drop(tcx, recognition.typing_env) {
            continue;
        }
        match ty.has_param() {
            true => _ = found.own.drops.insert(ty),
            false if drops(ty) => found.any = true,
            false => {}
        }
    }
    found
}

/// May `a` and `b`, two cells' types, be one cell? As ADR 0852's `may_be`
/// says of two types, a type parameter's any; and a slice's an array's of
/// its items, and a `dyn`'s any type where a value may be made one in
/// place, as one made the other, an `Rc<RefCell<Circle>>` an
/// `Rc<RefCell<dyn Shape>>`, shares its cell (ADR 0397): the cell's own
/// value, and a struct's last field's, `Framed<dyn Shape>`'s. Elsewhere,
/// in a `Vec`'s items or what an `Rc` points at, a `dyn` is only a `dyn`
/// of its trait: a `RefCell<Vec<Rc<dyn Fn()>>>` is no `RefCell<Vec<Rc<Node>>>`
/// (ADR 0960).
pub(in crate::lower) fn may_be_cell<'tcx>(tcx: TyCtxt<'tcx>, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
    may_be_cell_at(tcx, (a, b), true)
}

/// `may_be_cell` of `a` and `b`, where `unsized_here`, a `dyn` there may
/// be any type made one in place.
fn may_be_cell_at<'tcx>(tcx: TyCtxt<'tcx>, (a, b): (Ty<'tcx>, Ty<'tcx>), unsized_here: bool) -> bool {
    match (a.kind(), b.kind()) {
        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => true,
        (ty::Slice(x) | ty::Array(x, _), ty::Slice(y) | ty::Array(y, _)) => may_be_cell_at(tcx, (*x, *y), false),
        _ => match (
            crate::lower::analysis::head(tcx, a),
            crate::lower::analysis::head(tcx, b),
        ) {
            (Some((x, of_x)), Some((y, of_y))) => {
                let tail = unsized_in_place(tcx, a);
                x == y
                    && of_x.len() == of_y.len()
                    && (of_x.iter().zip(&of_y).enumerate())
                        .all(|(i, (&x, &y))| may_be_cell_at(tcx, (x, y), tail == Some(i)))
            }
            _ => true,
        },
    }
}

/// Which of `ty`'s type arguments, a struct's, counted among its types as
/// `head` lists them, a value of it may be made unsized by in place, as
/// Rust's `Unsize` of a struct is: the one its last field is, or is a
/// struct's tail of, `RefCell<T>`'s `T` through its `UnsafeCell<T>`. Not
/// a `Vec`'s or an `Rc`'s items, behind a pointer, which no value of it is
/// made another in place (ADR 0960).
pub(in crate::lower) fn unsized_in_place<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<usize> {
    let ty::Adt(adt, args) = *ty.kind() else {
        return None;
    };
    let at = unsized_tail(tcx, adt)?;
    Some(args.iter().take(at).filter(|arg| arg.as_type().is_some()).count())
}

/// The generic argument of `adt` its last field is the tail of, by its
/// index among all of them.
fn unsized_tail<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>) -> Option<usize> {
    if !adt.is_struct() {
        return None;
    }
    let last = adt.non_enum_variant().fields.iter().last()?;
    let mut ty = tcx.type_of(last.did).instantiate_identity().skip_normalization();
    loop {
        match *ty.kind() {
            ty::Param(param) => return Some(param.index as usize),
            ty::Adt(inner, args) => ty = args.get(unsized_tail(tcx, inner)?)?.as_type()?,
            _ => return None,
        }
    }
}

/// The crate's thread-locals no code but the crate's reaches: not visible
/// outside the crate, and each use of one in its bodies a read or a write
/// of what it holds, `get`, `set`, `take`, `replace`, `with_borrow` or
/// `with_borrow_mut`; never its cell lent, `with`'s, nor the key handed on.
fn closed_keys<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> HashSet<DefId> {
    use crate::lower::recognition::local_key_access;
    let mut keys = HashSet::new();
    let mut open = HashSet::new();
    for body in bodies {
        let thir = &body.thir;
        let mut accessed = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && local_key_access(tcx, id).is_some()
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
            {
                accessed.insert(strip(thir, arg));
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && def_id.is_local()
                && is_thread_local_key(tcx, def_id)
            {
                keys.insert(def_id);
                if !accessed.contains(&id) {
                    open.insert(def_id);
                }
            }
        }
    }
    keys.retain(|&key| !open.contains(&key) && !tcx.visibility(key).is_public());
    keys
}

/// Whether `def_id` is a thread-local's key, a `LocalKey`.
fn is_thread_local_key(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.def_kind(def_id), DefKind::Const { .. })
        && matches!(tcx.type_of(def_id).instantiate_identity().skip_normalization().kind(),
            ty::Adt(adt, _) if crate::lower::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey))
}

/// Whether a borrow asks of one `held`: of a cell that may be its, one of
/// them mutable. `closed`, the keys only their own accesses reach.
fn asks<'tcx>(tcx: TyCtxt<'tcx>, closed: &HashSet<DefId>, theirs: Borrow<'tcx>, held: Borrow<'tcx>) -> bool {
    (theirs.mutable || held.mutable) && overlap(tcx, closed, theirs.cell, held.cell)
}

/// Whether two cells may be one: a thread-local's and another's of its key,
/// or of its cell's type, where a reference to its cell may be had: not of
/// one of `closed`, its cell no `with` lends, which a borrow through a
/// reference or a field, React's `fiber.return.borrow()` while
/// `HYDRATION_PARENT_FIBER.set(..)` runs, never is (ADR 0960, amended);
/// two types alike.
fn overlap<'tcx>(tcx: TyCtxt<'tcx>, closed: &HashSet<DefId>, a: Cell<'tcx>, b: Cell<'tcx>) -> bool {
    let held = |key: DefId| match tcx.type_of(key).instantiate_identity().skip_normalization().kind() {
        ty::Adt(_, args) => args.types().next().map(|cell| tcx.erase_and_anonymize_regions(cell)),
        _ => None,
    };
    let alike = |x: Ty<'tcx>, y: Ty<'tcx>| x == y || may_be_cell(tcx, x, y);
    match (a, b) {
        (Cell::Key(x), Cell::Key(y)) => x == y,
        (Cell::Key(key), Cell::Type(ty)) | (Cell::Type(ty), Cell::Key(key)) => {
            !closed.contains(&key) && held(key).is_none_or(|cell| alike(cell, ty))
        }
        (Cell::Type(x), Cell::Type(y)) => alike(x, y),
    }
}

//! Destructors (ADR 0098): the scopes, flags and temporaries that drop a
//! value where rustc does, and the JS that drops it. What dropping a type
//! runs is `types`'s, and what a body owns and moves, found before it's
//! lowered, `facts`'.
//!
//! Where a body drops what it owns is MIR's drop elaboration's, which
//! `mir/drops.rs` follows (ADR 0364); this is what each drop runs, the drop
//! functions it's written in, and the drops a std function lowered in place
//! makes of its own, `map_or`'s fallback a `try` whose `finally` drops it.
//! What this can't do yet is an error, found before any JS is written: a
//! value with a destructor that nothing drops would be a wrong answer.

use crate::collections::{IndexMap, IndexSet};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use rustc_hir::def::DefKind;

use rustc_middle::thir::{ExprKind, Pat, StmtKind as ThirStmt};
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::recognition::{ChannelEnd, StdItem};
use super::representation::variant_field;
use super::traits::item_drop_key;
use super::{FnCx, R, lower_first};
use crate::js::{self, Expr, Stmt, StmtKind};

mod facts;
mod functions;
mod glue;
mod inlined;
mod parts;
pub(in crate::lower) use parts::{
    aborting_finally, catches_unwind, cleanup, dropped_after, dropped_unwinding, each_dropped, finally_holds,
    is_aborting, is_cleanup, live_guarded, may_throw, poisoned_and_released, unseen,
};
mod ref_cells;
mod scopes;
mod types;
pub(super) use types::own_traits_dropping_nothing;
mod uses;

pub(super) use self::uses::*;

use facts::{Facts, find_facts};
pub(super) use inlined::inlined_drops;
pub(super) use ref_cells::{RefCells, holds_itself};
pub(super) use types::Drops;
pub(super) use types::no_type_here;
use types::{DropQuery, TypeDrops, describe};

/// Whether dropping `ty`, a type the crate knows, runs a destructor, of the
/// `Rc`s `counted` and no dictionaries (ADR 0320): an `Rc` of one is
/// counted, as its last drops it.
pub(super) fn drop_runs<'tcx>(
    recognition: super::recognition::Recognition<'_, 'tcx>,
    (library, closure_facts): (bool, &super::analysis::ClosureFacts<'tcx>),
    counted: &super::analysis::Counted<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    let state = TypeDrops::default();
    // Its type parameters, of no drop given here, drop nothing: their drops
    // are their callers', passed on as they are.
    without_dictionaries(recognition, (library, closure_facts), counted, &state).drops(ty) == Drops::Runs
}

/// `drop_runs` of facts that no longer change, the crate's `Rc`s counted
/// and its closures' facts settled: each type's answer, in each typing
/// environment, found once. The analysis after they settle asks it of the
/// same types in each of its bodies, which a walk of each type's parts
/// again made most of a crate's analysis (#884).
pub(super) struct DropRuns<'a, 'tcx> {
    recognition: super::recognition::Recognition<'a, 'tcx>,
    facts: (bool, &'a super::analysis::ClosureFacts<'tcx>),
    counted: &'a super::analysis::Counted<'tcx>,
    known: RefCell<IndexMap<(ty::TypingEnv<'tcx>, Ty<'tcx>), bool>>,
}

impl<'a, 'tcx> DropRuns<'a, 'tcx> {
    pub(super) fn new(
        recognition: super::recognition::Recognition<'a, 'tcx>,
        facts: (bool, &'a super::analysis::ClosureFacts<'tcx>),
        counted: &'a super::analysis::Counted<'tcx>,
    ) -> Self {
        DropRuns {
            recognition,
            facts,
            counted,
            known: RefCell::default(),
        }
    }

    /// `drop_runs` of `ty`, asked in `typing_env`.
    pub(super) fn runs(&self, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
        if let Some(&known) = self.known.borrow().get(&(typing_env, ty)) {
            return known;
        }
        let recognition = super::recognition::Recognition {
            typing_env,
            ..self.recognition
        };
        let runs = drop_runs(recognition, self.facts, self.counted, ty);
        self.known.borrow_mut().insert((typing_env, ty), runs);
        runs
    }
}

/// Whether dropping `ty` in `owner` runs anything, where it's given the
/// drops `taken` of its slots (`traits::drop_slot`) and no dictionaries:
/// the lowering's `FnCx::drops` of it there, `!= Drops::Nothing`, asked
/// before any body is lowered, by what a collect's facts are asked of
/// (#737). The one answer of both: its type's own drop, `Held<J>`'s, runs
/// whatever `J`'s caller gives.
pub(super) fn drops_given<'tcx>(
    recognition: super::recognition::Recognition<'_, 'tcx>,
    (library, closure_facts): (bool, &super::analysis::ClosureFacts<'tcx>),
    counted: &super::analysis::Counted<'tcx>,
    (foreign, owner, taken): (&super::library::Foreign<'_, 'tcx>, DefId, &[u32]),
    ty: Ty<'tcx>,
) -> bool {
    let tcx = recognition.tcx;
    let mut state = TypeDrops::default();
    state.set_slots(super::traits::drop_items(tcx, foreign, owner));
    for &index in taken {
        state.give(super::traits::drop_slot(
            tcx,
            (foreign, &Default::default()),
            owner,
            index,
        ));
    }
    without_dictionaries(recognition, (library, closure_facts), counted, &state).drops(ty) != Drops::Nothing
}

/// What drops, asked in `recognition`'s typing environment with no
/// dictionaries given: a type parameter's drop is a maybe.
fn without_dictionaries<'a, 'tcx>(
    recognition: super::recognition::Recognition<'a, 'tcx>,
    (library, closure_facts): (bool, &'a super::analysis::ClosureFacts<'tcx>),
    counted: &'a super::analysis::Counted<'tcx>,
    state: &'a TypeDrops<'tcx>,
) -> DropQuery<'a, 'tcx> {
    DropQuery {
        recognition,
        evidence: super::traits::EvidenceQuery::without_evidence(recognition),
        library,
        counted,
        closure_facts,
        state,
        ref_cells: None,
        thir_args: None,
    }
}

/// The drop functions one drop makes, which come before its code: a call
/// to one may be in a branch, as an `Option`'s, that another isn't in.
#[derive(Default)]
struct Made<'tcx> {
    functions: Vec<(Ty<'tcx>, String)>,
    defs: Vec<Stmt>,
}

/// A copied default's drops: those it's given, by the type each drops in
/// its impl's terms, and each associated type a caller could give one for
/// (ADRs 0098, 0489).
pub(super) type DefaultDrops<'tcx> = (IndexMap<Ty<'tcx>, GivenDrop>, IndexSet<ty::AliasTy<'tcx>>);

/// A drop a body is given, keyed by the type it drops, its slot's type
/// (`traits::drop_slot`): a type parameter, an associated type of one, or
/// a type built of them. Its name, and its slot's index among its
/// function's own, which is what its callers pass it as and what its uses
/// are reported by; `None` of one given only for a while, a dictionary
/// entry's method's own or a copied default's (#887: one derivation of a
/// slot, its type's, where three numberings had met).
#[derive(Clone, Debug)]
pub(in crate::lower) struct GivenDrop {
    index: Option<u32>,
    name: String,
}

impl GivenDrop {
    /// One its function is given for its own slot `index`.
    pub(in crate::lower) fn own(index: u32, name: String) -> GivenDrop {
        GivenDrop {
            index: Some(index),
            name,
        }
    }

    /// One given for a while, not its function's own slot.
    pub(in crate::lower) fn lent(name: String) -> GivenDrop {
        GivenDrop { index: None, name }
    }
}

/// What a body inside another, a closure's or a copied default's, takes
/// from the drops of the one it's in while it's lowered, given back as it
/// ends: its statement and scopes, and a default's, the item's drops.
pub(super) struct EnclosingDrops<'tcx> {
    /// Whether a copied default's drops were put in place of the function's,
    /// which `outer` keeps.
    swapped: bool,
    /// The enclosing body's drop functions: a nested body, a closure's or
    /// a copied default's, is a JS function of its own, which declares its
    /// own.
    hoisted: Option<Hoisted<'tcx>>,
}

/// A function's drops while a copied default body has its own.
struct SwappedDrops<'tcx> {
    params: IndexMap<Ty<'tcx>, GivenDrop>,
    given: IndexSet<Ty<'tcx>>,
    slots: IndexSet<ty::AliasTy<'tcx>>,
    used: IndexSet<Ty<'tcx>>,
    cache: IndexMap<types::CacheKey<'tcx>, Drops<'tcx>>,
    sizes: IndexMap<Ty<'tcx>, usize>,
}

/// A drop `lend_drop_param` lent: of which type, what it hid, and whether
/// that type was given none before.
pub(super) struct LentDrop<'tcx> {
    ty: Ty<'tcx>,
    before: Option<GivenDrop>,
    newly_given: bool,
}

/// A function's drops, as its bodies are lowered.
#[derive(Default)]
pub(super) struct DropState<'tcx> {
    /// What a type drops depends on, beside its types (ADR 0098).
    types: TypeDrops<'tcx>,
    sizes: RefCell<IndexMap<Ty<'tcx>, usize>>,
    /// Each generic `Drop` impl's `drop`, as it's given its arguments, that
    /// never unwinds (`drop_never_unwinds`, #861).
    never_unwinding: RefCell<IndexMap<(DefId, GenericArgsRef<'tcx>), bool>>,
    /// Each body's facts, by the address of its THIR: a closure's is its own.
    facts: IndexMap<usize, Rc<Facts>>,
    /// The function being lowered's drop functions, by the type each drops.
    param_drops: IndexMap<Ty<'tcx>, GivenDrop>,
    /// The types whose given drops the body has used.
    used_drops: IndexSet<Ty<'tcx>>,
    /// Each closure made here that holds a value with a destructor: the body
    /// it's made in, and the variables it holds, which its drop drops.
    closures: IndexMap<DefId, (DefId, Vec<(Expr, Ty<'tcx>)>)>,
    /// The drop functions of the function or closure being lowered, each
    /// declared once at its top; `None` where each drop declares its own.
    hoisted: Option<Hoisted<'tcx>>,
    /// The drops of each function a copied default is lowered in, innermost
    /// last, in its impl's terms (ADR 0495).
    outer: Vec<SwappedDrops<'tcx>>,
    /// The drop functions made here whose body never throws, so a part
    /// dropped by one needs no `finally` for those after it (#416).
    quiet: IndexSet<String>,
}

/// A body's drop functions, each of a type inside itself or with a long
/// drop: its name, and its `const`.
#[derive(Default)]
pub(super) struct Hoisted<'tcx> {
    functions: Vec<(Ty<'tcx>, String)>,
    defs: Vec<Stmt>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What a type drops, asked of types and this function's facts alone.
    pub(super) fn drop_query(&self) -> DropQuery<'_, 'tcx> {
        DropQuery {
            recognition: self.recognition(),
            evidence: self.evidence_query(),
            library: self.krate.library,
            counted: self.krate.counted,
            closure_facts: self.krate.closure_facts,
            state: &self.drop_state.types,
            ref_cells: Some((self.ref_cells(), self.krate.cell_borrows)),
            thir_args: (self.copied_default_args())
                .map(|args| crate::lower::traits::default_body_args(self.tcx, args, self.frame.owner)),
        }
    }

    /// What dropping a `ty` runs. Of a copied default's type, the impl's,
    /// as its MIR is in the impl's terms (ADR 0176, #803).
    pub(in crate::lower) fn drops(&self, ty: Ty<'tcx>) -> Drops<'tcx> {
        self.drop_query().drops(ty)
    }

    /// Whether dropping a `ty` may panic: a `Drop` of the crate's that isn't
    /// one it never unwinds (ADR 0902), its own or a part's, a field's of
    /// the crate's types, a type argument's of std's, a tuple's, an array's
    /// or a closure's capture's; of a type parameter, a `dyn` or an alias,
    /// anyone's. The one answer, which what drops items it passes over asks.
    pub(in crate::lower) fn drop_may_panic(&self, ty: Ty<'tcx>) -> bool {
        self.drop_may_panic_in(ty, &mut IndexSet::default())
    }

    fn drop_may_panic_in(&self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty) || self.drops(ty) == Drops::Nothing {
            return false;
        }
        match ty.kind() {
            ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) => true,
            ty::Adt(adt, args) if self.krate.foreign.compiled_by_rust_js(adt.did()) => {
                let own = (self.tcx.adt_destructor(adt.did()))
                    .is_some_and(|destructor| !self.drop_never_unwinds(destructor.did, args));
                own || self.glue_part_may_panic(ty, seen)
            }
            ty::Adt(_, args) => args.types().any(|arg| self.drop_may_panic_in(arg, seen)),
            ty::Tuple(_) => self.glue_part_may_panic(ty, seen),
            ty::Array(item, _) | ty::Slice(item) => self.drop_may_panic_in(*item, seen),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).any(|upvar| self.drop_may_panic_in(upvar, seen))
            }
            _ => false,
        }
    }

    /// Whether dropping a part rustc's drop glue of `ty` drops may panic, any
    /// of a part rust-js can't read (#887).
    fn glue_part_may_panic(&self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        match self.drop_query().glue_parts(ty) {
            Some(parts) => parts.into_iter().any(|part| self.drop_may_panic_in(part, seen)),
            None => true,
        }
    }

    /// Whether the crate's `Drop` impl's `drop`, of a type given `args`,
    /// never unwinds: one of no type parameters `never_unwinding` holds, a
    /// generic one as it's given `args` (#861), the guide's `Defer` of a
    /// closure that can't panic. Each asked once a function.
    pub(in crate::lower) fn drop_never_unwinds(&self, drop: DefId, args: GenericArgsRef<'tcx>) -> bool {
        if self.krate.never_unwinding.contains(&drop) {
            return true;
        }
        if !self.tcx.generics_of(drop).requires_monomorphization(self.tcx) {
            return false;
        }
        let args = self.tcx.erase_and_anonymize_regions(args);
        if let Some(&known) = self.drop_state.never_unwinding.borrow().get(&(drop, args)) {
            return known;
        }
        // Asked again as it's judged, of a drop that drops its own type: as
        // one that may.
        self.drop_state.never_unwinding.borrow_mut().insert((drop, args), false);
        let never = super::analysis::never_unwinds_given(
            (&self.recognition(), self.krate.never_unwinding, self.krate.bodies),
            (drop, args, self.typing_env),
            (&|ty| self.drops(ty) != Drops::Nothing, &|ty| self.drop_may_panic(ty)),
        );
        self.drop_state.never_unwinding.borrow_mut().insert((drop, args), never);
        never
    }

    pub(in crate::lower) fn has_drops(&self, ty: Ty<'tcx>) -> bool {
        self.drop_query().has_drops(ty)
    }

    fn drops_itself(&self, ty: Ty<'tcx>) -> bool {
        self.drop_query().drops_itself(ty)
    }

    /// A call of `callee` of `args`: the type parameters it takes no
    /// destructor of, checked, a library's now, from its manifest, the
    /// crate's once each is lowered (ADR 0190).
    pub(in crate::lower) fn check_drops_given(&self, callee: DefId, args: GenericArgsRef<'tcx>, span: Span) -> R<()> {
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        let given: Vec<(u32, Option<Vec<u32>>)> = args
            .iter()
            .enumerate()
            .filter_map(|(index, arg)| {
                let ty = arg.as_type()?;
                let params = owned_params(self.tcx, ty);
                let own = self.drop_query().drops_but_params(ty) != Drops::Nothing;
                (own || !params.is_empty()).then(|| (index as u32, (!own).then_some(params)))
            })
            .collect();
        if given.is_empty() {
            return Ok(());
        }
        if let Some(item) = self.krate.foreign.item(callee) {
            for (index, params) in (given.iter()).filter(|(index, _)| {
                item.calling
                    .param(*index)
                    .is_some_and(|param| param.has(crate::library::ParamFact::NoDrop))
            }) {
                let Some(params) = params else {
                    let (path, name) = (self.tcx.def_path_str(callee), param_name(self.tcx, callee, *index));
                    return Err(self.unsupported(
                        span,
                        &format!("giving `{path}`'s `{name}` a type with a destructor, where it takes none"),
                    ));
                };
                self.drops_observed
                    .borrow_mut()
                    .no_drops
                    .entry(owner)
                    .or_default()
                    .extend(params);
            }
            return Ok(());
        }
        if callee.is_local() {
            self.drops_observed.borrow_mut().checks.push(DropCheck {
                caller: owner,
                callee,
                span,
                given,
            });
        }
        Ok(())
    }

    /// A value of `ty` that drops only what its type parameters do, taken as
    /// one that drops nothing, which the function's callers then give none of
    /// (ADR 0190): a generic iterator chrono steps, loops over or folds.
    /// `false` of one that drops something of its own, or in a copied
    /// default, whose parameters are its trait's.
    pub(in crate::lower) fn require_no_drops(&self, ty: Ty<'tcx>) -> bool {
        if self.in_copied_default() || self.drop_query().drops_but_params(ty) != Drops::Nothing {
            return false;
        }
        let owner = self.tcx.typeck_root_def_id(self.frame.owner);
        let params = owned_params(self.tcx, ty);
        self.drops_observed
            .borrow_mut()
            .no_drops
            .entry(owner)
            .or_default()
            .extend(params);
        true
    }

    /// A generic iterator taken by value, `items.fold(..)`, which drops it as
    /// it's done, which JS doesn't: one its callers give none with a
    /// destructor (ADR 0190). Not one what it makes, `output`, holds, and is
    /// closed to drop, `take(n)` of a `repeat`, whose drop it is (ADR 0422).
    pub(in crate::lower) fn require_consumed_drops_none(&self, receiver: Ty<'tcx>, output: Ty<'tcx>) {
        if self.recognition().closed_on_drop(output).is_none() {
            self.require_no_drops(receiver);
        }
    }

    /// Is `drop`, a type's `Drop::drop`, one rust-js runs (ADR 0100)?
    pub(in crate::lower) fn runs_drop(&self, drop: DefId) -> bool {
        self.drop_query().runs_drop(drop)
    }

    /// A closure, at `closure`, made here: the places its drop drops.
    pub(super) fn closure_made(&mut self, closure: DefId, places: Vec<(Expr, Ty<'tcx>)>) {
        self.drop_state.closures.insert(closure, (self.frame.owner, places));
    }

    /// Drop what the closure `closure` took: the variables it took, where it
    /// was made. Only there, or in a closure made inside it, can JS see them.
    /// Each in turn, the rest still dropped after one whose drop panics, as
    /// a closure's drop glue drops its captures, a struct's fields
    /// (`each_dropped`, #846).
    fn drop_taken(&mut self, closure: DefId, span: Span, made: &mut Made<'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let made_here = self.drop_state.closures.get(&closure).cloned().filter(|(made_in, _)| {
            std::iter::successors(Some(self.frame.owner), |&d| self.tcx.opt_parent(d)).any(|d| d == *made_in)
        });
        let Some((_, places)) = made_here else {
            return Err(self.unsupported(span, "dropping a closure that holds a value with a destructor here"));
        };
        let mut parts = Vec::new();
        for (place, ty) in places {
            let mut dropped = Vec::new();
            self.drop_in(place, ty, span, made, &mut dropped)?;
            parts.push(dropped);
        }
        out.extend(parts::each_dropped(parts, self.js_span(span)));
        Ok(())
    }

    /// The drop a closure that carries its drop carries, `$dropping`'s:
    /// what it took dropped, where it's made (ADR 0484).
    pub(super) fn carried_drop(&mut self, closure: DefId, span: Span) -> R<Vec<Stmt>> {
        let mut made = Made::default();
        let mut code = Vec::new();
        self.drop_taken(closure, span, &mut made, &mut code)?;
        Ok(made.defs.into_iter().chain(code).collect())
    }

    /// What the closure `closure` holds, `closure_made`'s, where it's made.
    pub(super) fn closure_held(&self, closure: DefId) -> Vec<(Expr, Ty<'tcx>)> {
        (self.drop_state.closures.get(&closure)).map_or_else(Vec::new, |(_, held)| held.clone())
    }

    /// Drop `value`, a `ty`: its own `drop`, then each part's, in Rust's
    /// order. `value` is read more than once, so it must read the same.
    pub(super) fn drop_value(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<()> {
        let mut made = Made::default();
        let mut code = Vec::new();
        self.drop_in(value, ty, span, &mut made, &mut code)?;
        out.extend(made.defs);
        out.extend(code);
        Ok(())
    }

    /// How many drops writing a `ty`'s in place takes, found once for each
    /// type; of one inside itself, those outside where it's inside.
    fn drop_size(&self, ty: Ty<'tcx>, stack: &mut Vec<Ty<'tcx>>) -> usize {
        if !self.has_drops(ty) || stack.contains(&ty) {
            return 0;
        }
        if let Some(&known) = self.drop_state.sizes.borrow().get(&ty) {
            return known;
        }
        stack.push(ty);
        let sum = |cx: &Self, tys: &mut dyn Iterator<Item = Ty<'tcx>>, stack: &mut Vec<Ty<'tcx>>| {
            tys.map(|t| cx.drop_size(t, stack)).sum::<usize>()
        };
        let size = match ty.kind() {
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drop_size(args.type_at(0), stack)
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.drop_size(args.type_at(0), stack),
            ty::Array(item, _) | ty::Slice(item) => self.drop_size(*item, stack),
            // A `RefCell`'s value, and what a counted `Rc` points at, after its
            // count (ADR 0320); a counted `Weak` drops only a count, an
            // uncounted one nothing (ADR 0843).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::RefCell) => self.drop_size(args.type_at(0), stack),
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => self.drop_size(pointee, stack) + 1,
            ty::Adt(..) if self.counted_weak(ty).is_some() => 1,
            ty::Adt(..) if self.uncounted_weak(ty).is_some() => 0,
            // Its own `drop`, and the parts rustc's drop glue drops (#887).
            ty::Tuple(_) | ty::Adt(..) => {
                let own = matches!(ty.kind(), ty::Adt(adt, _)
                    if self.tcx.adt_destructor(adt.did()).is_some_and(|d| self.runs_drop(d.did)));
                match self.drop_query().glue_parts(ty) {
                    Some(parts) => usize::from(own) + sum(self, &mut parts.into_iter(), stack),
                    // Of a part rust-js can't read, as long as one can be.
                    None => usize::MAX / 2,
                }
            }
            _ => 1,
        };
        stack.pop();
        self.drop_state.sizes.borrow_mut().insert(ty, size);
        size
    }

    /// A variable's of `ty`, refused where what it holds drops as JS can't
    /// drop it (ADR 0321).
    pub(in crate::lower) fn check_held_drops(&self, ty: Ty<'tcx>, span: Span) -> R<()> {
        match self.drops(self.reveal(ty)) {
            Drops::Unsupported(t, what) => Err(self.unsupported(span, &describe(t, what))),
            Drops::Nothing | Drops::Runs => Ok(()),
        }
    }

    fn drop_in(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, made: &mut Made<'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let ty = self.reveal(ty);
        match self.drops(ty) {
            Drops::Nothing => return Ok(()),
            Drops::Unsupported(t, what) => return Err(self.unsupported(span, &describe(t, what))),
            Drops::Runs => {}
        }
        // A type inside itself, as a list in a `Box` of itself, would need a
        // function of its own to drop.
        // One whose drop is a function already, in this drop: that.
        if let Some((_, name)) = made.functions.iter().find(|(t, _)| *t == ty) {
            let js_span = self.js_span(span);
            let name = name.clone();
            out.push(StmtKind::Expr(self.drop_function_call(&name, value)).at(js_span));
            return Ok(());
        }
        // One declared at the body's top already.
        if let Some(name) = self
            .drop_state
            .hoisted
            .as_ref()
            .and_then(|hoisted| hoisted.functions.iter().find(|(t, _)| *t == ty))
            .map(|(_, name)| name.clone())
        {
            let js_span = self.js_span(span);
            out.push(StmtKind::Expr(self.drop_function_call(&name, value)).at(js_span));
            return Ok(());
        }
        // A type of the crate's own whose drop is long, or inside itself, as
        // a list is, gets a function of its own, which calls itself for the
        // ones inside: its drop is written once, not once for each path to it.
        // Which is inside itself is the drop query's to say, through
        // whatever drops it, a map's entries too.
        if let ty::Adt(adt, _) = ty.kind()
            && (self.krate.foreign.compiled_by_rust_js(adt.did()))
            && (self.drops_itself(ty) || self.drop_size(ty, &mut Vec::new()) > 8)
        {
            let type_name = self.tcx.item_name(adt.did()).to_string();
            let name = self.fresh(&format!("drop{type_name}"));
            let param = self.fresh(&lower_first(&type_name));
            let js_span = self.js_span(span);
            // Declared once, at the top of the body, where it hoists them.
            match self.drop_state.hoisted.as_mut() {
                Some(hoisted) => hoisted.functions.push((ty, name.clone())),
                None => made.functions.push((ty, name.clone())),
            }
            let mut body = Vec::new();
            self.drop_parts(Expr::var(&param), ty, span, made, &mut body)?;
            if !parts::may_throw(&body) {
                self.drop_state.quiet.insert(name.clone());
            }
            let def = StmtKind::Const(name.clone(), Expr::arrow(vec![param.into()], body)).at(js_span);
            match self.drop_state.hoisted.as_mut() {
                Some(hoisted) => hoisted.defs.push(def),
                None => made.defs.push(def),
            }
            out.push(StmtKind::Expr(self.drop_function_call(&name, value)).at(js_span));
            return Ok(());
        }
        self.drop_parts(value, ty, span, made, out)
    }

    /// A call of `name`, a drop function made here, of `value`: known never
    /// to throw where its body never does (`quiet`).
    fn drop_function_call(&self, name: &str, value: Expr) -> Expr {
        let mut call = Expr::call(Expr::var(name), vec![value]);
        if self.drop_state.quiet.contains(name) {
            call.facts = call.facts.with(js::Known::NeverThrows);
        }
        call
    }
}

/// A call of one of the crate's functions, given what each of its type
/// parameters is given: `None` of a value with a destructor of its own,
/// else the caller's type parameters it holds (ADR 0190).
pub(super) struct DropCheck {
    caller: DefId,
    callee: DefId,
    span: Span,
    given: Vec<(u32, Option<Vec<u32>>)>,
}

/// Each call of the crate's functions given a value with a destructor where
/// the function takes none: an error at the call. One given a caller's type
/// parameter makes the caller take none of that either (ADR 0190). Whether
/// any was an error.
pub(super) fn check_no_drops(
    tcx: TyCtxt<'_>,
    no_drops: &mut IndexMap<DefId, BTreeSet<u32>>,
    checks: &[DropCheck],
) -> bool {
    let mut reported = IndexSet::default();
    let mut changed = true;
    while changed {
        changed = false;
        for check in checks {
            let required = no_drops.get(&check.callee).cloned().unwrap_or_default();
            for (index, params) in check.given.iter().filter(|(index, _)| required.contains(index)) {
                match params {
                    None if reported.insert(check.span) => {
                        let (path, name) = (tcx.def_path_str(check.callee), param_name(tcx, check.callee, *index));
                        let what = format!("giving `{path}`'s `{name}` a type with a destructor, where it takes none");
                        super::refusal::refuse(tcx, check.span, super::refusal::Refused::Feature(&what));
                    }
                    None => {}
                    Some(params) => {
                        let caller = no_drops.entry(check.caller).or_default();
                        for &param in params {
                            changed |= caller.insert(param);
                        }
                    }
                }
            }
        }
    }
    !reported.is_empty()
}

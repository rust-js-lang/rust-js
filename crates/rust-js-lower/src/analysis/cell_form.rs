//! A `RefCell` borrow's form, decided once from MIR, counted unless a
//! positive proof shows a quieter one holds (#2490, Campaign 4). Phase 1 is
//! a shadow: the compile takes today's forms, its deciders' (ADR 0328,
//! 0362, 0863), and where the session records the `cell-form` shadow
//! (`--shadow cell-form=<file>`), each borrow's proof is written beside
//! the form it took, no output changed, so the forms a flip would change
//! are measured before any is (design point 4).
//!
//! The proof, of each borrow's guard: from the call that makes it to its
//! drop, following each move of it to a local, nothing runs that may ask
//! of its cell. Its range's calls are each read twice: alone, where only
//! std's code of what it guards passes, and with the crate's bodies'
//! borrow summaries (`CellBorrows::call_asks`, design point 3), where a
//! call none of whose code may borrow a cell this one may be passes too.
//! The first that fails is what defeated it, `--shadow`'s bucket.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, BasicBlock, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{self, ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;

use super::cell_borrows::{Borrow, Cell, CellBorrows, resolve_runs_nothing};
use crate::recognition::{Recognition, Std, StdItem};
use crate::session::ShadowKind;

mod wholes;
pub(crate) use wholes::{Whole, whole_uses};

/// The form the compile took of a borrow, today's deciders' answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Today {
    /// The cell itself, unchecked: a quiet field's, class's or thread-local
    /// module variable's (`QuietCells::calls`), or of a type no borrow of
    /// which is counted (`quiet_ref_cell`).
    Unchecked,
    /// Checked where it's made, not counted (`QuietCells::momentary`).
    Momentary,
    /// `$borrow(c)`, counted and released as its guard drops.
    Counted,
    /// Of a form its declaration gives, a class's flattened state (ADR
    /// 1044), never counted: a proof that fails it is a refusal, never a
    /// quiet count (design point 1). A field that holds its value (ADR
    /// 0362) is a handle where it's quiet, `$fieldCell` where it's not
    /// (`representation::held`), as its borrows' forms say.
    Declared,
}

impl Today {
    fn text(self) -> &'static str {
        match self {
            Today::Unchecked => "unchecked",
            Today::Momentary => "momentary",
            Today::Counted => "counted",
            Today::Declared => "declared",
        }
    }
}

/// What, in a guard's range, the proof can't pass: its bucket.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Defeat {
    /// A function of the crate's, run while it's held.
    CrateFn,
    /// A closure of the crate's, run while it's held.
    Closure,
    /// std's code given the crate's, a bound's impl or a closure, `map`'s.
    StdRunsCrate,
    /// A type parameter's or a `dyn`'s method, or a property read of one:
    /// what may be any code.
    Generic,
    /// A binding's, a library's: code rust-js can't see, which may call
    /// back.
    Unseen,
    /// A call of a fn pointer.
    FnPointer,
    /// A value's destructor, dropped while it's held.
    Destructor,
    /// Another borrow, of a cell this one may be.
    SecondBorrow,
    /// The guard given on, by value or as `&mut` to what isn't its own
    /// `Deref`, returned, or stored in a place.
    Escapes,
    /// A `try_borrow`, which asks whether it's borrowed: counted by design.
    TryBorrow,
    /// An `.await` or a `yield`, past which anything may run.
    Suspends,
    /// Its field used whole somewhere in the crate, lent to what isn't its
    /// own borrow or write, moved or copied (`whole_uses`): every borrow of
    /// it counted (design point 6).
    WholeUse,
}

impl Defeat {
    fn text(self) -> &'static str {
        match self {
            Defeat::CrateFn => "crate-fn",
            Defeat::Closure => "closure",
            Defeat::StdRunsCrate => "std-runs-crate",
            Defeat::Generic => "generic",
            Defeat::Unseen => "unseen",
            Defeat::FnPointer => "fn-pointer",
            Defeat::Destructor => "destructor",
            Defeat::SecondBorrow => "second-borrow",
            Defeat::Escapes => "escapes",
            Defeat::TryBorrow => "try-borrow",
            Defeat::Suspends => "suspends",
            Defeat::WholeUse => "whole-use",
        }
    }
}

/// What the proof needs of the crate: who may borrow what, and what drops.
pub(crate) struct Proving<'r, 'a, 'tcx> {
    pub(crate) recognition: &'r Recognition<'a, 'tcx>,
    pub(crate) borrows: &'r CellBorrows<'tcx>,
    pub(crate) drops: &'r dyn Fn(Ty<'tcx>) -> bool,
}

/// A `RefCell` borrow's proof, of its guard's range (`guard_range`): its
/// call's block, its cell's type, what it's borrowed of and its field where
/// it's one, the first defeat alone and with the summaries, and the
/// function whose call is the second, where it's a call's.
struct Ranged<'tcx> {
    block: BasicBlock,
    /// Whether it's a `try_borrow`, which asks whether its cell is
    /// borrowed: counted by design, but where nothing can have it borrowed
    /// (`TypeProofs::uncontested`).
    tried: bool,
    cell: Ty<'tcx>,
    of: &'static str,
    field: Option<&'tcx ty::FieldDef>,
    alone: Option<Defeat>,
    summed: Option<Defeat>,
    by: Option<DefId>,
    exceptions: IndexSet<u32>,
}

/// Of `body`, its MIR, and `shapes`, its THIR's, each `RefCell` borrow's
/// proof of its guard's range, a `try_borrow`'s too.
fn ranges_of<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    body: &mir::Body<'tcx>,
    shapes: Option<&Shapes<'_, 'tcx>>,
) -> Vec<Ranged<'tcx>> {
    let tcx = proving.recognition.tcx;
    let owner = body.source.def_id();
    let mut ranges = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        if data.is_cleanup {
            continue;
        }
        let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            continue;
        };
        let (mutable, tried) = match proving.recognition.classify(def_id, generic_args) {
            Some(Std::Borrow { mutable, lock: false }) => (mutable, false),
            Some(Std::TryBorrow { mutable, lock: false }) => (mutable, true),
            _ => continue,
        };
        let Some(cell) = args.first().map(|arg| arg.node.ty(body, tcx).peel_refs()) else {
            continue;
        };
        if !proving.recognition.is_std_type(cell, StdItem::RefCell) {
            continue;
        }
        let held = Borrow {
            cell: Cell::Type(tcx.erase_and_anonymize_regions(cell)),
            mutable,
            class: proving.borrows.flow().call_class(owner, block),
        };
        let (of, field) = receiver_of(tcx, body, &args[0].node);
        let (alone, summed, by, exceptions) = guard_range(
            proving,
            (body, shapes),
            (block, destination.local),
            (held, field.map(|field| field.did)),
        );
        ranges.push(Ranged {
            block,
            tried,
            cell,
            of,
            field,
            alone,
            summed,
            by,
            exceptions,
        });
    }
    ranges
}

/// How `TypeProofs::of_crate` proves a body, its owner's: each its own
/// typing's `Proving`, given to what's run of it.
pub(crate) type Prove<'p, 'tcx> = dyn Fn(DefId, &mut dyn FnMut(&Proving<'_, '_, 'tcx>)) + 'p;

/// What the proof finds of the crate's `RefCell` types, of every borrow's
/// range in its bodies: the fields used whole none of whose type's borrows'
/// ranges it fails with the summaries, so no whole use, itself a moment's
/// borrow, can meet one held (#2490's whole-use proof B); and the
/// `try_borrow`s, by their body and block, of such a type, which nothing
/// can find borrowed, `Ok` as Rust's is (ADR 0863's "a `RefCell` no borrow
/// of which can meet another").
#[derive(Default)]
pub(crate) struct TypeProofs {
    pub(crate) quiet_typed: IndexSet<DefId>,
    pub(crate) uncontested: IndexSet<(DefId, BasicBlock)>,
}

impl TypeProofs {
    /// Of the crate's bodies, each its MIR, its THIR and the proving of its
    /// own typing, and its fields used whole, `wholes`.
    pub(crate) fn of_crate<'b, 'tcx: 'b>(
        tcx: ty::TyCtxt<'tcx>,
        bodies: impl Iterator<Item = (&'b mir::Body<'tcx>, &'b Thir<'tcx>)>,
        prove: &Prove<'_, 'tcx>,
        wholes: &IndexMap<DefId, Whole>,
    ) -> Self {
        let mut loud: IndexSet<Ty<'tcx>> = IndexSet::default();
        let mut tries: Vec<(DefId, BasicBlock, Ty<'tcx>)> = Vec::new();
        for (body, thir) in bodies {
            let owner = body.source.def_id();
            let shapes = (!thir.exprs.is_empty()).then(|| Shapes::of(thir));
            prove(owner, &mut |proving| {
                loud.extend(held_while_running(proving, body));
                for ranged in ranges_of(proving, body, shapes.as_ref()) {
                    let cell = tcx.erase_and_anonymize_regions(ranged.cell);
                    if ranged.summed.is_some() {
                        loud.insert(cell);
                    }
                    if ranged.tried {
                        tries.push((owner, ranged.block, cell));
                    }
                }
            });
        }
        // A loud type of a type parameter's, a generic body's, is loud of
        // each instance.
        let is_loud = |cell: Ty<'tcx>| loud.iter().any(|&loud| may_be_instance(loud, cell));
        // Of a field of a type of no parameters: a generic one's cell is of
        // each instance's type.
        let quiet_typed = (wholes.keys().copied())
            .filter(|&field| {
                let ty = tcx.type_of(field).instantiate_identity().skip_normalization();
                !crate::generic_facts::may_be_any_type(ty) && !is_loud(tcx.erase_and_anonymize_regions(ty))
            })
            .collect();
        let uncontested = (tries.into_iter())
            .filter(|&(.., cell)| !is_loud(cell))
            .map(|(owner, block, _)| (owner, block))
            .collect();
        TypeProofs {
            quiet_typed,
            uncontested,
        }
    }
}

/// Whether `general`, a type that may name type parameters, may be
/// `concrete`: each parameter any type, each other part the same.
fn may_be_instance<'tcx>(general: Ty<'tcx>, concrete: Ty<'tcx>) -> bool {
    if general == concrete {
        return true;
    }
    if !crate::generic_facts::may_be_any_type(general) {
        return false;
    }
    let parts = |ty: Ty<'tcx>| -> Option<Vec<ty::GenericArg<'tcx>>> {
        match ty.kind() {
            ty::Adt(_, args) | ty::Closure(_, args) => Some(args.to_vec()),
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => Some(vec![(*inner).into()]),
            ty::Tuple(items) => Some(items.iter().map(Into::into).collect()),
            _ => None,
        }
    };
    let same_head = match (general.kind(), concrete.kind()) {
        (ty::Adt(a, _), ty::Adt(b, _)) => a.did() == b.did(),
        (ty::Closure(a, _), ty::Closure(b, _)) => a == b,
        (ty::Ref(_, _, a), ty::Ref(_, _, b)) => a == b,
        (ty::Slice(_), ty::Slice(_)) | (ty::Array(..), ty::Array(..)) => true,
        (ty::Tuple(a), ty::Tuple(b)) => a.len() == b.len(),
        (ty::Adt(..) | ty::Closure(..) | ty::Ref(..) | ty::Slice(_) | ty::Array(..) | ty::Tuple(_), _) => false,
        // A type parameter, any type; what it can't take apart, a raw
        // pointer, a function's: it may be.
        _ => return true,
    };
    let (Some(general), Some(concrete), true) = (parts(general), parts(concrete), same_head) else {
        return false;
    };
    general.len() == concrete.len()
        && general
            .iter()
            .zip(&concrete)
            .all(|(g, c)| match (g.as_type(), c.as_type()) {
                (Some(g), Some(c)) => may_be_instance(g, c),
                _ => true,
            })
}

/// Of `body`, its MIR, the `RefCell` types a call borrows while it runs
/// code, which the proof reads no range of: a thread-local's
/// `with_borrow(_mut)`'s closure, and a cell's `replace_with`'s function.
/// Each is loud (`TypeProofs`).
fn held_while_running<'tcx>(proving: &Proving<'_, '_, 'tcx>, body: &mir::Body<'tcx>) -> Vec<Ty<'tcx>> {
    let tcx = proving.recognition.tcx;
    let mut held = Vec::new();
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            continue;
        };
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            continue;
        };
        if !matches!(
            proving.recognition.classify(def_id, generic_args),
            Some(Std::LocalBorrow { .. } | Std::CellReplaceWith)
        ) {
            continue;
        }
        // The cell, or the thread-local's key of one.
        let Some(receiver) = args.first().map(|arg| arg.node.ty(body, tcx).peel_refs()) else {
            continue;
        };
        let cell = match receiver.kind() {
            ty::Adt(_, key_args) if proving.recognition.is_std_type(receiver, StdItem::LocalKey) => {
                key_args.types().next()
            }
            _ => Some(receiver),
        };
        held.extend(
            cell.filter(|&cell| proving.recognition.is_std_type(cell, StdItem::RefCell))
                .map(|cell| tcx.erase_and_anonymize_regions(cell)),
        );
    }
    held
}

/// Of `body`, its MIR, and `thir`, its THIR where it has one, each
/// `RefCell` borrow's proof beside `today`'s form of it, a `cell-form`
/// shadow line each: `site`, today's form, the proof's alone and with the
/// summaries, the borrow's shape in its source, what it's borrowed of and
/// its field where it's one, its cell's type, the first defeat of each, its
/// function, where, the function whose call defeated it with the
/// summaries, and the ADR 0262 differences its range is passed by (39, 44),
/// Tu's decisions kept as today's output keeps them, not proofs. A field used whole (`wholes`) defeats each borrow of it, but
/// with the summaries where every whole use is a lend to a parameter only
/// ever borrowed (proof A, `Whole::lent_to_borrows`) or no borrow of its
/// cell's type is loud (proof B, `TypeProofs::quiet_typed`). A `try_borrow`
/// is counted by design, but one nothing can find borrowed: of a cell no
/// borrow of whose type is loud (`TypeProofs::uncontested`), or a flattened
/// state's, each borrow of which is proved quiet or refused (#687's rule).
pub(crate) fn shadow_cell_forms_of<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    body: &mir::Body<'tcx>,
    (thir, wholes, proofs): (Option<&Thir<'tcx>>, &IndexMap<DefId, Whole>, &TypeProofs),
    today: &dyn Fn(BasicBlock, Ty<'tcx>, Option<&'tcx ty::FieldDef>) -> Today,
) {
    let tcx = proving.recognition.tcx;
    if !crate::session::shadowing(tcx, ShadowKind::CellForm) {
        return;
    }
    let owner = body.source.def_id();
    if !owner.is_local() {
        return;
    }
    let shapes = thir.filter(|thir| !thir.exprs.is_empty()).map(Shapes::of);
    for ranged in ranges_of(proving, body, shapes.as_ref()) {
        let Ranged {
            block,
            tried,
            cell,
            of,
            field,
            mut alone,
            mut summed,
            mut by,
            exceptions,
        } = ranged;
        // A field used whole, where its range proved nothing else: its
        // cell kept whole, with its flag, which each borrow checks, and is
        // never counted where the range is quiet, a momentary borrow
        // (design point 6); with the summaries, unchecked where proof A or
        // B passes it. A declared form has no flag: the proof fails it.
        let taken = today(block, cell, field);
        if tried && taken != Today::Declared && !proofs.uncontested.contains(&(owner, block)) {
            (alone, summed, by) = (Some(Defeat::TryBorrow), Some(Defeat::TryBorrow), None);
        }
        let (mut alone_checked, mut summed_checked) = (false, false);
        if let Some((field, whole)) = field.and_then(|field| Some((field.did, wholes.get(&field.did)?))) {
            let proved = whole.lent_to_borrows || proofs.quiet_typed.contains(&field);
            match taken == Today::Declared {
                true => {
                    alone.get_or_insert(Defeat::WholeUse);
                    if summed.is_none() && !proved {
                        summed = Some(Defeat::WholeUse);
                        by = None;
                    }
                }
                false => {
                    alone_checked = alone.is_none();
                    summed_checked = summed.is_none() && !proved;
                }
            }
        }
        let data = &body[block];
        let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
            continue;
        };
        let fun_span = fun_span(func, data.terminator());
        let classify = |id, args| proving.recognition.classify(id, args);
        let shape = (shapes.as_ref())
            .and_then(|shapes| shapes.shape_at(fun_span, &classify))
            .unwrap_or("unknown");
        let verdict = |defeat: Option<Defeat>, checked: bool| match (defeat, checked) {
            (Some(_), _) => "counted",
            (None, true) => "momentary",
            (None, false) => "quiet",
        };
        let word = |defeat: Option<Defeat>, checked: bool| match (defeat, checked) {
            (Some(defeat), _) => defeat.text(),
            (None, true) => Defeat::WholeUse.text(),
            (None, false) => "-",
        };
        let at = tcx
            .sess
            .source_map()
            .span_to_diagnostic_string(data.terminator().source_info.span);
        // Its count, what a flip would do of it: a quieter form today the
        // proof counts gains `$borrow`, a counted one it proves quiet loses
        // it, and an unchecked one it checks gains the check.
        let answer = match (taken, summed, summed_checked) {
            (Today::Counted, None, _) => "loses-count",
            (Today::Counted, Some(_), _) | (Today::Momentary, None, _) | (_, None, false) => "agrees",
            (_, None, true) => "gains-check",
            (_, Some(_), _) => "gains-count",
        };
        rust_js_program::decisions::record("cell-form", answer);
        let line = format!(
            "site\t{}\t{}\t{}\t{shape}\t{of}\t{}\t{}\t{}\t{}\t{}\t{at}\t{}\t{}",
            taken.text(),
            verdict(alone, alone_checked),
            verdict(summed, summed_checked),
            field.map_or_else(|| "-".to_string(), |field| tcx.def_path_str(field.did)),
            tcx.erase_and_anonymize_regions(cell),
            word(alone, alone_checked),
            word(summed, summed_checked),
            tcx.def_path_str(owner),
            by.map_or_else(|| "-".to_string(), |by| tcx.def_path_str(by)),
            match exceptions.is_empty() {
                true => "-".to_string(),
                false => exceptions.iter().map(u32::to_string).collect::<Vec<_>>().join(","),
            },
        );
        crate::session::shadow(tcx, ShadowKind::CellForm, || line);
    }
}

/// ADR 0262's difference 39: a guard that's only a call's receiver is held
/// while the call runs, not while its other arguments do.
const RECEIVER_ONLY: u32 = 39;
/// ADR 0262's difference 44: a guard read only for a binding's argument is
/// held while it's read, not while the binding runs.
const ARGUMENT_READ: u32 = 44;

/// Of the guard a borrow ending `block` makes in `local`, the first defeat
/// in its range, alone and with the summaries, the function whose call is
/// the second, where it's a call's, and the ADR 0262 differences its range
/// is passed by, Tu's decisions, not proofs: from the borrow's return
/// to each drop of it, a move of it to a local followed, an unwind's
/// cleanup left out (it runs nothing a program sees past the panic).
fn guard_range<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    (body, shapes): (&mir::Body<'tcx>, Option<&Shapes<'_, 'tcx>>),
    (made, local): (BasicBlock, Local),
    (held, held_field): (Borrow<'tcx>, Option<DefId>),
) -> (Option<Defeat>, Option<Defeat>, Option<DefId>, IndexSet<u32>) {
    let tcx = proving.recognition.tcx;
    let owner = body.source.def_id();
    let mut guards: IndexSet<Local> = IndexSet::default();
    guards.insert(local);
    // Each local made a reference to a guard, `&g` or `&mut g`, which its
    // own `Deref` takes.
    let mut lent: IndexSet<Local> = IndexSet::default();
    // Each local that's what it guards read through it: its `Deref`'s
    // reference, a reborrow or a copy of what that's of, or its clone or
    // `as_deref()`.
    let mut values: IndexSet<Local> = IndexSet::default();
    // The blocks whose calls are a call's other arguments, of which the
    // guard is only the receiver (`receiver_only`).
    let mut arguments: IndexSet<BasicBlock> = IndexSet::default();
    // The ADR 0262 differences a call in its range passed by alone, which
    // the proof keeps as today's output does, never a proof of its own.
    let mut exceptions: IndexSet<u32> = IndexSet::default();
    // What a call runs that the proof alone can't pass, and whether the
    // summaries pass it: of code rust-js sees only, where `seen_only`.
    let asked = |def_id, generic_args, receiver, seen_only: bool| {
        let defeat = (proving.borrows)
            .unproven_call(proving.recognition, (owner, def_id, generic_args), receiver, held)
            .filter(|&defeat| !seen_only || defeat != super::cell_borrows::Unproven::Unseen)?;
        let asks = match seen_only {
            true => {
                (proving.borrows).call_asks_seen(proving.recognition, (owner, def_id, generic_args), receiver, held)
            }
            false => (proving.borrows).call_asks(
                proving.recognition,
                (owner, def_id, generic_args),
                receiver,
                held,
                proving.drops,
            ),
        };
        Some((defeat_of(defeat), !asks))
    };
    let (mut alone, mut summed): (Option<Defeat>, Option<Defeat>) = (None, None);
    let mut by: Option<DefId> = None;
    let mut note = |defeat: Defeat, won_back: bool, callee: Option<DefId>| {
        alone.get_or_insert(defeat);
        if !won_back && summed.is_none() {
            summed = Some(defeat);
            by = callee;
        }
    };
    let Some(next) = (body[made].terminator().successors()).find(|&b| !body[b].is_cleanup) else {
        return (None, None, None, IndexSet::default());
    };
    let mut seen: IndexSet<BasicBlock> = IndexSet::default();
    let mut work = vec![next];
    let moves = |operand: &Operand<'tcx>, guards: &IndexSet<Local>| matches!(operand, Operand::Move(place) if guards.contains(&place.local));
    while let Some(block) = work.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body[block];
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            match rvalue {
                Rvalue::Use(operand, ..) if moves(operand, &guards) => match place.projection.is_empty() {
                    true => _ = guards.insert(place.local),
                    false => note(Defeat::Escapes, false, None),
                },
                Rvalue::Ref(_, _, lent_place) if guards.contains(&lent_place.local) && place.projection.is_empty() => {
                    lent.insert(place.local);
                }
                Rvalue::Aggregate(_, operands) if operands.iter().any(|op| moves(op, &guards)) => {
                    note(Defeat::Escapes, false, None)
                }
                Rvalue::Ref(_, _, of)
                | Rvalue::CopyForDeref(of)
                | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..)
                    if values.contains(&of.local)
                        && place.projection.is_empty()
                        && matches!(of.projection.first(), None | Some(mir::PlaceElem::Deref)) =>
                {
                    values.insert(place.local);
                }
                _ => {}
            }
        }
        let terminator = data.terminator();
        let mut ends = false;
        match &terminator.kind {
            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => ends = true,
            TerminatorKind::Drop { place, .. } => {
                let ty = place.ty(body, tcx).ty;
                // A type parameter's value's drop is what each caller's
                // instance runs, its callers' to say (#1321): each instance's,
                // where every one is seen.
                if crate::generic_facts::may_be_any_type(ty) && ty.needs_drop(tcx, proving.recognition.typing_env) {
                    let instances = proving.borrows.instances_of(tcx, owner, ty);
                    let runs = |ty: Ty<'tcx>| (proving.drops)(ty) && !proving.recognition.is_guard(ty);
                    let ran: Vec<Ty<'tcx>> = instances.iter().flatten().copied().filter(|&ty| runs(ty)).collect();
                    if instances.is_none() || !ran.is_empty() {
                        let won = instances.is_some()
                            && ran
                                .iter()
                                .all(|&ty| destructor_asks(proving, owner, ty, held).is_some_and(|asks| !asks));
                        note(Defeat::Destructor, won, None);
                    }
                } else if (proving.drops)(ty) && !proving.recognition.is_guard(ty) {
                    let won = destructor_asks(proving, owner, ty, held).is_some_and(|asks| !asks);
                    note(Defeat::Destructor, won, None);
                }
            }
            TerminatorKind::Return if guards.contains(&mir::RETURN_PLACE) => note(Defeat::Escapes, false, None),
            TerminatorKind::Yield { .. } => note(Defeat::Suspends, false, None),
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                ..
            } => {
                let Some((def_id, generic_args)) = func.const_fn_def() else {
                    let ptr = func.ty(body, tcx);
                    let won = !proving
                        .borrows
                        .pointer_call_asks(proving.recognition, ptr, held, proving.drops);
                    note(Defeat::FnPointer, won, None);
                    work.extend(terminator.successors().filter(|&b| !body[b].is_cleanup));
                    continue;
                };
                // Its THIR call, where the proof finds it, whose receiver
                // says what a binding's method runs.
                let call = shapes.and_then(|shapes| shapes.call_at(fun_span(func, terminator)));
                let receiver = call.and_then(|(thir, call_args)| Some((thir, *call_args.first()?)));
                let given_guard = args.iter().any(|arg| moves(&arg.node, &guards));
                let lends_guard = args
                    .iter()
                    .any(|arg| arg.node.place().is_some_and(|p| lent.contains(&p.local)));
                match proving.recognition.classify(def_id, generic_args) {
                    // `drop(g)`: its range ends.
                    Some(Std::Drop) if given_guard => ends = true,
                    // `Ref::map(g, f)`: the mapped guard is the guard, `f` run
                    // while it's held.
                    Some(Std::GuardMap { .. }) if given_guard => {
                        guards.insert(destination.local);
                        if let Some(f) = args.get(1) {
                            let f = f.node.ty(body, tcx);
                            if let ty::Closure(closure, closure_args) = *f.kind() {
                                let won = !proving.borrows.call_asks(
                                    proving.recognition,
                                    (owner, closure, closure_args),
                                    None,
                                    held,
                                    proving.drops,
                                );
                                note(Defeat::Closure, won, Some(closure));
                            }
                        }
                    }
                    Some(Std::GuardValue { .. }) if lends_guard => {
                        values.insert(destination.local);
                        if let Some(target) = target
                            && let Some(calls) = receiver_only(body, &guards, destination.local, *target)
                        {
                            arguments.extend(calls);
                        }
                    }
                    _ if given_guard => note(Defeat::Escapes, false, None),
                    _ if lends_guard => {
                        let won = !proving.borrows.call_asks(
                            proving.recognition,
                            (owner, def_id, generic_args),
                            None,
                            held,
                            proving.drops,
                        );
                        note(Defeat::Escapes, won, Some(def_id));
                    }
                    // A borrow of another field's cell, or its own call that
                    // runs none of the crate's code, `replace`: two fields'
                    // places are two cells, whichever objects they're of.
                    Some(
                        Std::Borrow { lock: false, .. }
                        | Std::TryBorrow { lock: false, .. }
                        | Std::CellReplace
                        | Std::CellGet
                        | Std::CellGetMut,
                    ) if let Some(held_field) = held_field
                        && let Some(arg) = args.first()
                        && receiver_of(tcx, body, &arg.node)
                            .1
                            .is_some_and(|theirs| theirs.did != held_field) => {}
                    Some(Std::Borrow { mutable, lock: false } | Std::TryBorrow { mutable, lock: false })
                        if let Some(theirs) = (args.first())
                            .map(|arg| arg.node.ty(body, tcx).peel_refs())
                            .filter(|&cell| proving.recognition.is_std_type(cell, StdItem::RefCell)) =>
                    {
                        let theirs = Borrow {
                            cell: Cell::Type(tcx.erase_and_anonymize_regions(theirs)),
                            mutable,
                            class: proving.borrows.flow().call_class(owner, block),
                        };
                        if proving.borrows.borrows_meet(tcx, theirs, held) {
                            note(Defeat::SecondBorrow, false, Some(def_id));
                        }
                    }
                    // A promise's resolve, given what runs nothing (rd654).
                    _ if call.is_some_and(|(thir, call_args)| {
                        resolve_runs_nothing(proving.recognition, thir, (def_id, generic_args), call_args)
                    }) => {}
                    // A binding given what the guard guards, read through it,
                    // at its statement's end: held while that's read, not while
                    // the binding runs, as JS's `prepare(response.nonce)` reads
                    // it first (ADR 0863, amended, rd427; ADR 0262's difference
                    // 44).
                    _ if crate::bindings::is_binding(tcx, def_id)
                        && args
                            .iter()
                            .any(|arg| arg.node.place().is_some_and(|p| values.contains(&p.local)))
                        && only_temporaries(body, &guards)
                        && target.is_some_and(|target| dropped_next(body, &guards, target)) =>
                    {
                        if asked(def_id, generic_args, receiver, false).is_some_and(|(_, won)| !won) {
                            exceptions.insert(ARGUMENT_READ);
                        }
                    }
                    // One of a call's other arguments, of which the guard is
                    // only the receiver, `target.borrow_mut().push(show(v))`:
                    // held while the call runs, not while they do, as JS's
                    // `target.push(show(v))` evaluates them first; of them,
                    // only what rust-js sees asks (ADR 0863, amended; ADR
                    // 0262's difference 39).
                    _ if arguments.contains(&block) => {
                        let seen = asked(def_id, generic_args, receiver, true);
                        if let Some((defeat, won)) = seen {
                            note(defeat, won, Some(def_id));
                        }
                        if seen.is_none_or(|(_, won)| won)
                            && asked(def_id, generic_args, receiver, false).is_some_and(|(_, won)| !won)
                        {
                            exceptions.insert(RECEIVER_ONLY);
                        }
                    }
                    _ => {
                        if let Some((defeat, won)) = asked(def_id, generic_args, receiver, false) {
                            note(defeat, won, Some(def_id));
                        }
                    }
                }
                // What it guards, cloned or `as_deref()`ed.
                if args
                    .first()
                    .and_then(|arg| arg.node.place())
                    .is_some_and(|p| values.contains(&p.local))
                    && (tcx.is_lang_item(def_id, rustc_hir::attrs::lang_items::LangItem::CloneFn)
                        || crate::recognition::is_as_deref(tcx, def_id))
                {
                    values.insert(destination.local);
                }
            }
            _ => {}
        }
        if !ends {
            work.extend(terminator.successors().filter(|&b| !body[b].is_cleanup));
        }
    }
    (alone, summed, by, exceptions)
}

/// Where a call's callee is written: its constant's span, else the call's.
fn fun_span(func: &Operand<'_>, terminator: &mir::Terminator<'_>) -> rustc_span::Span {
    match func {
        Operand::Constant(c) => c.span,
        _ => terminator.source_info.span,
    }
}

/// Whether each of `guards` is a temporary, no `let`'s: what its statement
/// drops as it ends.
fn only_temporaries(body: &mir::Body<'_>, guards: &IndexSet<Local>) -> bool {
    guards
        .iter()
        .all(|&guard| !super::mir_reads::user_variable::user_variable(&body.local_decls[guard]))
}

/// Whether, from `from`, MIR runs straight on to a drop of one of
/// `guards`, calling nothing first: its statement's end.
fn dropped_next(body: &mir::Body<'_>, guards: &IndexSet<Local>, from: BasicBlock) -> bool {
    let mut at = from;
    for _ in 0..64 {
        match &body[at].terminator().kind {
            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => return true,
            TerminatorKind::Goto { target } | TerminatorKind::Drop { target, .. } => at = *target,
            _ => return false,
        }
    }
    false
}

/// Of a guard that's a temporary (`only_temporaries`) whose `Deref` gave
/// `value`, the blocks from `from` whose calls are the other arguments of
/// a call it's only the receiver of, that call its statement's last before
/// the guard drops, `target.borrow_mut().push(show(v))`'s `show(v)`: where
/// each way MIR goes from `from`, through an `if` or a `match` of them but
/// no loop, comes to that one call, and nothing else reads `value`.
fn receiver_only(
    body: &mir::Body<'_>,
    guards: &IndexSet<Local>,
    value: Local,
    from: BasicBlock,
) -> Option<Vec<BasicBlock>> {
    if !only_temporaries(body, guards) {
        return None;
    }
    let mut receivers: IndexSet<Local> = IndexSet::default();
    receivers.insert(value);
    let mut calls = Vec::new();
    // The call it's the receiver of, which each way comes to.
    let mut receiving: Option<BasicBlock> = None;
    let mut seen: IndexSet<BasicBlock> = IndexSet::default();
    let mut work = vec![from];
    while let Some(at) = work.pop() {
        if !seen.insert(at) {
            continue;
        }
        if seen.len() > 256 {
            return None;
        }
        let data = &body[at];
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind
                && let (place, Rvalue::Ref(_, _, of) | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..)) =
                    (&assign.0, &assign.1)
                && receivers.contains(&of.local)
                && place.projection.is_empty()
                && matches!(of.projection.as_slice(), [] | [mir::PlaceElem::Deref])
            {
                receivers.insert(place.local);
            }
        }
        let next = match &data.terminator().kind {
            TerminatorKind::Call { args, target, .. } => {
                let reads = |arg: &rustc_span::Spanned<Operand<'_>>| {
                    arg.node.place().is_some_and(|p| receivers.contains(&p.local))
                };
                match args.split_first() {
                    Some((receiver, rest)) if reads(receiver) => {
                        let last = !rest.iter().any(reads) && target.is_some_and(|t| dropped_next(body, guards, t));
                        if !last || receiving.is_some_and(|call| call != at) {
                            return None;
                        }
                        receiving = Some(at);
                        continue;
                    }
                    _ if args.iter().any(reads) => return None,
                    _ => {}
                }
                calls.push(at);
                (*target)?
            }
            TerminatorKind::Drop { place, target, .. } if !guards.contains(&place.local) => *target,
            TerminatorKind::Goto { target } | TerminatorKind::Assert { target, .. } => *target,
            TerminatorKind::SwitchInt { targets, .. } => {
                work.extend(targets.all_targets().iter().copied());
                continue;
            }
            TerminatorKind::FalseEdge {
                real_target,
                imaginary_target,
            } => {
                work.extend([*real_target, *imaginary_target]);
                continue;
            }
            TerminatorKind::Unreachable => continue,
            _ => return None,
        };
        work.push(next);
    }
    receiving.map(|_| calls)
}

/// The bucket of what `CellBorrows::unproven_call` says a call runs.
fn defeat_of(runs: super::cell_borrows::Unproven) -> Defeat {
    use super::cell_borrows::Unproven;
    match runs {
        Unproven::CrateFn => Defeat::CrateFn,
        Unproven::Closure => Defeat::Closure,
        Unproven::StdRunsCrate => Defeat::StdRunsCrate,
        Unproven::Generic => Defeat::Generic,
        Unproven::Unseen => Defeat::Unseen,
        Unproven::SecondBorrow => Defeat::SecondBorrow,
    }
}

/// Whether dropping a `ty` may ask of `held`: its `Drop` impl's body of
/// the crate's (`call_asks` of its `drop`), and each of its parts' drops
/// after it, a struct's fields', or what a std type holds, its type
/// arguments', an `Rc<State>`'s `State`'s, as std's drop glue runs only
/// theirs. `None` where that's not one the summaries know: a type
/// parameter's, a `dyn`'s, a library's `Drop`, or past a depth of parts.
fn destructor_asks<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    owner: DefId,
    ty: Ty<'tcx>,
    held: Borrow<'tcx>,
) -> Option<bool> {
    parts_drop_asks(proving, owner, ty, held, 8)
}

fn parts_drop_asks<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    owner: DefId,
    ty: Ty<'tcx>,
    held: Borrow<'tcx>,
    depth: u32,
) -> Option<bool> {
    let tcx = proving.recognition.tcx;
    if !(proving.drops)(ty) || proving.recognition.is_guard(ty) {
        return Some(false);
    }
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let depth = depth.checked_sub(1)?;
    let std = proving.recognition.is_std(adt.did());
    // Its parts, dropped after its own `drop`: of std's, its type
    // arguments, of the crate's, its fields.
    let parts: Vec<Ty<'tcx>> = match std {
        true => args.types().collect(),
        false => (adt.all_fields())
            .map(|field| field.ty(tcx, args).skip_normalization())
            .collect(),
    };
    let mut asks = false;
    for part in parts {
        asks |= parts_drop_asks(proving, owner, part, held, depth)?;
    }
    match tcx.adt_destructor(adt.did()) {
        // std's own `Drop`, of what it holds, its parts'.
        _ if std => Some(asks),
        None => Some(asks),
        Some(destructor) if destructor.did.is_local() => Some(
            asks || proving.borrows.call_asks(
                proving.recognition,
                (owner, destructor.did, args),
                None,
                held,
                proving.drops,
            ),
        ),
        Some(_) => None,
    }
}

/// What a borrow's `&RefCell`, `operand`, is of in its body: a field of
/// one of its parameters, `self.f` or `p.f`, or of a local, `l.f`; a
/// parameter or a local itself; or what a call gives, an `Rc`'s `Deref`.
/// And the field, of a struct's or a variant's.
fn receiver_of<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    operand: &Operand<'tcx>,
) -> (&'static str, Option<&'tcx ty::FieldDef>) {
    let assigned = crate::body_queries::assigned_once(body);
    let Some(mut local) = operand
        .place()
        .filter(|place| place.projection.is_empty())
        .map(|place| place.local)
    else {
        return ("place", None);
    };
    // Through reborrows, `&*r`.
    let lent = loop {
        match assigned.get(&local) {
            Some(Some(Rvalue::Ref(_, _, lent))) if lent.projection.as_slice() == [mir::PlaceElem::Deref] => {
                local = lent.local;
            }
            Some(Some(Rvalue::Ref(_, _, lent))) => break Some(*lent),
            _ => break None,
        }
    };
    let parameter = |local: Local| (1..=body.arg_count).contains(&local.as_usize());
    let field = lent.and_then(|lent| lent_field(tcx, body, lent));
    let of = match lent {
        Some(lent) if matches!(lent.projection.last(), Some(mir::PlaceElem::Field(..))) => {
            match parameter(lent.local) {
                true if lent.local.as_usize() == 1 && has_self(body) => "self-field",
                true => "param-field",
                false => "local-field",
            }
        }
        Some(lent) if parameter(lent.local) => "param",
        Some(_) => "local",
        None if parameter(local) => "param",
        None => "given",
    };
    (of, field)
}

/// The field `lent` is, of a struct's or of the variant its enum is read
/// as, `&x.f` or `&((*e) as Effect).cleanup`.
fn lent_field<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    lent: mir::Place<'tcx>,
) -> Option<&'tcx ty::FieldDef> {
    let (mir::PlaceElem::Field(field, _), parent) = lent.projection.split_last()? else {
        return None;
    };
    let parent = mir::PlaceRef {
        local: lent.local,
        projection: parent,
    }
    .ty(body, tcx);
    match parent.ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(&adt.non_enum_variant().fields[*field]),
        ty::Adt(adt, _) if adt.is_enum() => Some(&adt.variant(parent.variant_index?).fields[*field]),
        _ => None,
    }
}

/// Whether `body`'s function takes `self`.
fn has_self(body: &mir::Body<'_>) -> bool {
    body.var_debug_info.iter().any(|info| info.name.as_str() == "self")
}

/// Of a body's THIR, where each borrow's call is and what holds its guard:
/// its shape in the source, lisa's patterns' (#2490 comment 7132169613).
struct Shapes<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    /// Each expression's parent.
    parents: IndexMap<ExprId, ExprId>,
    /// Each `let` statement's initializer, and whether it has an `else`, and
    /// whether its pattern binds nothing it reads, `_held`.
    lets: IndexMap<ExprId, (bool, bool)>,
    /// Each call, by where its callee is written: `None` where two are.
    calls: IndexMap<rustc_span::Span, Option<ExprId>>,
}

impl<'t, 'tcx> Shapes<'t, 'tcx> {
    fn of(thir: &'t Thir<'tcx>) -> Self {
        let mut walk = Parents {
            thir,
            ids: (thir.exprs.iter_enumerated())
                .map(|(id, e)| (std::ptr::from_ref(e) as usize, id))
                .collect(),
            stack: Vec::new(),
            parents: IndexMap::default(),
        };
        if let Some(root) = thir.exprs.last_index() {
            walk.visit_expr(&thir[root]);
        }
        let mut lets = IndexMap::default();
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                initializer: Some(init),
                else_block,
                pattern,
                ..
            } = &stmt.kind
            {
                let unread = match &pattern.kind {
                    thir::PatKind::Wild => true,
                    thir::PatKind::Binding { name, .. } => name.as_str().starts_with('_'),
                    _ => false,
                };
                lets.insert(*init, (else_block.is_some(), unread));
            }
        }
        let mut calls: IndexMap<rustc_span::Span, Option<ExprId>> = IndexMap::default();
        for (id, e) in thir.exprs.iter_enumerated() {
            if let ExprKind::Call { fun, .. } = e.kind {
                calls
                    .entry(thir[fun].span)
                    .and_modify(|call| *call = None)
                    .or_insert(Some(id));
            }
        }
        Shapes {
            thir,
            parents: walk.parents,
            lets,
            calls,
        }
    }

    /// The call whose callee is written at `fun`, where one alone is: its
    /// THIR and its arguments.
    fn call_at(&self, fun: rustc_span::Span) -> Option<(&'t Thir<'tcx>, &'t [ExprId])> {
        let call = (*self.calls.get(&fun)?)?;
        match &self.thir[call].kind {
            ExprKind::Call { args, .. } => Some((self.thir, &args[..])),
            _ => None,
        }
    }

    /// The shape of the borrow whose callee is written at `fun`: what holds
    /// its guard, a `let`, a `match`'s, a `for`'s, an `if let`'s or a `while
    /// let`'s scrutinee, a `return`; else, a temporary of its statement, how
    /// it's used there, a method's receiver, read, written, an argument or
    /// a `Ref::map`'s.
    fn shape_at(
        &self,
        fun: rustc_span::Span,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let call = (thir.exprs.iter_enumerated())
            .find(|(_, e)| matches!(e.kind, ExprKind::Call { fun: f, .. } if thir[f].span == fun))?
            .0;
        Some(self.holder(call, classify).unwrap_or_else(|| self.used(call, classify)))
    }

    /// What holds the guard `call` makes past its statement, where one does:
    /// a `let` of the guard itself, or of a map of it, `Ref::map(c.borrow(),
    /// f)`; a scrutinee's temporary, which lives while its arms or its loop
    /// run, of a `match`, a `for`, an `if let` or a `while let`; a `let`-`else`'s;
    /// or a `return`. A `let` of what's made of it, `let n =
    /// c.borrow().len();`, drops it as the statement ends: none.
    fn holder(
        &self,
        call: ExprId,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let mut at = call;
        // Whether `at` is the guard, or a map of it, but for scopes.
        let mut itself = true;
        let mut mapped = false;
        loop {
            if let Some(&(with_else, unread)) = self.lets.get(&at) {
                return match (with_else, unread, itself) {
                    (true, ..) => Some("let-else"),
                    (_, _, false) => None,
                    _ if mapped => Some("map"),
                    (false, true, true) => Some("let-unread"),
                    (false, false, true) => Some("let"),
                };
            }
            let parent = *self.parents.get(&at)?;
            match &thir[parent].kind {
                ExprKind::Match { scrutinee, .. } if *scrutinee == at => {
                    return Some(if self.is_for(at) { "for" } else { "match" });
                }
                ExprKind::Let { expr, .. } if *expr == at => {
                    return Some(if self.in_loop(parent) { "while-let" } else { "if-let" });
                }
                ExprKind::Return { .. } if itself => return Some("returned"),
                // A statement's end, past which no temporary lives.
                ExprKind::Block { .. } | ExprKind::Loop { .. } | ExprKind::If { .. } | ExprKind::Closure(..) => {
                    return None;
                }
                ExprKind::Scope { .. } => {}
                ExprKind::Call { fun, .. }
                    if crate::fn_def(thir[*fun].ty)
                        .and_then(|(id, args)| classify(id, args))
                        .is_some_and(|std| matches!(std, Std::GuardMap { .. })) =>
                {
                    mapped = true;
                }
                _ => itself = false,
            }
            at = parent;
        }
    }

    /// How a temporary guard `call` makes is used in its statement.
    fn used(&self, call: ExprId, classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>) -> &'static str {
        let thir = self.thir;
        let mut at = call;
        loop {
            let Some(&parent) = self.parents.get(&at) else {
                return "tail";
            };
            match &thir[parent].kind {
                ExprKind::Scope { .. } | ExprKind::NeverToAny { .. } | ExprKind::Borrow { .. } => {}
                ExprKind::Deref { .. } => {
                    let mut place = parent;
                    while let Some(&up) = self.parents.get(&place)
                        && matches!(thir[up].kind, ExprKind::Scope { .. })
                    {
                        place = up;
                    }
                    let parent = place;
                    return match self.parents.get(&parent).map(|&p| &thir[p].kind) {
                        Some(ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. }) if *lhs == parent => {
                            "write"
                        }
                        Some(ExprKind::Borrow { .. } | ExprKind::RawBorrow { .. }) => self.lent_shape(parent),
                        _ => "read",
                    };
                }
                ExprKind::Call { fun, args, .. } => {
                    let std = crate::fn_def(thir[*fun].ty).and_then(|(id, args)| classify(id, args));
                    match std {
                        // The guard's own `Deref`, `*g` or a method's autoref.
                        Some(Std::GuardValue { .. }) => {}
                        Some(Std::GuardMap { .. }) => return "map",
                        _ => {
                            return if args.first() == Some(&at) {
                                "method"
                            } else {
                                "argument"
                            };
                        }
                    }
                }
                _ => return "other",
            }
            at = parent;
        }
    }

    /// Of `*guard` lent, `&*c.borrow()`: what takes the reference.
    fn lent_shape(&self, deref: ExprId) -> &'static str {
        let mut at = deref;
        while let Some(&parent) = self.parents.get(&at) {
            match &self.thir[parent].kind {
                ExprKind::Borrow { .. } | ExprKind::Scope { .. } => at = parent,
                ExprKind::Call { args, .. } => {
                    return if args.first() == Some(&at) {
                        "method"
                    } else {
                        "argument"
                    };
                }
                ExprKind::Match { .. } => return "match",
                _ => return "read",
            }
        }
        "read"
    }

    /// Whether `scrutinee`, a `match`'s, is a `for` loop's `IntoIterator::into_iter(..)`.
    fn is_for(&self, scrutinee: ExprId) -> bool {
        let mut at = scrutinee;
        loop {
            match &self.thir[at].kind {
                ExprKind::Scope { value, .. } => at = *value,
                ExprKind::Call { fn_span, .. } => return fn_span.is_desugaring(rustc_span::DesugaringKind::ForLoop),
                _ => return false,
            }
        }
    }

    /// Whether `e` is inside a `loop`, `while let`'s.
    fn in_loop(&self, mut e: ExprId) -> bool {
        while let Some(&parent) = self.parents.get(&e) {
            match &self.thir[parent].kind {
                ExprKind::Loop { .. } => return true,
                ExprKind::If { .. } | ExprKind::Scope { .. } | ExprKind::Block { .. } => e = parent,
                _ => return false,
            }
        }
        false
    }
}

/// Each THIR expression's parent, by a walk from the body's root.
struct Parents<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    ids: IndexMap<usize, ExprId>,
    stack: Vec<ExprId>,
    parents: IndexMap<ExprId, ExprId>,
}

impl<'t, 'tcx> Visitor<'t, 'tcx> for Parents<'t, 'tcx> {
    fn thir(&self) -> &'t Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'t thir::Expr<'tcx>) {
        let id = self.ids[&(std::ptr::from_ref(expr) as usize)];
        if let Some(&parent) = self.stack.last() {
            self.parents.insert(id, parent);
        }
        self.stack.push(id);
        visit::walk_expr(self, expr);
        self.stack.pop();
    }
}

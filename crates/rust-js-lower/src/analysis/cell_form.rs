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

mod shapes;
mod wholes;
use shapes::Shapes;
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
    /// A field's read or write of a struct JS may make or give, JS's own
    /// property access, which a getter or a setter may answer (#2113).
    PropertyAccess,
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
            Defeat::PropertyAccess => "property-access",
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
    /// Whether dropping a value of a type that names the body's type
    /// parameters runs code that may ask whether a cell is borrowed, as the
    /// body's lowering is given drops of them (`drops_taken`): none where
    /// it's given none.
    pub(crate) given_drops: &'r dyn Fn(Ty<'tcx>) -> bool,
}

/// A `RefCell` borrow's proof, of its guard's range (`guard_range`): its
/// call's block, its cell's type, what it's borrowed of and its field where
/// it's one, the first defeat alone and with the summaries, and the
/// function whose call is the second, where it's a call's.
// Flags, not a state: a `try_borrow` or not, and whether unseen code runs
// in its range, either or both.
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
    /// Whether code rust-js can't see may run while it's held
    /// (`GuardRange::unseen`).
    unseen: bool,
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
        let GuardRange {
            alone,
            summed,
            by,
            exceptions,
            unseen,
        } = guard_range(
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
            unseen,
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
            unseen: _,
        } = ranged;
        // A field used whole, where its range proved nothing else: its
        // cell kept whole, with its flag, which each borrow checks, and is
        // never counted where the range is quiet, a momentary borrow
        // (design point 6); with the summaries, unchecked where proof A or
        // B passes it. A declared form has no flag: the proof fails it.
        let taken = today(block, cell, field);
        let (alone_checked, summed_checked) = judged(
            proving,
            (owner, block, tried, field),
            (&mut alone, &mut summed, &mut by),
            (wholes, proofs),
        );
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

/// Of a borrow, `owner`'s at `block`, a `try_borrow` where `tried`, of
/// `field` where it's one, its range's defeats, alone and with the
/// summaries, and the function whose call is the second, as the proof
/// takes them past its range: a `try_borrow` counted by design, but one
/// nothing can find borrowed (`TypeProofs::uncontested`) or a flattened
/// state's (#687's rule); a field used whole (`wholes`) checked, where
/// proofs A and B don't pass it (design point 6), but a flattened state's,
/// which has no flag and whose whole uses are its declaration's. Whether
/// each, alone and with the summaries, is checked and not counted, a field
/// used whole's.
fn judged<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    (owner, block, tried, field): (DefId, BasicBlock, bool, Option<&'tcx ty::FieldDef>),
    (alone, summed, by): (&mut Option<Defeat>, &mut Option<Defeat>, &mut Option<DefId>),
    (wholes, proofs): (&IndexMap<DefId, Whole>, &TypeProofs),
) -> (bool, bool) {
    let tcx = proving.recognition.tcx;
    let declared = field.is_some_and(|field| crate::bindings::is_flattened_state_field(tcx, field));
    if tried && !declared && !proofs.uncontested.contains(&(owner, block)) {
        (*alone, *summed, *by) = (Some(Defeat::TryBorrow), Some(Defeat::TryBorrow), None);
    }
    let (mut alone_checked, mut summed_checked) = (false, false);
    if let Some((field, whole)) = field.and_then(|field| Some((field.did, wholes.get(&field.did)?))) {
        let proved = whole.lent_to_borrows || proofs.quiet_typed.contains(&field);
        match declared {
            // A declared form has no flag, and is never counted: its whole
            // uses are its declaration's to refuse or keep (`flattened_state_lent`,
            // #2478), and the proof fails it only by its range, as it
            // refuses nothing that compiles today (Tu, #2490).
            true => _ = alone.get_or_insert(Defeat::WholeUse),
            false => {
                alone_checked = alone.is_none();
                summed_checked = summed.is_none() && !proved;
            }
        }
    }
    (alone_checked, summed_checked)
}

/// Each `RefCell` borrow's form, of every body of the crate, the proof's
/// with the summaries (#2490's flip, design point 1): counted unless its
/// guard's range is proved to run nothing that may ask of its cell. A
/// borrow not counted is checked where it's made, or unchecked where no
/// borrow of its cell can meet another, as its cell's other borrows say
/// (`RefCells::quiet_class`, `quiet_field`). Each borrow by its body and
/// where its callee is written, as THIR's facts and MIR's lowering find it.
/// The one decider of a `RefCell` borrow's count: THIR's facts carry its
/// answers, and decide nothing of their own.
#[derive(Default)]
pub(crate) struct CellForms {
    /// Of each body, by where each borrow's callee is written, its form.
    forms: IndexMap<DefId, IndexMap<rustc_span::Span, CellFormOf>>,
}

/// What the proof says of a borrow, of each written at one place, which
/// it can't tell apart: the quieter answer only where each has it.
// Flags, not a state: two answers of the proof, either or both.
#[derive(Clone, Copy)]
struct CellFormOf {
    /// Proved not counted.
    uncounted: bool,
    /// Held only while code rust-js sees runs (`GuardRange::unseen`).
    seen: bool,
}

impl CellForms {
    /// Of the crate's bodies, each its MIR, its THIR and the proving of its
    /// own typing, `wholes`, its fields used whole, and `proofs`, its
    /// types'.
    pub(crate) fn of_crate<'b, 'tcx: 'b>(
        bodies: impl Iterator<Item = (&'b mir::Body<'tcx>, &'b Thir<'tcx>)>,
        prove: &Prove<'_, 'tcx>,
        (wholes, proofs): (&IndexMap<DefId, Whole>, &TypeProofs),
    ) -> Self {
        let mut of: IndexMap<DefId, IndexMap<rustc_span::Span, CellFormOf>> = IndexMap::default();
        for (body, thir) in bodies {
            let owner = body.source.def_id();
            let shapes = (!thir.exprs.is_empty()).then(|| Shapes::of(thir));
            let forms = of.entry(owner).or_default();
            prove(owner, &mut |proving| {
                for mut ranged in ranges_of(proving, body, shapes.as_ref()) {
                    judged(
                        proving,
                        (owner, ranged.block, ranged.tried, ranged.field),
                        (&mut ranged.alone, &mut ranged.summed, &mut ranged.by),
                        (wholes, proofs),
                    );
                    let data = &body[ranged.block];
                    let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
                        continue;
                    };
                    let form = CellFormOf {
                        uncounted: ranged.summed.is_none(),
                        seen: !ranged.unseen,
                    };
                    forms
                        .entry(fun_span(func, data.terminator()))
                        .and_modify(|both| {
                            both.uncounted &= form.uncounted;
                            both.seen &= form.seen;
                        })
                        .or_insert(form);
                }
            });
        }
        CellForms { forms: of }
    }

    fn form(&self, owner: DefId, fun: rustc_span::Span) -> Option<CellFormOf> {
        self.forms.get(&owner)?.get(&fun).copied()
    }

    /// Whether `owner`'s borrow whose callee is written at `fun` is counted:
    /// unless the proof says it isn't.
    pub(crate) fn counted(&self, owner: DefId, fun: rustc_span::Span) -> bool {
        !self.form(owner, fun).is_some_and(|form| form.uncounted)
    }

    /// Whether `owner`'s borrow whose callee is written at `fun` is held
    /// only while code rust-js sees runs, the crate's or std's, no
    /// library's nor JS's (ADR 0863): what a library that borrows its cell
    /// unchecked asks of a counted one (`RefCells::agrees`).
    pub(crate) fn seen(&self, owner: DefId, fun: rustc_span::Span) -> bool {
        self.form(owner, fun).is_some_and(|form| form.seen)
    }
}

/// ADR 0262's difference 39: a guard that's only a call's receiver is held
/// while the call runs, not while its other arguments do.
const RECEIVER_ONLY: u32 = 39;
/// ADR 0262's difference 44: a guard read only for a binding's argument is
/// held while it's read, not while the binding runs.
const ARGUMENT_READ: u32 = 44;

/// What the proof finds of a guard's range (`guard_range`).
struct GuardRange {
    /// The first defeat, alone and with the summaries.
    alone: Option<Defeat>,
    summed: Option<Defeat>,
    /// The function whose call is the second, where it's a call's.
    by: Option<DefId>,
    /// The ADR 0262 differences its range is passed by, Tu's decisions,
    /// not proofs.
    exceptions: IndexSet<u32>,
    /// Whether a call in it may run code rust-js can't see, a library's or
    /// JS's, a body no summary is of, or what may be any code, or it
    /// suspends or runs assembly (ADR 0863): a library that borrows its
    /// cell unchecked refuses a counted one (`RefCells::agrees`).
    unseen: bool,
}

/// Of the guard a borrow ending `block` makes in `local`, what its range
/// holds (`GuardRange`): from the borrow's return to each drop of it, a
/// move of it to a local followed, an unwind's cleanup left out (it runs
/// nothing a program sees past the panic).
fn guard_range<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    (body, shapes): (&mir::Body<'tcx>, Option<&Shapes<'_, 'tcx>>),
    (made, local): (BasicBlock, Local),
    (held, held_field): (Borrow<'tcx>, Option<DefId>),
) -> GuardRange {
    let tcx = proving.recognition.tcx;
    let owner = body.source.def_id();
    let mut guards: IndexSet<Local> = IndexSet::default();
    guards.insert(local);
    // Of `guards`, each moved whole out of, to a local or a call, whose
    // drop then drops nothing: the range goes on past it (#2490).
    let mut moved: IndexSet<Local> = IndexSet::default();
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
    let mut unseen = false;
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
    // Where the borrow's callee is written, as its THIR has it.
    let own = match &body[made].terminator().kind {
        TerminatorKind::Call { func, .. } => fun_span(func, body[made].terminator()),
        _ => body[made].terminator().source_info.span,
    };
    let Some(next) = (body[made].terminator().successors()).find(|&b| !body[b].is_cleanup) else {
        return GuardRange {
            alone: None,
            summed: None,
            by: None,
            exceptions,
            unseen,
        };
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
                // What holds no guard, a `try_borrow`'s `Err`, isn't it.
                Rvalue::Use(operand, ..)
                    if moves(operand, &guards)
                        && (operand.ty(body, tcx).walk())
                            .any(|part| part.as_type().is_some_and(|ty| proving.recognition.is_guard(ty))) =>
                {
                    match place.projection.is_empty() {
                        true => {
                            guards.insert(place.local);
                            // Moved out of whole, or out of an enum's payload, a
                            // `Result`'s or an `Option`'s, which holds one guard;
                            // not out of a tuple's part, a `map_split`'s half.
                            let whole = |from: &mir::Place<'tcx>| {
                                from.projection.is_empty()
                                    || from
                                        .projection
                                        .iter()
                                        .any(|elem| matches!(elem, mir::PlaceElem::Downcast(..)))
                            };
                            moved.extend(operand.place().filter(whole).map(|from| from.local));
                        }
                        false => note(Defeat::Escapes, false, None),
                    }
                }
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
        if property_access_asks(proving, body, (block, data), held) {
            note(Defeat::PropertyAccess, false, None);
        }
        let terminator = data.terminator();
        unseen |= runs_unseen(proving, (body, shapes), terminator);
        let mut ends = false;
        // Whether a guard's drop ends the range: of one still holding it,
        // not one moved out of, whose drop drops nothing (#2490). A map's
        // guards, `map_split`'s two halves, are counted (`GuardMap`).
        let last = |dropped: Local, moved: &IndexSet<Local>| !moved.contains(&dropped);
        // A value's drop while it's held, `ty`'s, which runs its destructor:
        // a type parameter's what each caller's instance runs, its callers'
        // to say (#1321), each instance's where every one is seen.
        let destructor = |ty: Ty<'tcx>| -> Option<bool> {
            if crate::generic_facts::may_be_any_type(ty) && ty.needs_drop(tcx, proving.recognition.typing_env) {
                // Given no drop of it that may ask, its drop asks nothing in
                // JS, whatever its instances are, as its lowering drops it.
                if !(proving.given_drops)(ty) {
                    return None;
                }
                let instances = proving.borrows.instances_of(tcx, owner, ty);
                let runs = |ty: Ty<'tcx>| (proving.drops)(ty) && !proving.recognition.is_guard(ty);
                let ran: Vec<Ty<'tcx>> = instances.iter().flatten().copied().filter(|&ty| runs(ty)).collect();
                (instances.is_none() || !ran.is_empty()).then(|| {
                    instances.is_some()
                        && ran
                            .iter()
                            .all(|&ty| destructor_asks(proving, owner, ty, held).is_some_and(|asks| !asks))
                })
            } else if (proving.drops)(ty) && !proving.recognition.is_guard(ty) {
                Some(destructor_asks(proving, owner, ty, held).is_some_and(|asks| !asks))
            } else {
                None
            }
        };
        match &terminator.kind {
            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => {
                ends = place.projection.is_empty() && last(place.local, &moved);
            }
            TerminatorKind::Drop { place, .. } => {
                if let Some(won) = destructor(place.ty(body, tcx).ty) {
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
                // Each guard moved whole into the call.
                let given: Vec<Local> = (args.iter())
                    .filter_map(|arg| match &arg.node {
                        Operand::Move(place) if place.projection.is_empty() && guards.contains(&place.local) => {
                            Some(place.local)
                        }
                        _ => None,
                    })
                    .collect();
                match proving.recognition.classify(def_id, generic_args) {
                    // `drop(g)`: its range ends, of the last guard holding it.
                    Some(Std::Drop) if given_guard => {
                        ends = given.iter().any(|&guard| last(guard, &moved));
                        moved.extend(given);
                    }
                    // `drop(v)` of another value: its destructor runs.
                    Some(Std::Drop) => {
                        let dropped = args.first().map(|arg| arg.node.ty(body, tcx));
                        if let Some(won) = dropped.and_then(destructor) {
                            note(Defeat::Destructor, won, None);
                        }
                    }
                    // `Ref::map(g, f)`: the mapped guard is the guard, `f` run
                    // while it's held.
                    // Its mapped guard releases a count as it drops, whatever
                    // its borrow was: counted (#1275).
                    Some(Std::GuardMap { .. }) if given_guard => {
                        note(Defeat::Escapes, false, None);
                        guards.insert(destination.local);
                        moved.extend(given);
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
                        let unmet = shapes
                            .is_some_and(|shapes| shapes.unmet(proving.recognition, own, fun_span(func, terminator)));
                        if proving.borrows.borrows_meet(tcx, theirs, held) && !unmet {
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
    GuardRange {
        alone,
        summed,
        by,
        exceptions,
        unseen,
    }
}

/// Whether a field's read or write in `data`, `block` of `body`, a guard's
/// range's, may ask of `held`: JS's property access of a struct JS may make
/// or give, which a getter or a setter of the program's may answer, by the
/// type it's of (`CellBorrows::property_access_asks`, #2113), as a THIR
/// field's is (`field_access_may_run`): a field of a struct of the
/// program's or a binding's. Not a variant's field, read by rust-js's own
/// form of an enum, nor what a closure took, a variable of its maker's in
/// JS, nor a part of std's own, a `Box`'s pointer MIR writes, which no
/// program reads.
fn property_access_asks<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    body: &mir::Body<'tcx>,
    (block, data): (BasicBlock, &mir::BasicBlockData<'tcx>),
    held: Borrow<'tcx>,
) -> bool {
    struct Projected<'b, 'r, 'a, 'tcx> {
        body: &'b mir::Body<'tcx>,
        recognition: &'r Recognition<'a, 'tcx>,
        /// The type of each place a field is read or written of.
        of: Vec<Ty<'tcx>>,
    }
    impl<'tcx> mir::visit::Visitor<'tcx> for Projected<'_, '_, '_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, _: mir::visit::PlaceContext, _: mir::Location) {
            for (base, elem) in place.iter_projections() {
                let mir::PlaceElem::Field(..) = elem else { continue };
                let of = base.ty(self.body, self.recognition.tcx).ty;
                if matches!(of.kind(), ty::Adt(adt, _) if adt.is_struct() && !self.recognition.is_std(adt.did())) {
                    self.of.push(of);
                }
            }
        }
    }
    let mut projected = Projected {
        body,
        recognition: proving.recognition,
        of: Vec::new(),
    };
    mir::visit::Visitor::visit_basic_block_data(&mut projected, block, data);
    (projected.of.into_iter()).any(|of| (proving.borrows).property_access_asks(proving.recognition, of, held))
}

/// Whether `terminator`, of `body`, in a guard's range, may run code rust-js
/// can't see (`GuardRange::unseen`): a call `CellBorrows::call_runs_unseen`
/// says may, but a promise's resolve given what runs nothing (rd654), a fn
/// pointer's that may be such code, a `yield` or assembly.
fn runs_unseen<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    (body, shapes): (&mir::Body<'tcx>, Option<&Shapes<'_, 'tcx>>),
    terminator: &mir::Terminator<'tcx>,
) -> bool {
    let owner = body.source.def_id();
    match &terminator.kind {
        TerminatorKind::Call { func, .. } => match func.const_fn_def() {
            Some((def_id, generic_args)) => {
                let call = shapes.and_then(|shapes| shapes.call_at(fun_span(func, terminator)));
                !call.is_some_and(|(thir, call_args)| {
                    resolve_runs_nothing(proving.recognition, thir, (def_id, generic_args), call_args)
                }) && (proving.borrows).call_runs_unseen(proving.recognition, (owner, def_id, generic_args))
            }
            None => {
                (proving.borrows).pointer_call_runs_unseen(proving.recognition, func.ty(body, proving.recognition.tcx))
            }
        },
        TerminatorKind::Yield { .. } | TerminatorKind::InlineAsm { .. } | TerminatorKind::TailCall { .. } => true,
        _ => false,
    }
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

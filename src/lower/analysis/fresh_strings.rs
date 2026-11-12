//! Whether a `&mut String` a call is given is to a `String` that is
//! provably fresh-empty there: `String::new()` or `String::default()`,
//! made in its place, a local or a field of one, or in a temporary moved
//! into it, `(t,)` or `s = t`, with nothing reading or writing that local
//! between that and the borrow the call is given, on every path to it
//! (#1297). Its capacity is 0 and its length
//! 0, as a JS string's, which carries no capacity (#1261), can't say of
//! another. What's read into it, by a reader of the crate's own, is then
//! read as into a `Vec::new()`, std's reads exactly. One owner: lowering
//! reads it of a call (`fresh_string_given`); it's never an answer of a
//! `String`'s `capacity()`, which stays refused.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_index::bit_set::DenseBitSet;
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{self, BasicBlock, Local, Location, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};

/// Whether the call ending `at` gives, as its argument `arg`, a `&mut` to a
/// `String` provably fresh-empty there.
pub(in crate::lower) fn fresh_string_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    at: BasicBlock,
    arg: usize,
) -> bool {
    let TerminatorKind::Call { args, .. } = &body[at].terminator().kind else {
        return false;
    };
    let Some(Operand::Move(given) | Operand::Copy(given)) = args.get(arg).map(|a| &a.node) else {
        return false;
    };
    if !given.projection.is_empty() {
        return false;
    }
    // The borrow given, through reborrows `&mut *r` of a reference nothing
    // else uses, back to `&mut s` of the `String` local.
    let mut reference = given.local;
    let (string, borrowed_at) = loop {
        let Some((place, location)) = only_definition(body, reference) else {
            return false;
        };
        if uses(body, reference) != 2 {
            return false;
        }
        match place {
            Borrowed::Place(place) => break (place, location),
            Borrowed::Reborrow(of) => reference = of,
        }
    };
    let is_string = matches!(string.ty(body, tcx).ty.kind(),
        ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String));
    is_string && made_empty_before(tcx, body, string, borrowed_at)
}

/// What a reference local is made of, by its one assignment.
enum Borrowed<'tcx> {
    /// `&mut s`, or `&mut s.0`: a local, or a field of one.
    Place(mir::Place<'tcx>),
    /// `&mut *r`.
    Reborrow(Local),
}

/// `local`'s one assignment, a `&mut` borrow, and where it is: `None` where
/// it's assigned anything else, or more than once.
fn only_definition<'tcx>(body: &mir::Body<'tcx>, local: Local) -> Option<(Borrowed<'tcx>, Location)> {
    let mut found = None;
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (index, statement) in data.statements.iter().enumerate() {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            if place.local != local {
                continue;
            }
            if found.is_some() || !place.projection.is_empty() {
                return None;
            }
            let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) = rvalue else {
                return None;
            };
            let borrowed = match of.projection.as_slice() {
                [mir::ProjectionElem::Deref] => Borrowed::Reborrow(of.local),
                fields if fields.iter().all(|elem| matches!(elem, mir::ProjectionElem::Field(..))) => {
                    Borrowed::Place(*of)
                }
                _ => return None,
            };
            found = Some((
                borrowed,
                Location {
                    block,
                    statement_index: index,
                },
            ));
        }
        if let TerminatorKind::Call { destination, .. } = &data.terminator().kind
            && destination.local == local
        {
            return None;
        }
    }
    found
}

/// How many times `local` is used, its definition among them.
fn uses(body: &mir::Body<'_>, local: Local) -> usize {
    struct Count {
        local: Local,
        n: usize,
    }
    impl<'tcx> Visitor<'tcx> for Count {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: Location) {
            if local == self.local && !matches!(context, PlaceContext::NonUse(_)) {
                self.n += 1;
            }
        }
    }
    let mut count = Count { local, n: 0 };
    count.visit_body(body);
    count.n
}

/// Whether, on every path to `at`, the last thing done with `string`'s
/// local is the making of `string`: `String::new()` or `String::default()`
/// made in it, or a temporary made so moved into it, whole, `s = t`, or as
/// its field, `s = (t,)`. Walking back from `at`, each path meets that
/// before any other use of the local, and none reaches the body's start.
fn made_empty_before<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, string: mir::Place<'tcx>, at: Location) -> bool {
    let predecessors = body.basic_blocks.predecessors();
    let mut seen = DenseBitSet::new_empty(body.basic_blocks.len());
    // Each block to walk back through from its end, but `at`'s, from `at`.
    let mut work = vec![(at.block, Some(at.statement_index))];
    'paths: while let Some((block, before)) = work.pop() {
        let data = &body[block];
        let end = match before {
            Some(index) => index,
            None => {
                if !seen.insert(block) {
                    continue;
                }
                let location = Location {
                    block,
                    statement_index: data.statements.len(),
                };
                if makes_empty(tcx, data.terminator(), string) {
                    continue;
                }
                if touches(string.local, |v| v.visit_terminator(data.terminator(), location)) {
                    return false;
                }
                data.statements.len()
            }
        };
        for index in (0..end).rev() {
            let location = Location {
                block,
                statement_index: index,
            };
            let statement = &data.statements[index];
            // What borrowck alone reads, a `let`'s `FakeRead` or
            // `PlaceMention` of it, or a type it's said to be, does nothing.
            if matches!(
                statement.kind,
                StatementKind::FakeRead(..) | StatementKind::PlaceMention(..) | StatementKind::AscribeUserType(..)
            ) {
                continue;
            }
            // A temporary moved into it, which must be made empty before.
            if let Some(moved) = moved_into(statement, string) {
                if !made_empty_before(tcx, body, moved, location) {
                    return false;
                }
                continue 'paths;
            }
            if touches(string.local, |v| v.visit_statement(statement, location)) {
                return false;
            }
        }
        if block == mir::START_BLOCK {
            return false;
        }
        // Each block that may run before it.
        for &from in &predecessors[block] {
            work.push((from, None));
        }
    }
    true
}

/// The temporary `statement` moves into `string`, whole, `s = move t`, or
/// as the field of the aggregate it makes of its local, `s = (move t,)`.
fn moved_into<'tcx>(statement: &mir::Statement<'tcx>, string: mir::Place<'tcx>) -> Option<mir::Place<'tcx>> {
    let StatementKind::Assign(assign) = &statement.kind else {
        return None;
    };
    let (place, rvalue) = &**assign;
    if place.local != string.local || !string.projection.starts_with(place.projection) {
        return None;
    }
    let moved = match (rvalue, &string.projection[place.projection.len()..]) {
        (Rvalue::Use(Operand::Move(moved), _), []) => moved,
        (Rvalue::Aggregate(_, operands), [mir::ProjectionElem::Field(field, _)]) => match operands.get(*field)? {
            Operand::Move(moved) => moved,
            _ => return None,
        },
        _ => return None,
    };
    moved.projection.is_empty().then_some(*moved)
}

/// Whether `terminator` is `String::new()` or `String::default()` made in
/// `string` itself.
fn makes_empty<'tcx>(tcx: TyCtxt<'tcx>, terminator: &mir::Terminator<'tcx>, string: mir::Place<'tcx>) -> bool {
    let TerminatorKind::Call { func, destination, .. } = &terminator.kind else {
        return false;
    };
    *destination == string
        && func
            .const_fn_def()
            .is_some_and(|(callee, args)| crate::lower::recognition::makes_empty_string(tcx, callee, args))
}

/// Whether what `visit` visits uses `string`'s local.
fn touches(string: Local, visit: impl FnOnce(&mut Touch)) -> bool {
    let mut touch = Touch { string, found: false };
    visit(&mut touch);
    touch.found
}

struct Touch {
    string: Local,
    found: bool,
}

impl<'tcx> Visitor<'tcx> for Touch {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: Location) {
        if local == self.string && !matches!(context, PlaceContext::NonUse(_)) {
            self.found = true;
        }
    }
}

//! What each local's later uses ask of its value, by std's steps (#1458,
//! #2474's item 6): the steps of `Iterator`, `DoubleEndedIterator` and
//! `ExactSizeIterator` called of it, and of each adapter made of it what
//! the adapter's own asked steps ask of what it holds, by std's catalog
//! alone (`std_steps::asked_of_held`). A local whose uses aren't each
//! read here, given to a call that isn't a step of it, returned, stored
//! or moved into a value, is asked every step (`Steps::ALL`), as any may
//! be. A value's question, by its maker's MIR identity, never its type's:
//! `BackStepped::asked_of`'s `steps`.

use crate::Body;
use crate::recognition::{Recognition, StdItem, Steps, is_double_ended, is_exact_size, is_std_def};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, visit::Visitor};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// What `all_bodies`' later uses ask of each local, by its body: each
/// body read from its MIR, its locals asked none but those its uses ask.
pub(super) fn asked_steps<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> (IndexMap<(DefId, mir::Local), Steps>, IndexSet<DefId>) {
    let mut asked: IndexMap<(DefId, mir::Local), Steps> = IndexMap::default();
    let mut read: IndexSet<DefId> = IndexSet::default();
    for body in all_bodies {
        let Some(mir) = body.mir.as_ref() else { continue };
        read.insert(body.def_id);
        for (local, steps) in of_body(tcx, recognition, &mir.body) {
            asked.insert((body.def_id, local), steps);
        }
    }
    (asked, read)
}

/// How a local's value reaches another's, whose asks it then asks: the
/// same value, moved or lent, `_5 = move _4` or `_5 = &mut _4`; or what
/// std's adapter of the catalog's name holds, `_5 = map(move _4, ..)`.
enum Reached {
    Itself,
    Held(String),
}

fn of_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    mir: &mir::Body<'tcx>,
) -> IndexMap<mir::Local, Steps> {
    let is_step_trait =
        |id: DefId| is_std_def(tcx, id, StdItem::Iterator) || is_double_ended(tcx, id) || is_exact_size(tcx, id);
    let mut steps: IndexMap<mir::Local, Steps> = IndexMap::default();
    // Of each pair, the second's value reaches the first's.
    let mut reaches: Vec<(mir::Local, mir::Local, Reached)> = Vec::new();
    // Each use read here, by where it is: no other of the local is.
    let mut seen: IndexSet<(mir::Location, mir::Local)> = IndexSet::default();
    let ask = |steps: &mut IndexMap<mir::Local, Steps>, local: mir::Local, asked: Steps| {
        let at = steps.entry(local).or_default();
        *at = at.and(asked);
    };
    // The return place is the caller's, which may ask any step.
    ask(&mut steps, mir::RETURN_PLACE, Steps::ALL);
    for (block, data) in mir.basic_blocks.iter_enumerated() {
        for (index, statement) in data.statements.iter().enumerate() {
            let location = mir::Location {
                block,
                statement_index: index,
            };
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (to, rvalue) = &**assign;
            let Some(to) = to.as_local() else { continue };
            let from = match rvalue {
                mir::Rvalue::Use(mir::Operand::Move(of) | mir::Operand::Copy(of), _) => of.as_local(),
                mir::Rvalue::Ref(_, _, of) => match of.as_local() {
                    Some(of) => Some(of),
                    // A reborrow, `&mut (*_r)`, is what `_r` lends.
                    None => match of.projection.as_slice() {
                        [mir::ProjectionElem::Deref] => Some(of.local),
                        _ => None,
                    },
                },
                _ => None,
            };
            if let Some(from) = from {
                seen.insert((location, from));
                reaches.push((to, from, Reached::Itself));
            }
        }
        let mir::TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let location = mir::Location {
            block,
            statement_index: data.statements.len(),
        };
        let Some((callee, _)) = func.const_fn_def() else {
            continue;
        };
        let Some(of_trait) = tcx.trait_of_assoc(callee) else {
            continue;
        };
        let into_iter = is_std_def(tcx, of_trait, StdItem::IntoIterator);
        if !is_step_trait(of_trait) && !into_iter {
            continue;
        }
        let Some(receiver) = args.first().and_then(|arg| match &arg.node {
            mir::Operand::Move(of) | mir::Operand::Copy(of) => of.as_local(),
            _ => None,
        }) else {
            continue;
        };
        let name = tcx.item_name(callee);
        let made = destination.as_local();
        let made_ty = made.map(|made| mir.local_decls[made].ty);
        // What's made of the receiver: the same value of an iterator's
        // `into_iter()`, or std's adapter, whose catalog says what each of
        // its steps asks of it. Each other argument isn't read here.
        let reached = match () {
            _ if into_iter && made_ty.is_some_and(|ty| ty.peel_refs() == mir.local_decls[receiver].ty.peel_refs()) => {
                Some(Reached::Itself)
            }
            _ if name.as_str() == "by_ref" => Some(Reached::Itself),
            _ => made_ty.and_then(|ty| recognition.catalog_name(ty)).map(Reached::Held),
        };
        match (made, reached) {
            (Some(made), Some(reached)) => reaches.push((made, receiver, reached)),
            _ if into_iter => continue,
            _ => ask(&mut steps, receiver, Steps::of(name.as_str())),
        }
        seen.insert((location, receiver));
    }
    // Each other use of a local, but where it's made and its storage's
    // marks and drops, asks every step: what it's given to, a call that
    // isn't its step, a value it's moved into, may ask any.
    struct Unread<'a> {
        seen: &'a IndexSet<(mir::Location, mir::Local)>,
        unread: IndexSet<mir::Local>,
    }
    impl<'tcx> Visitor<'tcx> for Unread<'_> {
        fn visit_local(&mut self, local: mir::Local, context: mir::visit::PlaceContext, location: mir::Location) {
            use mir::visit::{MutatingUseContext, NonMutatingUseContext, NonUseContext, PlaceContext};
            let made = matches!(
                context,
                PlaceContext::MutatingUse(
                    MutatingUseContext::Store | MutatingUseContext::Call | MutatingUseContext::Drop
                ) | PlaceContext::NonUse(NonUseContext::StorageLive | NonUseContext::StorageDead)
                    | PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect)
            );
            if !made && !self.seen.contains(&(location, local)) {
                self.unread.insert(local);
            }
        }
    }
    let mut unread = Unread {
        seen: &seen,
        unread: IndexSet::default(),
    };
    unread.visit_body(mir);
    for local in unread.unread {
        ask(&mut steps, local, Steps::ALL);
    }
    // What each value reaches asks of it, to a fixed point.
    loop {
        let mut changed = false;
        for (to, from, how) in &reaches {
            let of_to = steps.get(to).copied().unwrap_or_default();
            let asked = match how {
                Reached::Itself => of_to,
                Reached::Held(name) => crate::recognition::asked_of_held(name, of_to, "src"),
            };
            let at = steps.entry(*from).or_default();
            if at.and(asked) != *at {
                *at = at.and(asked);
                changed = true;
            }
        }
        if !changed {
            return steps;
        }
    }
}

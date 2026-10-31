//! A slice's `position` whose `Option` is only tested and its index taken,
//! `if let Some(index) = children.iter().position(|c| Rc::ptr_eq(c, &child))`:
//! which ones are (`mir_found_indexes`, a fact `LocalFacts` keeps), and JS's
//! own search of the array each is, `children.indexOf(child)` of a test by
//! `===`, else `children.findIndex(f)`, whose `-1` is its `None`, `if (index
//! !== -1)`, as a person writes it (rd316, ADR 1040).
//!
//! `findIndex` calls `f` on each item in order and stops at the first it
//! finds, as `position` does, and `indexOf(x)` finds the first item `===`
//! `x`, as `findIndex((c) => c === x)` does, `x` read once where it reads
//! the same each time. The `Option` is the body's alone, so nothing else
//! reads its `-1` (ADR 0262's first case).

use super::*;
use crate::lower::recognition::{is_iterator_position, is_slice_iter};

/// Of a body: each block whose call is a `position` `found_index` lowers,
/// and the `Option` each gives.
#[derive(Default)]
pub(super) struct FoundIndexes {
    calls: HashSet<BasicBlock>,
    /// The block of each one's slice's `iter()`, the array itself.
    items: HashSet<BasicBlock>,
    locals: HashSet<Local>,
}

impl FoundIndexes {
    /// Whether the call that ends `block` is a slice's `position` whose
    /// `Option` is only tested and its index taken.
    pub(super) fn call(&self, block: BasicBlock) -> bool {
        self.calls.contains(&block)
    }

    /// Whether the call that ends `block` is the `iter()` of a slice such
    /// a `position` searches: the array itself, which JS's search is of.
    pub(super) fn items(&self, block: BasicBlock) -> bool {
        self.items.contains(&block)
    }

    /// Whether `local` is such a `position`'s `Option`, `-1` where it's
    /// `None`.
    pub(super) fn holds(&self, local: Local) -> bool {
        self.locals.contains(&local)
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The `position`s `LocalFacts::found_indexes` says: each of a slice's
    /// `iter()`, made for it and stepped by nothing else, whose `Option`,
    /// written there alone, is read only by its discriminant and its
    /// `Some`'s index (`tested_and_taken`).
    pub(super) fn mir_found_indexes(&self, body: &mir::Body<'tcx>) -> FoundIndexes {
        let mut found = FoundIndexes::default();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let positioned = fn_def(func.ty(&body.local_decls, self.tcx))
                .is_some_and(|(def_id, _)| is_iterator_position(self.tcx, def_id));
            let fresh = (args.first().and_then(|receiver| receiver.node.place()))
                .filter(|receiver| receiver.projection.is_empty())
                .and_then(|receiver| self.fresh_slice_items(body, receiver.local));
            if positioned
                && let Some(items) = fresh
                && destination.projection.is_empty()
                && tested_and_taken(body, destination.local, 1)
            {
                found.calls.insert(block);
                found.items.insert(items);
                found.locals.insert(destination.local);
            }
        }
        found
    }

    /// Whether `receiver`, a `&mut` an iterator's method is given, is to a
    /// slice's `iter()` made for it: written once, `&mut it`, of an `it` a
    /// slice's `iter()` writes and nothing else reads. The block of that
    /// `iter()`.
    fn fresh_slice_items(&self, body: &mir::Body<'tcx>, receiver: Local) -> Option<BasicBlock> {
        let mut borrowed = None;
        let mut writes = 0;
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            if let StatementKind::Assign(assign) = &statement.kind
                && assign.0.local == receiver
            {
                writes += 1;
                borrowed = match &assign.1 {
                    Rvalue::Ref(_, BorrowKind::Mut { .. }, it) if it.projection.is_empty() => Some(it.local),
                    _ => None,
                };
            }
        }
        let it = borrowed.filter(|_| writes == 1)?;
        // Made by a slice's `iter()`, its items as the array has them.
        let made = body.basic_blocks.iter_enumerated().find_map(|(block, data)| {
            matches!(&data.terminator().kind, TerminatorKind::Call { func, destination, .. }
                if destination.local == it && destination.projection.is_empty()
                    && fn_def(func.ty(&body.local_decls, self.tcx)).is_some_and(|(def_id, _)| is_slice_iter(self.tcx, def_id)))
            .then_some(block)
        });
        // Its write and the borrow alone.
        struct Uses(Local, usize);
        impl<'tcx> Visitor<'tcx> for Uses {
            fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
                if local == self.0 && !matches!(context, PlaceContext::NonUse(_)) {
                    self.1 += 1;
                }
            }
        }
        let mut uses = Uses(it, 0);
        uses.visit_body(body);
        made.filter(|_| uses.1 == 2)
    }

    /// `made`, `$iter(items)` of a slice's `iter()` `FoundIndexes::items`
    /// says, the array its `position` searches: `items`.
    pub(super) fn found_index_items(&self, made: Expr, span: Span) -> R<Expr> {
        match crate::lower::iterators::fresh_iter_source(&made) {
            Some(items) => Ok(items.clone()),
            None => Err(self.unsupported(span, "this `iter()` of a slice a `position` searches, from its MIR")),
        }
    }

    /// `call`, `$position(it, f)` of a `position` `FoundIndexes::call`
    /// says, as JS's own search of the array `it` is, lent, which its
    /// `iter()` gave as it is (`found_index_items`): `items.indexOf(x)`
    /// where `f` is `(c) => c === x`, else `items.findIndex(f)`.
    pub(super) fn found_index(&self, call: Expr, span: Span) -> R<Expr> {
        let searched = match &call.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$position") => match &args[..] {
                [it, f] => Some((crate::lower::iterators::unlent(it).clone(), f.clone())),
                _ => None,
            },
            _ => None,
        };
        let Some((items, f)) = searched else {
            return Err(self.unsupported(span, "this `position` of a slice's items, from its MIR"));
        };
        Ok(match crate::lower::iterators::sought(&f) {
            Some(x) => Expr::call(Expr::member(items, "indexOf"), vec![x]),
            None => Expr::call(Expr::member(items, "findIndex"), vec![f]),
        })
    }
}

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
use rust_js_lower::combinators::Comb;
use rust_js_lower::recognition::{is_iterator_position, is_slice_items};

/// Of a body: each block whose call is a `position` `found_index` lowers,
/// and the `Option` each gives.
#[derive(Default)]
pub(super) struct FoundIndexes {
    calls: IndexSet<BasicBlock>,
    /// The block of each one's slice's `iter()`, the array itself.
    items: IndexSet<BasicBlock>,
    locals: IndexSet<Local>,
    /// Each block whose call is an `Option`'s `and_then` whose closure
    /// gives such a `position` (`gives_a_position`), its `Option` only
    /// tested and its index taken too: `-1` its `None` (#2074).
    through: IndexSet<BasicBlock>,
}

impl FoundIndexes {
    /// Whether the call that ends `block` is a slice's `position` whose
    /// `Option` is only tested and its index taken.
    pub(super) fn call(&self, block: BasicBlock) -> bool {
        self.calls.contains(&block)
    }

    /// Whether the call that ends `block` is an `and_then` whose closure
    /// gives a slice's `position`, its `Option` a found index: the
    /// closure's search JS's own, its `None` `-1` (#2074).
    pub(super) fn through(&self, block: BasicBlock) -> bool {
        self.through.contains(&block)
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

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
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
            // Asked of a `position` alone: it walks the whole body, which
            // of each call of a long one is its square (`a long function is
            // lowered block by block`).
            let fresh = || {
                (args.first().and_then(|receiver| receiver.node.place()))
                    .filter(|receiver| receiver.projection.is_empty())
                    .and_then(|receiver| self.fresh_slice_items(body, receiver.local))
            };
            if positioned
                && let Some(items) = fresh()
                && destination.projection.is_empty()
                && tested_and_taken(body, destination.local, 1)
            {
                found.calls.insert(block);
                found.items.insert(items);
                found.locals.insert(destination.local);
            }
            // Or an `Option`'s `and_then` whose closure gives one, `ext
            // .and_then(|e| DAYS.iter().position(|&d| d == e))`, its own
            // `Option` only tested and its index taken (#2074).
            let then_closure = (fn_def(func.ty(&body.local_decls, self.tcx)))
                .filter(|&(def_id, generic_args)| {
                    matches!(
                        self.recognition().classify(def_id, generic_args),
                        Some(Std::Comb(Comb::AndThen))
                    )
                })
                .and_then(|_| match args.get(1)?.node.ty(&body.local_decls, self.tcx).kind() {
                    ty::Closure(closure, _) => Some(*closure),
                    _ => None,
                });
            if let Some(closure) = then_closure
                && self.gives_a_position(closure)
                && destination.projection.is_empty()
                && tested_and_taken(body, destination.local, 1)
            {
                found.through.insert(block);
                found.locals.insert(destination.local);
            }
        }
        found
    }

    /// Whether the crate's closure `closure` gives a slice's `position`
    /// as it is: its return place written by one, of a slice's `iter()`
    /// made for it (`fresh_slice_items`), whose JS search
    /// `found_index_closure` makes of it.
    fn gives_a_position(&self, closure: rustc_span::def_id::DefId) -> bool {
        let Some(body) = (self.krate.closures.get(&closure))
            .and_then(|body| body.mir.as_ref())
            .map(|mir| &mir.body)
        else {
            return false;
        };
        body.basic_blocks.iter().any(|data| {
            matches!(&data.terminator().kind, TerminatorKind::Call { func, args, destination, .. }
                if destination.local == mir::RETURN_PLACE
                    && destination.projection.is_empty()
                    && fn_def(func.ty(&body.local_decls, self.tcx)).is_some_and(|(def_id, _)| is_iterator_position(self.tcx, def_id))
                    && (args.first().and_then(|receiver| receiver.node.place()))
                        .filter(|receiver| receiver.projection.is_empty())
                        .is_some_and(|receiver| self.fresh_slice_items(body, receiver.local).is_some()))
        })
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
                    && fn_def(func.ty(&body.local_decls, self.tcx)).is_some_and(|(def_id, _)| is_slice_items(self.tcx, def_id)))
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
        match rust_js_lower::iterators::fresh_iter_source(&made) {
            Some(items) => Ok(items.clone()),
            None => Err(self.unsupported(span, "this `iter()` of a slice a `position` searches, from its MIR")),
        }
    }
}

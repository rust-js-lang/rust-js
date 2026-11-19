//! What a body's statements may write, over-approximated where a walk
//! needs no more (#2112 B 3b): the walks of what callers give and of what
//! captures hold (`intervals::callers`, `captures`), which ask what may
//! survive a write as the lowering's own walk does, but whose answer only
//! narrows what a parameter is given. Everything that can be changed
//! through a pointer has escaped as the body starts: each local whose
//! address is taken where a write may come of it, each local a rewrite
//! made of a place behind a pointer (`made` on), and what each pointer
//! that may write points to. A direct write writes its local; any other,
//! a write through a pointer, a call, a drop, a yield or inline assembly,
//! writes everything escaped, whole. It fails closed: never less than the
//! precise query (`Effects::of`) says, often more.

use super::*;

impl Effects {
    /// `body`'s coarse effects (this module's).
    pub(in crate::lower) fn coarse<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        body: &Body<'tcx>,
        made: usize,
    ) -> Effects {
        let mut escaped: IndexMap<Target, Cap> = IndexMap::default();
        let mut of: IndexVec<Local, Targets> = IndexVec::from_elem_n(Targets::default(), body.local_decls.len());
        for (local, decl) in body.local_decls.iter_enumerated() {
            if local.as_usize() >= made {
                escaped.insert(Target::Whole(local), WHOLE);
            }
            if pointers_in(tcx, typing_env, decl.ty).writes {
                escaped.insert(Target::Pointee(local), WHOLE);
                of[local].unknown = true;
            }
        }
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let place = match &assign.1 {
                    Rvalue::Ref(_, mir::BorrowKind::Fake(_), _) => continue,
                    Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) => place,
                    Rvalue::Reborrow(_, mutability, place) if mutability.is_mut() => place,
                    Rvalue::Ref(_, _, place) | Rvalue::Reborrow(_, _, place)
                        if changes_through_shared(tcx, typing_env, place.ty(body, tcx).ty) =>
                    {
                        place
                    }
                    _ => continue,
                };
                escaped.insert(Target::Whole(place.local), WHOLE);
            }
        }
        let mut written: IndexMap<Location, (IndexMap<Target, Written>, bool)> = IndexMap::default();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                let location = Location { block, statement_index };
                let place = match &statement.kind {
                    StatementKind::Assign(assign) => Some(assign.0),
                    StatementKind::SetDiscriminant { place, .. } => Some(**place),
                    StatementKind::Intrinsic(_) => None,
                    _ => continue,
                };
                let at = match place {
                    Some(place) if !place.is_indirect() => (
                        IndexMap::from_iter([(Target::Whole(place.local), Written::Whole)]),
                        false,
                    ),
                    _ => (IndexMap::default(), true),
                };
                written.insert(location, at);
            }
            let location = body.terminator_loc(block);
            let at = match &data.terminator().kind {
                TerminatorKind::Call { destination, .. } => match destination.is_indirect() {
                    false => (
                        IndexMap::from_iter([(Target::Whole(destination.local), Written::Whole)]),
                        true,
                    ),
                    true => (IndexMap::default(), true),
                },
                TerminatorKind::Drop { .. } | TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } => {
                    (IndexMap::default(), true)
                }
                _ => continue,
            };
            written.insert(location, at);
        }
        // Escaped where the body starts, and so before each block: nothing
        // that's escaped comes back.
        let escaped_before = IndexVec::from_elem_n(escaped, body.basic_blocks.len());
        Effects {
            written,
            of,
            escaped_before,
            escapes: IndexMap::default(),
            cells: IndexSet::default(),
            fixed_length: IndexSet::default(),
            guarded: IndexMap::default(),
            stale: Default::default(),
        }
    }
}

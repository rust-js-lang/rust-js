//! What a test that a map holds a key shows, on MIR (rd558; #1672 M3,
//! step 5): where `m.contains_key(&k)` holds, `m[&k]` can't panic while
//! neither the map nor the key changes, as React's ReactMarkupServer reads
//! `stashedErrors.get(id)` after `stashedErrors.has(id)`. Nor where `m[&k]`
//! has given its value once; of a key whose lookup reads the same until
//! it's written (`key_stays`, #1844). The map and the key are each a local's root,
//! as a slice's is, a `RefCell`'s by the cell: what writes either, lends
//! the map `&mut`, or may write a cell forgets it (`State::forget`,
//! `forget_lengths`).

use super::*;
use crate::recognition::{MapKey, key_stays, map_key};

impl<'tcx> Walk<'_, 'tcx> {
    /// The map and the key a call that asks of a key is given, each a
    /// local's root, and what it asks: none of a map or a key lent.
    fn asked(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        state: &State,
    ) -> Option<(MapKey, (Local, Local))> {
        let (callee, callee_args) = func.const_fn_def()?;
        let asked = map_key(self.tcx, callee, callee_args)?;
        let local = |at: usize| {
            (args.get(at))
                .and_then(|arg| arg.node.place())
                .and_then(|p| p.as_local())
                .filter(|l| !self.lent.contains(l))
                .map(|l| state.root(l))
        };
        // A key whose lookup reads the same until it's written, and the
        // map's keys so (`key_stays`, #1844).
        let stays = |ty: Ty<'tcx>| key_stays(self.tcx, self.typing_env, ty);
        let map_ty = args.first().map(|a| a.node.ty(&*self.body, self.tcx).peel_refs());
        let key_ty = args.get(1).map(|a| a.node.ty(&*self.body, self.tcx).peel_refs());
        let held_keys = matches!(map_ty.map(|t| *t.kind()), Some(ty::Adt(_, a)) if a.types().next().is_some_and(stays));
        (held_keys && key_ty.is_some_and(stays)).then_some(())?;
        Some((asked, (local(0)?, local(1)?)))
    }

    /// What a call that asks of a key gives, `local`: of `contains_key`, a
    /// `bool` a switch of which shows the key held (`State::has_keys`); of
    /// `m[&k]`, given back, the key held past it, as it panics otherwise.
    pub(super) fn asked_key(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        local: Local,
        state: &State,
        after: &mut State,
    ) {
        match self.asked(func, args, state) {
            Some((MapKey::Has, held)) => {
                after.has_keys.insert(local, held);
            }
            Some((MapKey::Index, held)) => {
                after.keys.insert(held);
            }
            None => {}
        }
    }

    /// Whether a call is `m[&k]` of a key the map holds where it's called.
    pub(super) fn key_held(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        state: &State,
    ) -> bool {
        matches!(self.asked(func, args, state), Some((MapKey::Index, held)) if state.keys.contains(&held))
    }
}

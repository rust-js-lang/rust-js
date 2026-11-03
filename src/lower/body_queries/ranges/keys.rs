//! What a test that a map holds a key shows (rd558): where
//! `m.contains_key(k)` holds, `m[k]` can't panic while neither the map nor
//! `k` changes, so it's the map's `get(k)`, as React's ReactMarkupServer
//! reads `stashedErrors.get(id)` after `stashedErrors.has(id)`. The map is
//! found as a slice is, a `RefCell`'s by the cell (ADR 0881), and the key
//! is a variable, as it is or borrowed. What may change either forgets it,
//! as it does a slice's length: a variable set or bound again, a `&mut` of
//! it lent or kept, and of a `RefCell`'s, any call but one that only views
//! it, and a drop that may run code.

use super::known::Known;
use super::{Slice, Walk};
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{MapKey, map_key};
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId};
use rustc_middle::ty;

impl Walk<'_, '_> {
    /// Of a call `e` that asks a map of a key (`MapKey`), what it asks, its
    /// function, and the map and the key's variable.
    fn map_key_of(&self, e: ExprId) -> Option<(MapKey, ExprId, (Slice, LocalVarId))> {
        let thir = self.thir;
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
        let asked = map_key(self.tcx, def_id, generic_args)?;
        let map = self.slice(*args.first()?)?;
        let key = self.key_var(*args.get(1)?)?;
        Some((asked, fun, (map, key)))
    }

    /// The variable a key is, as it is or borrowed: `id` of `id`, `&id` or
    /// `&*id`.
    fn key_var(&self, mut e: ExprId) -> Option<LocalVarId> {
        let thir = self.thir;
        loop {
            e = strip(thir, e);
            match thir[e].kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Shared,
                    arg,
                } => e = arg,
                ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(..)) => e = arg,
                ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if !self.volatile.contains(&id) => {
                    return Some(id);
                }
                _ => return None,
            }
        }
    }

    /// The map and the key a test `m.contains_key(k)` shows it holds.
    pub(super) fn tested_key(&self, test: ExprId) -> Option<(Slice, LocalVarId)> {
        match self.map_key_of(test)? {
            (MapKey::Has, _, held) => Some(held),
            (MapKey::Index, ..) => None,
        }
    }

    /// A map's `m[k]`, called where `known` is: its `get(k)` where the map
    /// holds `k` every time it's reached, as an index in bounds is.
    pub(super) fn judge_key(&mut self, call: ExprId, known: &Option<Known>) {
        if let (Some((MapKey::Index, fun, held)), Some(k)) = (self.map_key_of(call), known) {
            *self.indexes.entry(fun).or_insert(true) &= k.keys.contains(&held);
        }
    }
}

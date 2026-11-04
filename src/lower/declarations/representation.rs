//! What a `&mut` and a `dyn` in a declared signature are, as lowering has
//! them (#619, #620, ADR 1077, amended): a box, `{ value: T }`, or the
//! object; a pair, `{ value: unknown; impl: Shape<unknown> }`, or `any`.

use rustc_middle::ty::Ty;
use serde_json::Value;

use crate::lower::representation::MutRefForm;

use super::{Declarations, keyword};

impl<'tcx> Declarations<'_, 'tcx> {
    /// What a `&mut` to `pointee` in the signature being declared is, as
    /// lowering has it (ADRs 0074, 0099, 1086, #619): a cell, a box or a
    /// handle, of a value JS can't change in place or a type parameter, or
    /// the object.
    pub(super) fn mut_ref_form(&self, pointee: Ty<'tcx>) -> MutRefForm {
        (self.representation.mut_ref)(self.owner, pointee)
    }

    /// A `dyn` that's no `Fn`'s and no type of TypeScript's, `ty`: of a
    /// trait whose dictionaries are declared, its pair, its value and its
    /// impl's dictionary, `{ value: unknown; impl: Shape<unknown> }`, of a
    /// value of some type and that type's dictionary (ADR 0049, #620); of
    /// another trait `any`, as what has no type here is.
    pub(super) fn dyn_value(&mut self, ty: Ty<'tcx>) -> Value {
        match (self.representation.pair_trait)(self.owner, ty) {
            Some(t) if self.declares_dictionary(t) => self.pair(t),
            _ => keyword("any"),
        }
    }
}

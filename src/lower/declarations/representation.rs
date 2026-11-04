//! What a `&mut` and a `dyn` in a declared signature are, as lowering has
//! them (#619, #620, ADR 1077, amended): a box, `{ value: T }`, or the
//! object; a pair, `{ value: unknown; impl: Shape<unknown> }`, or `any`.

use rustc_middle::ty::Ty;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, keyword};

/// What lowering's representation makes of a type in an item's signature,
/// asked of a context of that item, as its body is lowered in, so a
/// declaration says what its JS is (#619, #620, ADR 1077, amended): one
/// answer, lowering's own.
pub(in crate::lower) struct Representation<'a, 'tcx> {
    /// Whether a `&mut` to the type, in the item's signature, is a cell,
    /// a box or a handle, `{ value }`, whose `value` is its place:
    /// `is_cell_pointee`'s answer, as `param_is_box`'s of a parameter
    /// (ADRs 0074, 0099).
    pub(in crate::lower) cell_pointee: &'a dyn Fn(DefId, Ty<'tcx>) -> bool,
    /// The trait a `dyn` of the type is a pair of, `{ value, impl }`, its
    /// value and its impl's dictionary: `dynamic_trait`'s answer (ADR 0049).
    pub(in crate::lower) pair_trait: &'a dyn Fn(DefId, Ty<'tcx>) -> Option<DefId>,
}

/// `{ value: T }`: what holds a `value` of `ty`, a cell or a box (ADRs
/// 0023, 0074), as its JS reads it, `js::HELD`.
pub(super) fn held(ty: Value) -> Value {
    let value =
        json!({ "kind": "property", "name": crate::js::HELD, "optional": false, "readonly": false, "type": ty });
    json!({ "kind": "object", "members": [value] })
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// Whether a `&mut` to `pointee` in the signature being declared is a
    /// cell, a box or a handle, as lowering has it: a value JS can't change
    /// in place, or a type parameter (ADRs 0074, 0099, #619). One to an
    /// object is the object.
    pub(super) fn is_cell(&self, pointee: Ty<'tcx>) -> bool {
        (self.representation.cell_pointee)(self.owner, pointee)
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

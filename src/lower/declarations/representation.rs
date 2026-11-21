//! What a `&mut` and a `dyn` in a declared signature are, as lowering has
//! them (#619, #620, ADR 1077, amended): a box, `{ value: T }`, or the
//! object; a pair, `{ value: unknown; impl: Shape<unknown> }`, or `any`.

use rustc_middle::ty::Ty;
use serde_json::Value;

use crate::lower::representation::MutRefForm;

use super::{Declarations, keyword, reference, std_types};

impl<'tcx> Declarations<'_, 'tcx> {
    /// An `Option<T>`: its `Some`, `some_ts`, or `None`, JS's `null` too, as
    /// rust-js reads it `!= null` (ADR 0030), one union, `number | SomeNone
    /// | null | undefined`.
    pub(super) fn option_ts(&mut self, payload: Ty<'tcx>) -> Value {
        let some = self.some_ts(payload);
        let mut types = match some["kind"].as_str() {
            Some("union") => some["types"].as_array().cloned().unwrap_or_default(),
            _ => vec![some],
        };
        types.extend([keyword("null"), keyword("undefined")]);
        serde_json::json!({ "kind": "union", "types": types })
    }

    /// What an `Option<T>`'s `Some` is in JS, `T` as the `Option` holds it:
    /// `T` itself; or, of a payload lowering boxes, as it may look like
    /// `None` (`boxed_payload`, ADR 0051), what of `T` isn't `None`-like,
    /// and `@rust-js/std`'s `SomeNone`, the box of what is: `Option<()>`'s
    /// `SomeNone`, `Option<Option<number>>`'s `number | SomeNone` (#842).
    /// Not of a payload of a type parameter or a projection, `Option<T>`,
    /// `Option<R::Event>`, whose box is there only once what it's made of
    /// looks like `None`, which the declaration, generic, can't tell: `T`.
    pub(super) fn some_ts(&mut self, payload: Ty<'tcx>) -> Value {
        let value = self.ts(payload);
        let generic = crate::lower::recognition::known_by_bounds(crate::lower::representation::payload_through(
            self.tcx, payload,
        ));
        // `any` and `unknown` take the box already (#1850).
        let top = matches!(value["kind"].as_str(), Some("keyword"))
            && matches!(value["keyword"].as_str(), Some("any" | "unknown"));
        if generic || top || !(self.representation.boxed)(self.owner, payload) {
            return value;
        }
        let nullish = |v: &Value| {
            matches!(v["kind"].as_str(), Some("keyword")) && matches!(v["keyword"].as_str(), Some("null" | "undefined"))
        };
        let mut types: Vec<Value> = match value["kind"].as_str() {
            Some("union") => (value["types"].as_array().into_iter().flatten())
                .filter(|v| !nullish(v))
                .cloned()
                .collect(),
            _ if nullish(&value) => Vec::new(),
            _ => vec![value],
        };
        let some_none = self.imported_from(Some(std_types::PACKAGE), std_types::SOME_NONE, None);
        types.push(reference(&some_none, Vec::new()));
        match types.len() {
            1 => types.remove(0),
            _ => serde_json::json!({ "kind": "union", "types": types }),
        }
    }

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

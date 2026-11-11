//! An enum's declaration, of its variants' literals and constants, and a
//! union's (ADRs 0089, 0186). Split from declarations.rs by concept, as a
//! file is at most 1500 lines.

use super::{Declarations, documented, keyword, union};
use crate::lower::bindings::{declared_tag, field_key, is_otherwise, is_untagged, tag_key, variant_name};
use rustc_middle::ty;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

impl<'tcx> Declarations<'_, 'tcx> {
    pub(super) fn enumeration(&mut self, def_id: DefId) -> Value {
        let adt = self.tcx.adt_def(def_id);
        let untagged = is_untagged(self.tcx, def_id);
        let union = match untagged {
            // `export type Size = string | number;` of an untagged enum, its
            // payloads' union (ADR 0214).
            true => {
                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
                let types: Vec<Value> = (adt.variants().iter())
                    .map(|v| match v.fields.iter().next() {
                        Some(field) => self.ts(field.ty(self.tcx, args).skip_normalization()),
                        // One without fields, its name's string, or `null` or
                        // `undefined` (ADR 0676).
                        None => match crate::lower::untagged::unit_value(self.tcx, v).map(|e| e.kind) {
                            Some(crate::js::ExprKind::Null) => keyword("null"),
                            Some(crate::js::ExprKind::Undefined) => keyword("undefined"),
                            _ => json!({ "kind": "literal", "value": variant_name(self.tcx, v) }),
                        },
                    })
                    .collect();
                union(types)
            }
            // `{ TAG: "Circle"; _0: number } | "Empty"`, each variant as rust-js
            // makes it (ADR 0033), as ReScript's genType declares one, one of
            // no fields its name; of a discriminated union, each an object of
            // its tag, one of no fields too (ADR 0284).
            false => {
                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
                let key = tag_key(self.tcx, def_id);
                let tagged = declared_tag(self.tcx, def_id).is_some();
                // A keyed enum's variant is an object of its keys alone,
                // `{}` of one without fields (#504).
                let keyed = self.flattened.keyed.contains(&def_id);
                let types: Vec<Value> = (adt.variants().iter())
                    .map(|v| {
                        // An `otherwise`, the object it holds (ADR 0284).
                        if tagged
                            && is_otherwise(self.tcx, v.def_id)
                            && let Some(field) = v.fields.iter().next()
                        {
                            return self.ts(field.ty(self.tcx, args).skip_normalization());
                        }
                        let name = self.variant_literal(v);
                        if v.fields.is_empty() && !tagged && !keyed {
                            return name;
                        }
                        let tag = (!keyed).then(|| json!({ "kind": "property", "name": key, "optional": false, "readonly": false, "type": name }));
                        let fields = v.fields.iter().enumerate().map(|(i, field)| {
                            let ty = field.ty(self.tcx, args).skip_normalization();
                            let outer = self.function_names.len();
                            self.fill_names(field.did, ty);
                            let ty = self.field_ts(ty, Some(field));
                            self.function_names.truncate(outer);
                            let name = crate::lower::representation::variant_field(self.tcx, v, i);
                            json!({ "kind": "property", "name": name, "optional": false, "readonly": false, "type": ty })
                        });
                        json!({ "kind": "object", "members": tag.into_iter().chain(fields).collect::<Vec<_>>() })
                    })
                    .collect();
                union(types)
            }
        };
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": union,
        })
    }

    /// A variant's value as its literal type, `"Empty"`, `0` or `true`.
    pub(super) fn variant_literal(&self, variant: &ty::VariantDef) -> Value {
        let value = match crate::lower::bindings::variant_tag(self.tcx, variant).kind {
            crate::js::ExprKind::Bool(b) => json!(b),
            crate::js::ExprKind::Num(n) => json!(n as i64),
            _ => json!(variant_name(self.tcx, variant)),
        };
        json!({ "kind": "literal", "value": value })
    }

    /// Each variant of a numbered enum, its module's `const`, `export const
    /// Pending: 0;`, as its JS declares it (ADR 0722, amended).
    pub(super) fn variant_constants(&self, def_id: DefId) -> Vec<Value> {
        let adt = self.tcx.adt_def(def_id);
        if !crate::lower::bindings::numbered(self.tcx, adt) {
            return Vec::new();
        }
        (adt.variants().iter())
            .map(|v| {
                let declared = json!({
                    "kind": "const",
                    "name": self.js_name(v.def_id),
                    "exported": true,
                    "declare": false,
                    "type": self.variant_literal(v),
                });
                documented(declared, self.tcx, v.def_id)
            })
            .collect()
    }

    /// `export type Number = { int: number } | { float: number };` of a
    /// union, the object of the one field it holds (ADR 0406).
    pub(super) fn union_type(&mut self, def_id: DefId) -> Value {
        let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
        let fields: Vec<_> = self.tcx.adt_def(def_id).non_enum_variant().fields.iter().collect();
        let types: Vec<Value> = (fields.into_iter())
            .map(|field| {
                let ty = self.field_ts(field.ty(self.tcx, args).skip_normalization(), None);
                let member = json!({ "kind": "property", "name": field_key(self.tcx, field).text(), "optional": false, "readonly": false, "type": ty });
                json!({ "kind": "object", "members": [member] })
            })
            .collect();
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": union(types),
        })
    }
}

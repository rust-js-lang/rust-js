//! A struct's declaration, `export interface TagProps { variant: RouteTag;
//! text?: string; }` (ADR 0196): its fields, each by the name its JS has,
//! an `Option`'s optional; a `#[repr(transparent)]` one its field's type
//! (ADR 0584). Split by concept from declarations.rs at its cap.

use rustc_hir as hir;
use rustc_middle::ty;
use rustc_span::def_id::{DefId, LocalModId};
use serde_json::{Value, json};

use crate::lower::bindings::{field_default, field_key, is_flatten, is_rest};

use super::comments::documented;
use super::parts::flattened_keys;
use super::{Declarations, is_any, keyword, or_undefined, reference};

impl<'tcx> Declarations<'_, 'tcx> {
    /// `export interface TagProps { variant: RouteTag; text?: string; }`
    pub(super) fn structure(&mut self, def_id: DefId) -> Option<Value> {
        let adt = self.tcx.adt_def(def_id);
        let variant = adt.non_enum_variant();
        // A `#[repr(transparent)]` struct is its field (ADR 0584): a type of
        // that field's, `type Meters = number`.
        if let Some(at) = crate::lower::recognition::transparent_field(self.tcx, adt) {
            let field = self
                .tcx
                .type_of(variant.fields.raw[at].did)
                .instantiate_identity()
                .skip_normalization();
            return Some(json!({
                "kind": "type",
                "name": super::type_name(self.tcx, def_id),
                "exported": true,
                "declare": false,
                "typeParameters": self.generics(def_id),
                "type": self.ts(field),
            }));
        }
        if variant.ctor.is_some() {
            return None;
        }
        // A flattened field's struct is what this one extends (ADR 0204):
        // one TypeScript can't see, another module's (`any`), is what a
        // `Rest` is. A name both have is this one's, which TypeScript's
        // `Omit` says (ADR 0205).
        // A keyed enum's, an untagged enum's of records, or a type
        // parameter's, which an interface can't extend: what it's of
        // besides, `{ flags: number } & K` (#504, #1818).
        let flattened = variant.fields.iter().find(|f| is_flatten(self.tcx, f));
        let intersected = flattened
            .map(|field| self.tcx.type_of(field.did).instantiate_identity().skip_normalization())
            .filter(|ty| match ty.kind() {
                ty::Param(_) => true,
                ty::Adt(..) => self.flattened.enum_of_keys(*ty),
                _ => false,
            })
            .map(|ty| self.ts(ty));
        let extends: Vec<Value> = match flattened.filter(|_| intersected.is_none()) {
            Some(field) => {
                let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
                let own: Vec<String> = (variant.fields.iter())
                    .filter(|f| f.did != field.did)
                    .filter_map(|f| field_key(self.tcx, f).name().map(str::to_owned))
                    .collect();
                let shadowed: Vec<Value> = (flattened_keys(self.tcx, ty).into_iter())
                    .filter(|key| own.contains(key))
                    .map(|key| json!({ "kind": "literal", "value": key }))
                    .collect();
                let inner = self.ts(ty);
                match (is_any(&inner), shadowed.len()) {
                    (true, _) => Vec::new(),
                    (false, 0) => vec![inner],
                    (false, _) => vec![reference(
                        "Omit",
                        vec![inner, json!({ "kind": "union", "types": shadowed })],
                    )],
                }
            }
            None => Vec::new(),
        };
        let mut members = Vec::new();
        for field in &variant.fields {
            let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
            // What a JS caller gives besides, `...rest` (ADR 0195).
            if is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty() && intersected.is_none()) {
                members.push(json!({
                    "kind": "index",
                    "parameter": "prop",
                    "key": keyword("string"),
                    "type": keyword("unknown"),
                }));
                continue;
            }
            if is_flatten(self.tcx, field) {
                continue;
            }
            // A field with a default, JS's where it's missing, is one a caller
            // may leave out (ADR 0212); one of an `Option` too, and one marked
            // `#[rust_js::nullable]` may be `null`, which rust-js reads as
            // `None` (ADR 0030), where TypeScript's data has it.
            // As written, where it's an alias of one of TypeScript's.
            let written = (field.did.as_local()).and_then(|local| match self.tcx.hir_node_by_def_id(local) {
                hir::Node::Field(field) => Some(field.ty),
                _ => None,
            });
            // A nullable `Cell`'s, the `Option` it holds, the field (ADR 0772);
            // a `OnceCell`'s, absent until it's set (ADR 0847).
            // A nested one's, the `Option` it holds, `field?: T | null` (#470).
            let nested = crate::lower::bindings::holds_nested(self.tcx, field);
            // One of a value that may be `undefined`, which is a `Some`
            // held as it is, is there, `T | null` (rd645).
            let value = crate::lower::bindings::holds_value(self.tcx, field);
            // How it holds a `None`, the owner's answer (#819): its
            // declaration's, and of the type it's declared, which a
            // nullable one is there of (ADR 1072, amended).
            let nullable = crate::lower::representation::none_form(
                self.tcx,
                crate::lower::representation::NonePlace::Field(field, None),
            )
            .stores_null();
            let declared = crate::lower::representation::none_form(
                self.tcx,
                crate::lower::representation::NonePlace::Field(field, Some(ty)),
            );
            let option = match nullable {
                true => crate::lower::bindings::nullable_option(self.tcx, ty).and_then(|option| self.option(option)),
                false if nested => self.option(ty).and_then(|option| self.option(option)),
                // A plain one's too, of a cell that's the property it holds,
                // `byteSize?: number | undefined` (rd415).
                false => (self.option(ty).or_else(|| self.once_cell_item(ty)))
                    .or_else(|| self.cell_item(ty, Some(field)).and_then(|item| self.option(item))),
            };
            let outer = self.function_names.len();
            self.fill_names(field.did, ty);
            let (optional, ty) = match option {
                Some(inner) => {
                    let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                    // Nullable, or a plain one a `js::null()` is written
                    // to, which may hold `null` (ADR 0935, amended).
                    // A nullable one is there, whoever makes its struct
                    // (ADR 1072, amended).
                    let present = matches!(
                        declared,
                        crate::lower::representation::NoneForm::Null | crate::lower::representation::NoneForm::Value
                    );
                    match nullable || nested || value || self.null_fields.contains(&field.did) {
                        true => (!present, json!({ "kind": "union", "types": [inner, keyword("null")] })),
                        false => (true, inner),
                    }
                }
                None => (
                    field_default(self.tcx, field).is_some(),
                    self.as_written(written, |out| out.field_ts(ty, Some(field))),
                ),
            };
            self.function_names.truncate(outer);
            // One a caller may leave out takes `undefined` too, as leaving it
            // out does, under `exactOptionalPropertyTypes` as without it.
            let ty = if optional { or_undefined(ty) } else { ty };
            let property = json!({
                "kind": "property",
                "name": self.member_name(field),
                "optional": optional,
                "readonly": false,
                "type": ty,
            });
            members.push(documented(property, self.tcx, field.did));
        }
        // An object of methods' own, `containsNode(node: unknown): boolean`,
        // each that's `pub` (ADR 0943).
        for method in crate::lower::classes::own_methods(self.tcx, def_id) {
            if !self.tcx.visibility(method).is_public() {
                continue;
            }
            let name = format!("{}.{}", self.tcx.item_name(def_id), self.js_name(method));
            if self.leave_out(method, &name) {
                continue;
            }
            let function = self.function(method);
            let declared = json!({
                "kind": "method",
                "name": crate::lower::classes::property(self.tcx, method),
                "optional": false,
                "typeParameters": function["typeParameters"],
                "params": function["params"],
                "returns": function["returns"],
            });
            members.push(documented(declared, self.tcx, method));
        }
        // A method its impls bind of the JS object it is, `isPersistent():
        // boolean`, where no field has its name (rd144).
        for (method, name) in crate::lower::bindings::bound_methods(self.tcx, def_id) {
            if members.iter().any(|m| m["name"].as_str() == Some(name.as_str())) {
                continue;
            }
            let function = self.function(method);
            let declared = json!({
                "kind": "method",
                "name": name,
                "optional": false,
                "typeParameters": function["typeParameters"],
                "params": function["params"],
                "returns": function["returns"],
            });
            members.push(documented(declared, self.tcx, method));
        }
        if let Some(flattened) = intersected {
            return Some(json!({
                "kind": "type",
                "name": super::type_name(self.tcx, def_id),
                "exported": true,
                "declare": false,
                "typeParameters": self.generics(def_id),
                "type": { "kind": "intersection", "types": [{ "kind": "object", "members": members }, flattened] },
            }));
        }
        Some(json!({
            "kind": "interface",
            "name": super::type_name(self.tcx, def_id),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "extends": extends,
            "members": members,
        }))
    }

    /// A field's name as its declaration has it: its text, or the key a
    /// symbol names it by, `[Symbol.asyncIterator]` of JS's, and
    /// `[RESPONSE_SYMBOL]` of this module's own, declared a `unique symbol`,
    /// as TypeScript takes a computed key of no other (ADR 1051).
    fn member_name(&mut self, field: &ty::FieldDef) -> Value {
        let key = field_key(self.tcx, field);
        let Some((item, span)) = key.symbol_item() else {
            return json!(key.text());
        };
        if self.tcx.is_foreign_item(item) {
            return json!({ "computed": crate::lower::bindings::js_name(self.tcx, item) });
        }
        if let Some(local) = item.as_local()
            && LocalModId::new_unchecked(self.tcx.parent_module_from_def_id(local).to_local_def_id()) == self.module
        {
            let name = self.js_name(item);
            if !self.tcx.visibility(item).is_public() && !self.unexported.contains_key(&name) {
                let declared = json!({ "kind": "const", "name": name, "exported": false, "declare": true, "type": { "kind": "unique-symbol" } });
                self.unexported.insert(name.clone(), Some(declared));
            }
            return json!({ "computed": name });
        }
        self.tcx.dcx().span_err(
            span,
            "rust-js: a declared struct's field is named by another module's symbol, which its `.d.ts` can't name yet: name it by a symbol its own module holds, or JS's own, `js::symbol::async_iterator` (ADR 1051)",
        );
        json!(self.tcx.def_path_str(item))
    }
}

//! A struct's declaration, `export interface TagProps { variant: RouteTag;
//! text?: string; }` (ADR 0196): its fields, each by the name its JS has,
//! an `Option`'s optional; a `#[repr(transparent)]` one its field's type
//! (ADR 0584); a tuple struct its tuple, `[number, number]`; and one no
//! other crate can make opaque, `interface Email { readonly [$emailBrand]:
//! never }` (ADR 1187). Split by concept from declarations.rs at its cap.

use rustc_hir as hir;
use rustc_middle::ty;
use rustc_span::def_id::{DefId, LocalModId};
use serde_json::{Value, json};

use crate::bindings::{field_default, field_key, is_flatten, is_rest};

use super::comments::documented;
use super::parts::flattened_keys;
use super::{Declarations, is_any, keyword, or_undefined, reference};

impl<'tcx> Declarations<'_, 'tcx> {
    /// A struct's declaration where it's declared, exported or its module's
    /// own: opaque where no other crate can make it (`sealed`), else its
    /// `structure`. A class's and a global's fields are `structure`'s, as JS
    /// makes those.
    pub(super) fn struct_declaration(&mut self, def_id: DefId) -> Option<Value> {
        match sealed(self.tcx, def_id) {
            true => Some(self.opaque(def_id)),
            false => self.structure(def_id),
        }
    }

    /// A struct no other crate can make, `pub struct Email(String)`, as
    /// TypeScript is told of it: an interface branded by a `unique symbol` of
    /// its module's, as std's opaque types are (ADR 1149), which TypeScript
    /// takes only of what rust-js's JS gave, `readonly [$idBrand]: T` of its
    /// type parameters, its `pub` fields and methods; not its other fields,
    /// which no other crate reads, nor its JS, which is no API (ADR 1187).
    fn opaque(&mut self, def_id: DefId) -> Value {
        let name = super::type_name(self.tcx, def_id);
        // `$emailBrand`: a name no Rust item's JS name can take, as none has
        // a `$`, so no export of the module meets it.
        let mut brand = format!("${name}Brand");
        brand[1..2].make_ascii_lowercase();
        if !self.unexported.contains_key(&brand) {
            let declared = json!({ "kind": "const", "name": brand, "exported": false, "declare": true, "type": { "kind": "unique-symbol" } });
            self.unexported.insert(brand.clone(), Some(declared));
        }
        let generics = self.generics(def_id);
        let mut held: Vec<Value> = (generics.iter())
            .map(|param| json!({ "kind": "reference", "name": param["name"], "args": [] }))
            .collect();
        let held = match held.len() {
            0 => keyword("never"),
            1 => held.remove(0),
            _ => json!({ "kind": "tuple", "elements": held }),
        };
        let mut members = vec![json!({
            "kind": "property",
            "name": { "computed": brand },
            "optional": false,
            "readonly": true,
            "type": held,
        })];
        // Its `pub` fields, which another crate reads and writes: a named
        // one's as `structure` declares it, with its methods; a tuple
        // struct's by its index, `"0": number`.
        let variant = self.tcx.adt_def(def_id).non_enum_variant();
        if let Some(at) = crate::recognition::transparent_field(self.tcx, self.tcx.adt_def(def_id)) {
            // A `#[repr(transparent)]` one's JS is its field (ADR 0584): one
            // whose field is `pub`, which another crate reads, is that
            // field's type, branded, `string & { readonly [$taggedBrand]: T }`.
            let field = &variant.fields.raw[at];
            if field.vis.is_public() {
                let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
                return json!({
                    "kind": "type",
                    "name": name,
                    "exported": true,
                    "declare": false,
                    "typeParameters": generics,
                    "type": { "kind": "intersection", "types": [self.ts(ty), { "kind": "object", "members": members }] },
                });
            }
            self.methods(def_id, &mut members);
        } else if variant.ctor.is_some() {
            for (index, field) in variant.fields.iter().enumerate() {
                if field.vis.is_public() {
                    let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
                    let property = json!({ "kind": "property", "name": index.to_string(), "optional": false, "readonly": false, "type": self.ts(ty) });
                    members.push(documented(property, self.tcx, field.did));
                }
            }
            self.methods(def_id, &mut members);
        } else {
            let unseen: Vec<Value> = (variant.fields.iter())
                .filter(|field| !field.vis.is_public())
                .map(|field| self.member_name(field))
                .collect();
            match self.structure(def_id) {
                Some(declared) if declared["kind"] == "interface" => members.extend(
                    (declared["members"].as_array().into_iter().flatten())
                        .filter(|member| !(member["kind"] == "property" && unseen.contains(&member["name"])))
                        .cloned(),
                ),
                _ => self.methods(def_id, &mut members),
            }
        }
        documented(
            json!({
                "kind": "interface",
                "name": name,
                "exported": true,
                "declare": false,
                "typeParameters": generics,
                "extends": [],
                "members": members,
            }),
            self.tcx,
            def_id,
        )
    }

    /// `export interface TagProps { variant: RouteTag; text?: string; }`
    pub(super) fn structure(&mut self, def_id: DefId) -> Option<Value> {
        let adt = self.tcx.adt_def(def_id);
        let variant = adt.non_enum_variant();
        // A `#[repr(transparent)]` struct is its field (ADR 0584): a type of
        // that field's, `type Meters = number`.
        if let Some(at) = crate::recognition::transparent_field(self.tcx, adt) {
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
        // A unit struct named as a string, `#[rust_js::name = "trigger-press"]`,
        // is that string, as its value is (ADR 0013, ADR 1164).
        if let Some(value) = super::named_unit(self.tcx, def_id) {
            return Some(json!({
                "kind": "type",
                "name": super::type_name(self.tcx, def_id),
                "exported": true,
                "declare": false,
                "typeParameters": [],
                "type": { "kind": "literal", "value": value.literal() },
            }));
        }
        // A tuple struct is its array (ADR 0020): its tuple, `type Point =
        // [number, number]`; not one of no value, a unit struct or a handle
        // of `PhantomData`, nor one whose `#[rust_js::types]` names JS's.
        if variant.ctor.is_some() {
            if !holds_a_value(self.tcx, def_id) || super::written_types(self.tcx, def_id).is_some() {
                return None;
            }
            let elements: Vec<Value> = (variant.fields.iter())
                .map(|field| self.ts(self.tcx.type_of(field.did).instantiate_identity().skip_normalization()))
                .collect();
            return Some(json!({
                "kind": "type",
                "name": super::type_name(self.tcx, def_id),
                "exported": true,
                "declare": false,
                "typeParameters": self.generics(def_id),
                "type": { "kind": "tuple", "elements": elements },
            }));
        }
        // A flattened field's struct is what this one extends (ADR 0204):
        // one TypeScript can't see, another module's (`any`), is what a
        // `Rest` is. A name both have is this one's, which TypeScript's
        // `Omit` says (ADR 0205).
        // A keyed enum's, an untagged enum's of records, or a type
        // parameter's, which an interface can't extend: what it's of
        // besides, `{ flags: number } & K` (#504, #1818).
        let flattened = variant.fields.iter().find(|f| is_flatten(self.tcx, f));
        // A `js::Spread<&G>`'s, a copy of `G`'s keys but those the struct
        // writes after it, `Omit<G, "source">`, which Rust reads only as
        // that says (ADR 1164).
        let own_keys: Vec<Value> = (variant.fields.iter())
            .filter(|field| !is_flatten(self.tcx, field))
            .filter_map(|field| {
                field_key(self.tcx, field)
                    .name()
                    .map(|key| json!({ "kind": "literal", "value": key }))
            })
            .collect();
        let intersected = flattened
            .map(|field| self.tcx.type_of(field.did).instantiate_identity().skip_normalization())
            .filter(|ty| match ty.kind() {
                ty::Param(_) => true,
                ty::Adt(adt, _) => {
                    crate::recognition::is_js_spread(self.tcx, adt.did()) || self.flattened.enum_of_keys(*ty)
                }
                _ => false,
            })
            .map(|ty| match ty.kind() {
                ty::Adt(adt, args) if crate::recognition::is_js_spread(self.tcx, adt.did()) => {
                    let copied = self.ts(args.type_at(0).peel_refs());
                    let omitted = match own_keys.len() {
                        0 => keyword("never"),
                        1 => own_keys[0].clone(),
                        _ => json!({ "kind": "union", "types": own_keys }),
                    };
                    json!({ "kind": "reference", "name": "Omit", "args": [copied, omitted] })
                }
                _ => self.ts(ty),
            });
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
            // A `js::Accessor<T>`'s, its getter's value, which nothing sets,
            // `readonly canceled: boolean` (ADR 1164).
            if let ty::Adt(adt, args) = ty.kind()
                && crate::recognition::is_js_accessor(self.tcx, adt.did())
            {
                let property = json!({
                    "kind": "property",
                    "name": self.member_name(field),
                    "optional": false,
                    "readonly": true,
                    "type": self.ts(args.type_at(0)),
                });
                members.push(documented(property, self.tcx, field.did));
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
            let nested = crate::bindings::holds_nested(self.tcx, field);
            // One of a value that may be `undefined`, which is a `Some`
            // held as it is, is there, `T | null` (rd645).
            let value = crate::bindings::holds_value(self.tcx, field);
            // How it holds a `None`, the owner's answer (#819): its
            // declaration's, and of the type it's declared, which a
            // nullable one is there of (ADR 1072, amended).
            let nullable =
                crate::representation::none_form(self.tcx, crate::representation::NonePlace::Field(field, None))
                    .stores_null();
            let declared =
                crate::representation::none_form(self.tcx, crate::representation::NonePlace::Field(field, Some(ty)));
            let option = match nullable {
                true => crate::bindings::nullable_option(self.tcx, ty).and_then(|option| self.option(option)),
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
                    // Its `Some` as the field holds it: the value itself of
                    // one that holds it so (rd645), else as an `Option`
                    // holds it, ADR 0051's box of what looks like `None`
                    // (#842).
                    let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| match value {
                        true => out.ts(inner),
                        false => out.some_ts(inner),
                    });
                    // Nullable, or a plain one a `js::null()` is written
                    // to, which may hold `null` (ADR 0935, amended).
                    // A nullable one is there, whoever makes its struct
                    // (ADR 1072, amended).
                    let present = matches!(
                        declared,
                        crate::representation::NoneForm::Null | crate::representation::NoneForm::Value
                    );
                    match nullable || nested || value || self.null_fields.contains(&field.did) {
                        true => (!present, with_null(inner)),
                        // A present slot's key is there, `T | undefined` (ADR 1161).
                        false if crate::bindings::is_present(self.tcx, field) => (false, or_undefined(inner)),
                        false => (true, inner),
                    }
                }
                // One always `undefined`, a `()`'s, its key there or not
                // (#1851).
                None => (
                    field_default(self.tcx, field).is_some() || crate::representation::always_undefined(self.tcx, ty),
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
        self.methods(def_id, &mut members);
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
        let generics = self.generics(def_id);
        // Of each of a union's members, `#[rust_js::distributive]`: `R extends
        // unknown ? { .. } : never`, so checking `reason` narrows `event`, as
        // upstream's `ChangeDetails<Reason>` is (ADR 1164).
        if is_distributive(self.tcx, def_id) {
            let [parameter] = &generics[..] else {
                self.tcx.dcx().span_err(
                    self.tcx.def_span(def_id),
                    "rust-js: a `#[rust_js::distributive]` struct has one type parameter, the union its declaration is of each member of",
                );
                return None;
            };
            if !extends.is_empty() {
                self.tcx.dcx().span_err(
                    self.tcx.def_span(def_id),
                    "rust-js: a `#[rust_js::distributive]` struct flattens no struct, which its declaration would extend",
                );
                return None;
            }
            let check = json!({ "kind": "reference", "name": parameter["name"], "args": [] });
            return Some(json!({
                "kind": "type",
                "name": super::type_name(self.tcx, def_id),
                "exported": true,
                "declare": false,
                "typeParameters": generics,
                "type": {
                    "kind": "conditional",
                    "check": check,
                    "extends": keyword("unknown"),
                    "whenTrue": { "kind": "object", "members": members },
                    "whenFalse": keyword("never"),
                },
            }));
        }
        Some(json!({
            "kind": "interface",
            "name": super::type_name(self.tcx, def_id),
            "exported": true,
            "declare": false,
            "typeParameters": generics,
            "extends": extends,
            "members": members,
        }))
    }

    /// Its methods JS calls on it: its own, `containsNode(node: unknown):
    /// boolean`, each that's `pub` (ADR 0943), and those its impls bind of
    /// the JS object it is, `isPersistent(): boolean`, where no member has
    /// its name (rd144).
    fn methods(&mut self, def_id: DefId, members: &mut Vec<Value>) {
        for method in crate::classes::own_methods(self.tcx, def_id) {
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
                "name": crate::classes::property(self.tcx, method),
                "optional": false,
                "typeParameters": function["typeParameters"],
                "params": function["params"],
                "returns": function["returns"],
            });
            members.push(documented(declared, self.tcx, method));
        }
        for (method, name) in crate::bindings::bound_methods(self.tcx, def_id) {
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
            return json!({ "computed": crate::bindings::js_name(self.tcx, item) });
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

/// Whether `def_id` is marked `#[rust_js::distributive]`, declared of each of
/// its type parameter's union's members (ADR 1164).
fn is_distributive(tcx: rustc_middle::ty::TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [
        rustc_span::Symbol::intern("rust_js"),
        rustc_span::Symbol::intern("distributive"),
    ];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// `ty | null`, one union, `number | SomeNone | null` of a union `ty` (#842).
fn with_null(ty: Value) -> Value {
    let mut types = match ty["kind"].as_str() {
        Some("union") => ty["types"].as_array().cloned().unwrap_or_default(),
        _ => vec![ty],
    };
    types.push(keyword("null"));
    json!({ "kind": "union", "types": types })
}

/// Whether `did`, a struct, holds a value JS sees: a field that isn't a
/// `PhantomData`.
fn holds_a_value(tcx: rustc_middle::ty::TyCtxt<'_>, did: DefId) -> bool {
    (tcx.adt_def(did).non_enum_variant().fields.iter()).any(|field| {
        let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
        !matches!(ty.kind(), ty::Adt(adt, _) if adt.is_phantom_data())
    })
}

/// Whether another crate can name `did`, a struct, but not make it, as Rust
/// says: it's `pub`, and one of its fields isn't, `Email(String)`, or it's
/// `#[non_exhaustive]`; so declared opaque (ADR 1187). Not a private one,
/// which no other crate names, whose one outside maker is JS, a caller of
/// what names it, as TypeScript's interface it isn't exported (ADR 0385);
/// nor a class, nor a binding's type, whose `#[rust_js::types]` names JS's,
/// nor one that holds no value, a unit struct or a handle of `PhantomData`,
/// each JS's to make.
pub(super) fn sealed(tcx: rustc_middle::ty::TyCtxt<'_>, did: DefId) -> bool {
    let variant = tcx.adt_def(did).non_enum_variant();
    tcx.visibility(did).is_public()
        && (variant.is_field_list_non_exhaustive() || variant.fields.iter().any(|field| !field.vis.is_public()))
        && holds_a_value(tcx, did)
        && !crate::classes::is_class(tcx, did)
        && super::written_types(tcx, did).is_none()
}

/// Whether a struct's module declares it, so another names it (ADR 0210,
/// amended): one of named fields, a unit struct named as a string (ADR
/// 1164), and each tuple struct that holds a value, a `#[repr(transparent)]`
/// one among them; not a unit struct or a handle of `PhantomData`, JS's.
pub(super) fn declared(tcx: rustc_middle::ty::TyCtxt<'_>, did: DefId) -> bool {
    tcx.adt_def(did).non_enum_variant().ctor.is_none()
        || super::named_unit(tcx, did).is_some()
        || holds_a_value(tcx, did)
}

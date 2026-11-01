//! A class's declaration (ADR 0761): `export declare class Component {
//! .. }`, its constructor, fields, prototype and statics; or, of a class
//! whose constructor function its module doesn't export, its type alone.

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword, type_name};

/// A `pub` class's declaration, exported as its JS exports it: a class
/// whose constructor function isn't, as its `new` isn't `pub`, declared
/// and its type alone exported, `declare class ReactTestInstance { .. }
/// export type { ReactTestInstance };` (ADR 0761, amended).
pub(super) fn exported_class(tcx: TyCtxt<'_>, mut declared: Value, class: DefId) -> Vec<Value> {
    if crate::lower::classes::exports_its_value(tcx, class) || declared["exported"] != true {
        return vec![declared];
    }
    let name = declared["name"].clone();
    declared["exported"] = json!(false);
    vec![
        declared,
        json!({ "kind": "export", "names": [[name, name]], "typeOnly": true }),
    ]
}

impl Declarations<'_, '_> {
    /// `export declare class Component { constructor(props?: unknown);
    /// props?: unknown; setState(partialState?: unknown): void; }`, a JS
    /// class's (ADR 0761): its fields `this`'s, its constructor `new`'s,
    /// its constants and methods its prototype's, and its statics; each
    /// its impl's that's `pub`.
    pub(super) fn class(&mut self, def_id: DefId) -> Value {
        use crate::lower::classes::{Member, member};
        let fields = self.structure(def_id).map(|s| s["members"].clone());
        let mut members: Vec<Value> = Vec::new();
        let mut constructor = None;
        for field in fields.and_then(|f| f.as_array().cloned()).unwrap_or_default() {
            if field["kind"] == "property" {
                let mut property = json!({
                    "kind": "property",
                    "name": field["name"],
                    "static": false,
                    "optional": field["optional"],
                    "type": field["type"],
                });
                if let Some(docs) = field.get("docs") {
                    property["docs"] = docs.clone();
                }
                members.push(property);
            }
        }
        for &imp in self.tcx.inherent_impls(def_id) {
            for &item in self.tcx.associated_item_def_ids(imp) {
                if !self.tcx.visibility(item).is_public() {
                    continue;
                }
                let declared = match member(self.tcx, item) {
                    Some(Member::Constructor) => {
                        constructor = Some(json!({ "kind": "constructor", "params": self.function(item)["params"] }));
                        continue;
                    }
                    Some(Member::Const) => {
                        let constant = self.constant(item);
                        json!({ "kind": "property", "name": constant["name"], "static": false, "optional": false, "type": constant["type"] })
                    }
                    Some(Member::Getter) => {
                        json!({ "kind": "getter", "name": self.js_name(item), "returns": self.function(item)["returns"] })
                    }
                    Some(kind @ (Member::Method | Member::Static)) => {
                        let function = self.function(item);
                        let mut params = function["params"].as_array().cloned().unwrap_or_default();
                        if kind == Member::Method && !params.is_empty() {
                            params.remove(0);
                        }
                        json!({
                            "kind": "method",
                            "name": self.js_name(item),
                            "static": kind == Member::Static,
                            "typeParameters": function["typeParameters"],
                            "params": params,
                            "returns": function["returns"],
                        })
                    }
                    Some(Member::Own | Member::Namespace) | None => continue,
                };
                members.push(documented(declared, self.tcx, item));
            }
        }
        // Each method of a trait of classes it implements (rd283).
        members.extend(self.class_trait_methods(def_id));
        // Its `Display`, its prototype's `toString` (#358).
        if crate::lower::classes::has_to_string(self.tcx, def_id) {
            let returns = keyword("string");
            members.push(json!({ "kind": "method", "name": "toString", "static": false, "typeParameters": [], "params": [], "returns": returns }));
        }
        members.splice(0..0, constructor);
        json!({
            "kind": "class",
            "name": type_name(self.tcx, def_id),
            "exported": true,
            "declare": true,
            "members": members,
        })
    }
}

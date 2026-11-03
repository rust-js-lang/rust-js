//! A module's namespace in its declarations (ADR 1037): its methods the
//! module's functions, of no `self`, and its type `typeof
//! ReactFiberReconciler`, the module imported whole.

use rustc_hir::def::DefKind;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword, type_name};

impl Declarations<'_, '_> {
    /// Each `pub` method of `adt`, a namespace, its module's function,
    /// declared without its `self`; but one that's the function it passes
    /// its parameters on to, which its module re-exports (rd292b).
    pub(super) fn namespace_functions(
        &mut self,
        adt: DefId,
        forwarded: &std::collections::HashMap<DefId, DefId>,
    ) -> Vec<Value> {
        let tcx = self.tcx;
        let mut items = Vec::new();
        let out = self;
        for &imp in tcx.inherent_impls(adt) {
            for &method in tcx.associated_item_def_ids(imp) {
                if !tcx.visibility(method).is_public() || forwarded.contains_key(&method) {
                    continue;
                }
                out.erased = out.erased(method);
                let mut function = out.function(method);
                if let Some(params) = function["params"].as_array_mut()
                    && !params.is_empty()
                {
                    params.remove(0);
                }
                function["name"] = json!(out.js_name(method));
                items.push(documented(function, tcx, method));
            }
        }
        items
    }

    /// The type of `adt`'s value, a module's namespace, `typeof
    /// ReactFiberReconciler`, its module imported whole by the struct's
    /// name, `import type * as ReactFiberReconciler from ..` (ADR 1037).
    /// This module's own, which it can't name so, `any`.
    pub(super) fn namespace_type(&mut self, adt: DefId) -> Value {
        let name = type_name(self.tcx, adt);
        let from = match adt.as_local() {
            Some(local) => {
                let module = self.tcx.parent_module_from_def_id(local);
                if module == self.module {
                    return keyword("any");
                }
                Ok(self.files[&module].clone())
            }
            None => {
                let methods = (self.tcx.inherent_impls(adt).iter())
                    .flat_map(|&imp| self.tcx.associated_item_def_ids(imp).iter().copied());
                let Some(from) = methods
                    .filter_map(|item| self.foreign.item(item))
                    .map(|item| item.from.clone())
                    .next()
                else {
                    return keyword("any");
                };
                Err(from)
            }
        };
        self.namespaces.insert((from, name.clone()));
        json!({ "kind": "typeof", "name": name })
    }

    /// The object of `adt`'s constants another crate or JS can name, an
    /// inherent impl's, `export const ReactiveFlags: { readonly NONE: number;
    /// .. };`, where it has any (#498): each its own type, read only. Not
    /// one of `pub` methods too, which no object type declares yet (#507).
    pub(super) fn type_constants(&mut self, adt: DefId) -> Option<Value> {
        let tcx = self.tcx;
        let items: Vec<DefId> = (tcx.inherent_impls(adt).iter())
            .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
            .collect();
        let methods = items
            .iter()
            .any(|&item| tcx.def_kind(item) == DefKind::AssocFn && tcx.visibility(item).is_public());
        let constants: Vec<DefId> = (items.into_iter())
            .filter(|&item| crate::lower::analysis::exported_value_const(tcx, item))
            .collect();
        if constants.is_empty() || methods {
            return None;
        }
        let members: Vec<Value> = (constants.into_iter())
            .map(|constant| {
                let ty = self.ts(tcx.type_of(constant).instantiate_identity().skip_normalization());
                json!({ "kind": "property", "name": self.js_name(constant), "optional": false, "readonly": true, "type": ty })
            })
            .collect();
        Some(json!({
            "kind": "const",
            "name": type_name(tcx, adt),
            "exported": true,
            "declare": false,
            "type": { "kind": "object", "members": members },
        }))
    }
}

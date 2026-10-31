//! A module's namespace in its declarations (ADR 1037): its methods the
//! module's functions, of no `self`, and its type `typeof
//! ReactFiberReconciler`, the module imported whole.

use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword, type_name};

impl Declarations<'_, '_> {
    /// Each `pub` method of `adt`, a namespace, its module's function,
    /// declared without its `self`.
    pub(super) fn namespace_functions(&mut self, adt: DefId) -> Vec<Value> {
        let tcx = self.tcx;
        let mut items = Vec::new();
        let out = self;
        for &imp in tcx.inherent_impls(adt) {
            for &method in tcx.associated_item_def_ids(imp) {
                if !tcx.visibility(method).is_public() {
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
}

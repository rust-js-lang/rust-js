//! A type's values in its declarations: a module's namespace (ADR 1037),
//! its methods the module's functions, of no `self`, and its type `typeof
//! ReactFiberReconciler`, the module imported whole; and a type's object
//! of constants and methods (ADR 0047, #507).

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

    /// The `pub` type `imp`, an inherent impl, is of, where it's another
    /// module's: its object of methods is this module's (ADR 0047).
    pub(super) fn foreign_impl_type(&self, imp: DefId) -> Option<DefId> {
        let tcx = self.tcx;
        if tcx.def_kind(imp) != (DefKind::Impl { of_trait: false }) {
            return None;
        }
        let rustc_middle::ty::Adt(adt, _) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind() else {
            return None;
        };
        let own = (adt.did().as_local()).is_some_and(|local| tcx.parent_module_from_def_id(local) == self.module);
        (!own && tcx.visibility(adt.did()).is_public()).then_some(adt.did())
    }

    /// `adt`'s object of methods (ADR 0047), as its JS has it, where this
    /// module's impls give it anything another crate or JS can name:
    /// `export const Counter: { readonly START: number; "new"(): Counter;
    /// tick(counter: Counter): void; };`. Its constants first, each its own
    /// type, read only (#498), then its `pub` methods, each of its receiver
    /// first, as the object's function takes it, and of a generic impl its
    /// type parameters, `get<T>(wrapped: Wrapped<T>): T` (#507). What its
    /// object holds is what naming gave it (`FnInfo::owner`): no class's
    /// prototype, no object's own method (ADR 0943).
    pub(super) fn type_object(&mut self, adt: DefId) -> Option<Value> {
        let tcx = self.tcx;
        let held = |item: &DefId| {
            (self.fns.get(item))
                .filter(|info| info.module == self.module && info.owner.is_some())
                .filter(|_| crate::lower::classes::member(tcx, *item).is_none())
        };
        let items: Vec<(DefId, DefId)> = (tcx.inherent_impls(adt).iter())
            .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().map(move |&item| (imp, item)))
            .filter(|(_, item)| held(item).is_some())
            .collect();
        let name = held(&items.first()?.1)?.owner.clone()?;
        let mut members: Vec<Value> = (items.iter())
            .filter(|&&(_, item)| crate::lower::analysis::exported_value_const(tcx, item))
            .map(|&(_, constant)| {
                let ty = self.ts(tcx.type_of(constant).instantiate_identity().skip_normalization());
                json!({ "kind": "property", "name": self.js_name(constant), "optional": false, "readonly": true, "type": ty })
            })
            .collect();
        let methods = (items.iter())
            .filter(|&&(_, item)| tcx.def_kind(item) == DefKind::AssocFn && tcx.visibility(item).is_public());
        for &(imp, method) in methods {
            let outer = std::mem::take(&mut self.erased);
            let mut erased = self.erased(imp);
            erased.extend(self.erased(method));
            self.erased = erased;
            let function = self.function(method);
            let mut type_parameters = self.generics(imp);
            type_parameters.extend(function["typeParameters"].as_array().cloned().unwrap_or_default());
            let declared = json!({
                "kind": "method",
                "name": self.js_name(method),
                "typeParameters": type_parameters,
                "params": function["params"],
                "returns": function["returns"],
            });
            members.push(documented(declared, tcx, method));
            self.erased = outer;
        }
        (!members.is_empty()).then(|| {
            json!({
                "kind": "const",
                "name": name,
                "exported": true,
                "declare": false,
                "type": { "kind": "object", "members": members },
            })
        })
    }
}

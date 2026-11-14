//! A JS type TypeScript's DOM doesn't declare, which a binding says rust-js
//! declares, `#[rust_js::declares]` (#1790): `BatteryManager` of the Battery
//! Status API, read whole of its IDL though TypeScript has none. A crate's
//! declarations that name it declare it once, as TypeScript's DOM declares
//! its classes, `declare global { interface BatteryManager extends
//! EventTarget { readonly charging: boolean; .. } }`, of the binding itself,
//! by the one mapping of Rust to TypeScript: each getter a property,
//! read-only where no setter writes it, each method a method, and what it
//! derefs to what it extends.

use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use serde_json::{Value, json};

use super::super::bindings::{self, JsForm};
use super::Declarations;
use super::written::Written;

impl<'tcx> Declarations<'_, 'tcx> {
    /// `did`, a binding's type its declarations name as it's `written`,
    /// declared once in `declare global { .. }` where it says rust-js
    /// declares it: a global's name, of no module.
    pub(super) fn declare_global(&mut self, did: DefId, written: &Written) {
        let Written::Reference { from: None, name, .. } = written else {
            return;
        };
        let path = [Symbol::intern("rust_js"), Symbol::intern("declares")];
        let key = format!("global {name}");
        if self.tcx.get_attrs_by_path(did, &path).next().is_none() || self.unexported.contains_key(&key) {
            return;
        }
        // Named while it's made, as a member's type may name it again.
        self.unexported.insert(key.clone(), None);
        let interface = self.global_interface(did, name);
        self.unexported
            .insert(key, Some(json!({ "kind": "global", "declarations": [interface] })));
    }

    /// The interface `did`'s binding is, its methods' and accessors'.
    fn global_interface(&mut self, did: DefId, name: &str) -> Value {
        let tcx = self.tcx;
        let methods: Vec<DefId> = (tcx.inherent_impls(did).iter())
            .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
            .filter(|&item| tcx.def_kind(item) == DefKind::AssocFn && tcx.associated_item(item).is_method())
            .filter(|&item| tcx.visibility(item).is_public() && bindings::is_binding(tcx, item))
            .collect();
        let set: Vec<String> = (methods.iter())
            .filter_map(|&m| match bindings::js_form(tcx, m) {
                JsForm::Set(property) => Some(property),
                _ => None,
            })
            .collect();
        let mut members = Vec::new();
        for method in methods {
            let member = match bindings::js_form(tcx, method) {
                JsForm::Get(property) => {
                    let function = self.function(method);
                    json!({
                        "kind": "property",
                        "name": property,
                        "optional": false,
                        "readonly": !set.contains(&property),
                        "type": function["returns"],
                    })
                }
                JsForm::Call(called) => {
                    let function = self.function(method);
                    json!({
                        "kind": "method",
                        "name": called,
                        "optional": false,
                        "typeParameters": function["typeParameters"],
                        "params": function["params"],
                        "returns": function["returns"],
                    })
                }
                _ => continue,
            };
            members.push(member);
        }
        // What it derefs to, the class it extends: `EventTarget`.
        let extends: Vec<Value> = deref_target(tcx, did)
            .map(|target| self.ts(target))
            .into_iter()
            .collect();
        json!({
            "kind": "interface",
            "name": name,
            "exported": false,
            "declare": false,
            "typeParameters": [],
            "extends": extends,
            "members": members,
        })
    }
}

/// `<did as Deref>::Target`, a binding's class's parent, where it derefs.
fn deref_target(tcx: TyCtxt<'_>, did: DefId) -> Option<Ty<'_>> {
    let target = tcx.lang_items().deref_target()?;
    let ty = tcx.type_of(did).instantiate_identity().skip_normalization();
    let projection = Ty::new_projection(tcx, ty::IsRigid::No, target, [ty]);
    (tcx.try_normalize_erasing_regions(
        ty::TypingEnv::fully_monomorphized(),
        ty::Unnormalized::new_wip(projection),
    ))
    .ok()
}

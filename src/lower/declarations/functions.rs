//! What a function is to its declaration: `export function Tag(props:
//! TagProps): ReactNode;`, its parameters, which of them may be left out,
//! and what it gives.

use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword};

impl<'tcx> Declarations<'_, 'tcx> {
    pub(super) fn function(&mut self, def_id: DefId) -> Value {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let idents = self.tcx.fn_arg_idents(def_id);
        let decl = (def_id.as_local()).and_then(|local| self.tcx.hir_node_by_def_id(local).fn_decl());
        // Its trailing `Option`s are optional, `greeting?: string | null`, so a
        // JS caller leaves them out (ADR 0443).
        let tcx = self.tcx;
        let is_option =
            |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
        // Its last, a slice marked `#[rust_js::rest]`, is a rest parameter,
        // `...args: T[]`, which those before it may still be left out before
        // (ADR 0664).
        let rest = super::super::bindings::takes_rest(self.tcx, def_id).then(|| sig.inputs().len() - 1);
        // One it names `#[rust_js::nullable(..)]` is `T | null` as declared,
        // never left out, which each caller gives `null` for `None` and the
        // body takes as it is (ADR 0861, amended, the user's decision,
        // 2026-10-27).
        let nullable_params = super::super::bindings::nullable_params(self.tcx, def_id);
        let nullable = |i: usize| {
            (idents.get(i).copied().flatten()).is_some_and(|ident| nullable_params.contains(&ident.name))
                && is_option(sig.inputs()[i])
        };
        let before = &sig.inputs()[..rest.unwrap_or(sig.inputs().len())];
        let optional_from = before.len()
            - (before.iter().enumerate().rev())
                .take_while(|&(i, &ty)| is_option(ty) && !nullable(i))
                .count();
        let params: Vec<Value> = (sig.inputs().iter().enumerate())
            .map(|(i, &ty)| {
                // As its JS spells it, a word JS keeps given apart, `new$`.
                let name = match idents.get(i).copied().flatten() {
                    // Its JS name's, without a `_` of one unused (ADR 0678).
                    Some(ident) => {
                        let named = super::super::parameter_name(self.tcx, def_id, ident.name.as_str());
                        crate::names::js_ident(&named)
                    }
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let written = decl.and_then(|decl| decl.inputs.get(i));
                let ty = match nullable(i).then(|| self.option(ty)).flatten() {
                    Some(inner) => {
                        let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                        json!({ "kind": "union", "types": [inner, keyword("null")] })
                    }
                    None => self.as_written(written, |out| out.ts(ty)),
                };
                let rest = rest == Some(i);
                json!({ "name": name, "optional": i >= optional_from && !rest, "rest": rest, "type": ty })
            })
            .collect();
        let output = sig.output();
        let written = decl.and_then(|decl| match decl.output {
            hir::FnRetTy::Return(written) => Some(written),
            hir::FnRetTy::DefaultReturn(_) => None,
        });
        // A nullable result's `None` is `null` where it's made, never
        // `undefined`, `string | null` (ADRs 0574, 0578).
        let nullable = (super::super::bindings::nullable_result(self.tcx, def_id))
            .then(|| self.option(output))
            .flatten();
        let returns = match (output.is_unit(), nullable) {
            (true, _) => keyword("void"),
            (false, Some(inner)) => {
                let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                json!({ "kind": "union", "types": [inner, keyword("null")] })
            }
            (false, None) => self.as_written(written, |out| out.ts(output)),
        };
        json!({
            "kind": "function",
            "name": self.js_name(def_id),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "params": params,
            "returns": returns,
        })
    }

    /// Each method of a trait of classes `class` implements, its
    /// prototype's, the impl's or the trait's default, as a class's
    /// method is declared (rd283, ADR 1035).
    pub(super) fn class_trait_methods(&mut self, class: DefId) -> Vec<Value> {
        let mut members = Vec::new();
        for imp in self.tcx.hir_crate_items(()).definitions() {
            let imp = imp.to_def_id();
            if crate::lower::classes::trait_impl_class(self.tcx, imp) != Some(class) {
                continue;
            }
            let trait_id = self.tcx.impl_trait_id(imp);
            if !self.tcx.visibility(trait_id).is_public() {
                continue;
            }
            for &item in self.tcx.associated_item_def_ids(trait_id) {
                let function = self.function(item);
                let mut params = function["params"].as_array().cloned().unwrap_or_default();
                if !params.is_empty() {
                    params.remove(0);
                }
                let declared = json!({
                    "kind": "method",
                    "name": crate::lower::bindings::fn_name(self.tcx, item),
                    "static": false,
                    "typeParameters": function["typeParameters"],
                    "params": params,
                    "returns": function["returns"],
                });
                members.push(documented(declared, self.tcx, item));
            }
        }
        members
    }
}

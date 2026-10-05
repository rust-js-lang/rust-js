//! A module's TypeScript declarations, `Tag.d.ts` beside its JS, for the
//! TypeScript that imports it (ADR 0196): what it exports, as Rust types it,
//! each by the name it has in JS. An `Option` field is optional, a unit-only
//! enum its names, a `memo`'s component its props. What has no type here,
//! another module's type or a JS object's, is `any`, so a caller isn't held
//! to less than Rust holds it to.

use std::collections::BTreeSet;
use std::fmt::Write;

use rustc_hir::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::{DefId, LocalModDefId};

use super::bindings::{field_key, fn_name, is_binding, is_flatten, is_mark, is_rest, variant_name};
use super::recognition::{StdItem, is_std_def};
use super::representation::Num;

/// What a module's `.d.ts` says, but its header: `None` where it exports nothing.
pub(super) fn module(tcx: TyCtxt<'_>, module: LocalModDefId, default_export: Option<&str>) -> Option<String> {
    let mut out = Declarations {
        tcx,
        module,
        imports: BTreeSet::new(),
    };
    // Each declaration, a blank line between them, as one writes them.
    let mut items = Vec::new();
    for item in tcx.hir_module_free_items(module) {
        let def_id = item.owner_id.to_def_id();
        if !tcx.visibility(def_id).is_public() || is_mark(tcx, def_id) {
            continue;
        }
        let mut item = String::new();
        match tcx.def_kind(def_id) {
            DefKind::Fn if !is_binding(tcx, def_id) => out.function(def_id, &mut item),
            DefKind::Struct => out.structure(def_id, &mut item),
            DefKind::Enum => out.enumeration(def_id, &mut item),
            DefKind::Const { .. } | DefKind::Static { .. } if tcx.item_name(def_id).as_str() != "_" => {
                out.constant(def_id, &mut item)
            }
            _ => {}
        }
        if !item.is_empty() {
            items.push(item);
        }
    }
    if let Some(name) = default_export {
        items.push(format!("export default {name};\n"));
    }
    if items.is_empty() {
        return None;
    }
    let body = items.join("\n");
    // `import type { NamedExoticComponent, ReactNode } from "react";`, each module's.
    let mut imports = String::new();
    let mut modules: Vec<&str> = out.imports.iter().map(|(from, _)| from.as_str()).collect();
    modules.dedup();
    for from in modules {
        let names: Vec<&str> = out
            .imports
            .iter()
            .filter(|(f, _)| f == from)
            .map(|(_, n)| n.as_str())
            .collect();
        let _ = writeln!(imports, "import type {{ {} }} from {from:?};", names.join(", "));
    }
    if !imports.is_empty() {
        imports.push('\n');
    }
    Some(imports + &body)
}

struct Declarations<'tcx> {
    tcx: TyCtxt<'tcx>,
    module: LocalModDefId,
    /// The types of JS modules' it names, `("react", "ReactNode")`, imported from them.
    imports: BTreeSet<(String, String)>,
}

impl<'tcx> Declarations<'tcx> {
    /// `<C>`: an item's type parameters, but those `impl Trait` stands for.
    fn generics(&self, def_id: DefId) -> String {
        let names: Vec<String> = self
            .tcx
            .generics_of(def_id)
            .own_params
            .iter()
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { synthetic: false, .. }))
            .map(|param| param.name.to_string())
            .collect();
        match names.is_empty() {
            true => String::new(),
            false => format!("<{}>", names.join(", ")),
        }
    }

    /// `export function Tag(props: TagProps): ReactNode;`
    fn function(&mut self, def_id: DefId, out: &mut String) {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let idents = self.tcx.fn_arg_idents(def_id);
        let params: Vec<String> = sig
            .inputs()
            .iter()
            .enumerate()
            .map(|(i, &ty)| {
                let name = match idents.get(i).copied().flatten() {
                    Some(ident) => super::camel_case(ident.name.as_str()),
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                format!("{name}: {}", self.ts(ty))
            })
            .collect();
        let output = sig.output();
        let ret = match output.is_unit() {
            true => "void".to_string(),
            false => self.ts(output),
        };
        let _ = writeln!(
            out,
            "export function {}{}({}): {ret};",
            fn_name(self.tcx, def_id),
            self.generics(def_id),
            params.join(", ")
        );
    }

    /// `export interface TagProps { variant: RouteTag; text?: string; }`
    fn structure(&mut self, def_id: DefId, out: &mut String) {
        let adt = self.tcx.adt_def(def_id);
        let variant = adt.non_enum_variant();
        if variant.ctor.is_some() {
            return;
        }
        let name = self.tcx.item_name(def_id);
        // A flattened field's struct is what this one extends (ADR 0204):
        // one TypeScript can't see, another module's (`any`), is what a
        // `Rest` is.
        let flattened = variant
            .fields
            .iter()
            .find(|f| is_flatten(self.tcx, f))
            .map(|field| self.ts(self.tcx.type_of(field.did).instantiate_identity().skip_normalization()));
        let extends = match &flattened {
            Some(ts) if ts != "any" => format!(" extends {ts}"),
            _ => String::new(),
        };
        let _ = writeln!(out, "export interface {name}{}{extends} {{", self.generics(def_id));
        for field in &variant.fields {
            let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
            // What a JS caller gives besides, `...rest` (ADR 0195).
            if is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty()) {
                let _ = writeln!(out, "  [prop: string]: unknown;");
                continue;
            }
            if is_flatten(self.tcx, field) {
                continue;
            }
            let key = field_key(self.tcx, field);
            match self.option(ty) {
                Some(inner) => {
                    let _ = writeln!(out, "  {key}?: {};", self.ts(inner));
                }
                None => {
                    let _ = writeln!(out, "  {key}: {};", self.ts(ty));
                }
            }
        }
        let _ = writeln!(out, "}}");
    }

    /// `export type RouteTag = "foundation" | "intermediate";`, of an enum
    /// whose variants hold nothing: each is its name (ADR 0013).
    fn enumeration(&mut self, def_id: DefId, out: &mut String) {
        let adt = self.tcx.adt_def(def_id);
        let name = self.tcx.item_name(def_id);
        let union = match adt.variants().iter().all(|v| v.fields.is_empty()) {
            true => adt
                .variants()
                .iter()
                .map(|v| format!("{:?}", variant_name(self.tcx, v)))
                .collect::<Vec<_>>()
                .join(" | "),
            false => "any".to_string(),
        };
        let _ = writeln!(out, "export type {name} = {union};");
    }

    /// `export const IconChevron: NamedExoticComponent<IconChevronProps>;`, a
    /// `thread_local!`'s value, or a `const`'s.
    fn constant(&mut self, def_id: DefId, out: &mut String) {
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        let ty = match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::LocalKey) => args.type_at(0),
            _ => ty,
        };
        let _ = writeln!(out, "export const {}: {};", fn_name(self.tcx, def_id), self.ts(ty));
    }

    fn option(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// `ty` as a TypeScript type.
    fn ts(&mut self, ty: Ty<'tcx>) -> String {
        let tcx = self.tcx;
        if let Some(num) = Num::of(ty) {
            return if num.big() { "bigint" } else { "number" }.to_string();
        }
        match ty.kind() {
            ty::Bool => "boolean".to_string(),
            ty::Char | ty::Str => "string".to_string(),
            ty::Ref(_, inner, _) => self.ts(*inner),
            ty::Tuple(items) if items.is_empty() => "undefined".to_string(),
            ty::Tuple(items) => {
                let items: Vec<String> = items.iter().map(|t| self.ts(t)).collect();
                format!("[{}]", items.join(", "))
            }
            ty::Array(item, _) | ty::Slice(item) => format!("{}[]", self.grouped(*item)),
            ty::Param(param) => param.name.to_string(),
            ty::FnPtr(..) | ty::Closure(..) | ty::Dynamic(..) => "(...args: any[]) => any".to_string(),
            ty::Adt(adt, args) => {
                let did = adt.did();
                if tcx.is_lang_item(did, LangItem::Option) {
                    return format!("{} | undefined", self.ts(args.type_at(0)));
                }
                if tcx.is_lang_item(did, LangItem::String) {
                    return "string".to_string();
                }
                if is_std_def(tcx, did, StdItem::Vec) {
                    return format!("{}[]", self.grouped(args.type_at(0)));
                }
                if ty.is_box() {
                    return self.ts(args.type_at(0));
                }
                if is_rest(tcx, ty) {
                    return "Record<string, unknown>".to_string();
                }
                // A binding's, as it says it's typed: React's `Memo<P>`,
                // `react#NamedExoticComponent`, is `NamedExoticComponent<P>`.
                let path = [Symbol::intern("rust_js"), Symbol::intern("types")];
                if let Some(declared) = tcx
                    .get_attrs_by_path(did, &path)
                    .next()
                    .and_then(|attr| attr.value_str())
                {
                    let declared = declared.to_string();
                    let name = match declared.rsplit_once('#') {
                        Some((from, name)) => {
                            self.imports.insert((from.to_string(), name.to_string()));
                            name.to_string()
                        }
                        None => declared,
                    };
                    let args: Vec<String> = args.types().map(|t| self.ts(t)).collect();
                    return match args.is_empty() {
                        true => name,
                        false => format!("{name}<{}>", args.join(", ")),
                    };
                }
                // One of this module's, by its name; another's is `any`, as
                // its declarations aren't imported (ADR 0196).
                let ours = did
                    .as_local()
                    .is_some_and(|local| tcx.parent_module_from_def_id(local) == self.module)
                    && tcx.visibility(did).is_public();
                if !ours {
                    return "any".to_string();
                }
                let args: Vec<String> = args.types().map(|t| self.ts(t)).collect();
                match args.is_empty() {
                    true => tcx.item_name(did).to_string(),
                    false => format!("{}<{}>", tcx.item_name(did), args.join(", ")),
                }
            }
            _ => "any".to_string(),
        }
    }

    /// `ty`, in parentheses where it's a union: `(string | undefined)[]`.
    fn grouped(&mut self, ty: Ty<'tcx>) -> String {
        let ts = self.ts(ty);
        match ts.contains(' ') {
            true => format!("({ts})"),
            false => ts,
        }
    }
}

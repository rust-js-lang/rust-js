//! A module's TypeScript declarations, `Tag.d.ts` beside its JS, for the
//! TypeScript that imports it (ADR 0196): what it exports, as Rust types it,
//! each by the name it has in JS. An `Option` field is optional, a unit-only
//! enum its names, a `memo`'s component its props. What has no type here,
//! another module's type or a JS object's, is `any`, so a caller isn't held
//! to less than Rust holds it to. They're the model of @rust-js/typescript
//! (ADR 0206), which TypeScript prints (ADR 0207).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::{DefId, LocalModId};
use serde_json::{Value, json};

use super::bindings::{
    field_default, field_key, fn_name, is_binding, is_flatten, is_mark, is_nullable, is_rest, is_untagged, variant_name,
};
use super::recognition::{StdItem, is_std_def};
use super::representation::Num;

/// What a module's `.d.ts` declares, but its header: `None` where it
/// exports nothing.
pub(super) fn module(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    default_export: Option<DefId>,
    files: &HashMap<LocalModId, Vec<String>>,
) -> Option<Value> {
    let mut out = Declarations {
        tcx,
        module,
        files,
        imports: BTreeSet::new(),
        module_imports: BTreeSet::new(),
        foreign: BTreeMap::new(),
        erased: HashMap::new(),
    };
    let mut items = Vec::new();
    for item in tcx.hir_module_free_items(module) {
        let def_id = item.owner_id.to_def_id();
        if !tcx.visibility(def_id).is_public() || is_mark(tcx, def_id) {
            continue;
        }
        out.erased = out.erased(def_id);
        let declaration = match tcx.def_kind(def_id) {
            DefKind::Fn if !is_binding(tcx, def_id) => Some(out.function(def_id)),
            DefKind::Struct => out.structure(def_id),
            DefKind::Enum => Some(out.enumeration(def_id)),
            DefKind::TyAlias => Some(out.alias(def_id)),
            DefKind::Const { .. } | DefKind::Static { .. } if tcx.item_name(def_id).as_str() != "_" => {
                Some(out.constant(def_id))
            }
            _ => None,
        };
        items.extend(declaration);
    }
    if let Some(def_id) = default_export {
        // One only `js::export_default!` exports is declared, but exported
        // by no name of its own, as `function Recap() {..}` is in JS.
        if !tcx.visibility(def_id).is_public() {
            out.erased = out.erased(def_id);
            let mut declared = out.function(def_id);
            declared["exported"] = json!(false);
            declared["declare"] = json!(true);
            items.push(declared);
        }
        items.push(json!({ "kind": "export-default", "name": fn_name(tcx, def_id) }));
    }
    // Another crate's untagged enums it names, `js::Json`, each declared here,
    // not exported: the union of its payloads, named, so it can be recursive.
    items.extend(out.foreign.into_values().flatten());
    if items.is_empty() {
        return None;
    }
    // `import type { NamedExoticComponent, ReactNode } from "react";`, each module's.
    let mut declarations = Vec::new();
    let mut modules: Vec<&str> = out.imports.iter().map(|(from, _)| from.as_str()).collect();
    modules.dedup();
    for from in modules {
        let names: Vec<&str> = (out.imports.iter())
            .filter(|(f, _)| f == from)
            .map(|(_, n)| n.as_str())
            .collect();
        declarations.push(json!({ "kind": "import", "from": from, "names": names, "typeOnly": true }));
    }
    // `import type { RouteItem } from "./routes.js";`, of the crate's other
    // modules, by their path, which the output makes their file's (ADR 0210).
    let mut paths: Vec<&Vec<String>> = out.module_imports.iter().map(|(path, _)| path).collect();
    paths.dedup();
    for path in paths {
        let names: Vec<&str> = (out.module_imports.iter())
            .filter(|(p, _)| p == path)
            .map(|(_, n)| n.as_str())
            .collect();
        declarations.push(json!({ "kind": "import", "module": path, "names": names, "typeOnly": true }));
    }
    declarations.extend(items);
    Some(json!({ "declarations": declarations }))
}

/// An item's type parameters bound by a trait that says what it is to
/// TypeScript, each with the trait: `(0, ReactNode)` of `fn Tag<C: ReactNode>`.
fn erasures(tcx: TyCtxt<'_>, def_id: DefId) -> Vec<(u32, DefId)> {
    (tcx.clauses_of(def_id).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.skip_binder())
        .filter_map(|bound| match bound.self_ty().kind() {
            ty::Param(param) if written_types(tcx, bound.def_id()).is_some() => Some((param.index, bound.def_id())),
            _ => None,
        })
        .collect()
}

/// A parameter's name, of its type's: `event` of a `MouseEvent`, `element`
/// of an `HTMLButtonElement`, its last word's; `value` of any other.
fn param_name(ty: &Value) -> String {
    let Some(name) = ty["name"].as_str().filter(|_| ty["kind"] == "reference") else {
        return "value".to_string();
    };
    let start = (name.char_indices())
        .filter(|&(i, c)| c.is_uppercase() && name[i + c.len_utf8()..].starts_with(|n: char| n.is_lowercase()))
        .map(|(i, _)| i)
        .next_back()
        .unwrap_or(0);
    let word = &name[start..];
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => "value".to_string(),
    }
}

/// What `#[rust_js::types]` says an item is to TypeScript.
fn written_types(tcx: TyCtxt<'_>, did: DefId) -> Option<String> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("types")];
    let declared = tcx.get_attrs_by_path(did, &path).next()?.value_str()?;
    Some(declared.to_string())
}

/// What a type's name imports: the name, or a namespace's, `JSX` of
/// `JSX.Element` (ADR 0236).
fn imported(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_string()
}

/// A type the model names, with its arguments: `ReactNode`, `Omit<A, "b">`.
fn reference(name: &str, args: Vec<Value>) -> Value {
    json!({ "kind": "reference", "name": name, "args": args })
}

fn keyword(keyword: &str) -> Value {
    json!({ "kind": "keyword", "keyword": keyword })
}

fn is_any(ty: &Value) -> bool {
    ty["kind"] == "keyword" && ty["keyword"] == "any"
}

struct Declarations<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    module: LocalModId,
    /// The crate's modules that have a file, by their path: those whose
    /// types another's declarations import (ADR 0210).
    files: &'a HashMap<LocalModId, Vec<String>>,
    /// The types of JS modules' it names, `("react", "ReactNode")`, imported from them.
    imports: BTreeSet<(String, String)>,
    /// The types of the crate's other modules it names, by their path.
    module_imports: BTreeSet<(Vec<String>, String)>,
    /// Another crate's untagged enums it names, by name: each one's
    /// declaration, `None` while it's being made, as a payload names it.
    foreign: BTreeMap<String, Option<Value>>,
    /// The type parameters of the item being declared that are a type of
    /// TypeScript's, by index: `C: ReactNode`'s `ReactNode`.
    erased: HashMap<u32, Value>,
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// `["C"]`: an item's type parameters, but those `impl Trait` stands
    /// for, and those that are a type of TypeScript's.
    fn generics(&mut self, def_id: DefId) -> Vec<Value> {
        let erased = erasures(self.tcx, def_id);
        (self.tcx.generics_of(def_id).own_params.iter())
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { synthetic: false, .. }))
            .filter(|param| !erased.iter().any(|(index, _)| *index == param.index))
            .map(|param| json!({ "name": param.name.as_str() }))
            .collect()
    }

    /// An item's type parameters that are a type of TypeScript's, by index,
    /// as the trait each is bound by says (`#[rust_js::types]`): `C: ReactNode`
    /// is a `ReactNode`, as a person writes `children: ReactNode`.
    fn erased(&mut self, def_id: DefId) -> HashMap<u32, Value> {
        let mut erased = HashMap::new();
        for (index, bound) in erasures(self.tcx, def_id) {
            if !erased.contains_key(&index)
                && let Some(declared) = written_types(self.tcx, bound)
            {
                erased.insert(index, self.written(bound, &declared, ty::List::empty()));
            }
        }
        erased
    }

    /// The arguments `did`'s declaration takes of `args`: those of its
    /// type parameters that aren't a type of TypeScript's.
    fn type_args(&mut self, did: DefId, args: ty::GenericArgsRef<'tcx>) -> Vec<Value> {
        let erased = erasures(self.tcx, did);
        (args.iter().enumerate())
            .filter(|(index, _)| !erased.iter().any(|(e, _)| *e as usize == *index))
            .filter_map(|(_, arg)| arg.as_type())
            .map(|t| self.ts(t))
            .collect()
    }

    /// A type as its binding says TypeScript has it, `#[rust_js::types]`, of
    /// its Rust type's arguments `args`.
    fn written(&mut self, did: DefId, declared: &str, args: ty::GenericArgsRef<'tcx>) -> Value {
        let params = &self.tcx.generics_of(did).own_params;
        // `{ [key: string]: T }`, an object of `T`s by name, as
        // `Dict<T>` is: TypeScript's own spelling of one, which a
        // recursive alias may hold, as `Record<string, T>` it may not.
        if let Some(index) = declared.strip_prefix("{ [").and_then(|d| d.strip_suffix(" }"))
            && let Some((key, value)) = index.split_once("]: ")
            && let Some((parameter, key)) = key.split_once(": ")
        {
            let value = match params.iter().find(|p| p.name.as_str() == value) {
                Some(param) => self.ts(args.type_at(param.index as usize)),
                None => reference(value, Vec::new()),
            };
            return json!({ "kind": "object", "members": [
                { "kind": "index", "parameter": parameter, "key": keyword(key), "type": value },
            ] });
        }
        let (from, named) = match declared.rsplit_once('#') {
            Some((from, named)) => (Some(from), named),
            None => (None, declared),
        };
        // Arguments written are all of TypeScript's type's, `<>` none:
        // react's `JSX::Element<T>` is a `JSX.Element` whatever its tag's
        // element (ADR 0224). Else they're the Rust type's.
        let (name, given) = match named.split_once('<') {
            Some((name, given)) => (name, Some(given.trim_end_matches('>'))),
            None => (named, None),
        };
        // A namespace's member, `JSX.Element`, imports the namespace.
        if let Some(from) = from {
            self.imports.insert((from.to_string(), imported(name)));
        }
        // A written argument that's one of the Rust type's own type
        // parameters is what it's given: `Map<string, T>` of a
        // `Dict<f64>` would be `Map<string, number>`.
        let type_args: Vec<Value> = match given {
            Some(given) => (given.split(',').map(str::trim))
                .filter(|arg| !arg.is_empty())
                .map(|arg| match params.iter().find(|p| p.name.as_str() == arg) {
                    Some(param) => self.ts(args.type_at(param.index as usize)),
                    None => reference(arg, Vec::new()),
                })
                .collect(),
            None => args.types().map(|t| self.ts(t)).collect(),
        };
        reference(name, type_args)
    }

    /// A type written as an alias that says what it is to TypeScript,
    /// `#[rust_js::types = "react#MouseEventHandler<T>"]`: that, as a person
    /// writes it, `MouseEventHandler<HTMLButtonElement>`, where rustc's
    /// type is the alias expanded. Its arguments are those written, one
    /// left to its default left out, as TypeScript's has the same.
    fn written_alias(&mut self, written: &hir::Ty<'tcx>) -> Option<Value> {
        let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = &written.kind else {
            return None;
        };
        let Res::Def(DefKind::TyAlias, did) = path.res else {
            return None;
        };
        let declared = written_types(self.tcx, did)?;
        let (from, named) = declared.rsplit_once('#').unwrap_or(("", declared.as_str()));
        let (name, wanted) = match named.split_once('<') {
            Some((name, wanted)) => (name, wanted.trim_end_matches('>')),
            None => (named, ""),
        };
        let given: Vec<&hir::Ty<'tcx>> = (path.segments.last()?.args)
            .map(|args| args.args.iter())
            .into_iter()
            .flatten()
            .filter_map(|arg| match arg {
                hir::GenericArg::Type(ty) => Some(ty.as_unambig_ty()),
                _ => None,
            })
            .collect();
        let params: Vec<Symbol> = (self.tcx.generics_of(did).own_params.iter())
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { .. }))
            .map(|param| param.name)
            .collect();
        let mut args = Vec::new();
        for wanted in wanted.split(',').map(str::trim).filter(|w| !w.is_empty()) {
            let at = params.iter().position(|param| param.as_str() == wanted)?;
            let Some(given) = given.get(at) else { break };
            let ty = rustc_hir_analysis::lower_ty(self.tcx, given);
            args.push(self.ts(ty));
        }
        if !from.is_empty() {
            self.imports.insert((from.to_string(), imported(name)));
        }
        Some(reference(name, args))
    }

    /// `Option<T>`'s `T` as written, where `written` is one.
    fn written_option<'h>(&self, written: &'h hir::Ty<'tcx>) -> Option<&'h hir::Ty<'tcx>> {
        let hir::TyKind::Path(hir::QPath::Resolved(None, path)) = &written.kind else {
            return None;
        };
        let Res::Def(DefKind::Enum, did) = path.res else {
            return None;
        };
        if !self.tcx.is_lang_item(did, LangItem::Option) {
            return None;
        }
        match path.segments.last()?.args?.args {
            [hir::GenericArg::Type(inner)] => Some(inner.as_unambig_ty()),
            _ => None,
        }
    }

    /// `export function Tag(props: TagProps): ReactNode;`
    fn function(&mut self, def_id: DefId) -> Value {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let idents = self.tcx.fn_arg_idents(def_id);
        let decl = (def_id.as_local()).and_then(|local| self.tcx.hir_node_by_def_id(local).fn_decl());
        let params: Vec<Value> = (sig.inputs().iter().enumerate())
            .map(|(i, &ty)| {
                let name = match idents.get(i).copied().flatten() {
                    Some(ident) => super::camel_case(ident.name.as_str()),
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let written = decl
                    .and_then(|decl| decl.inputs.get(i))
                    .and_then(|t| self.written_alias(t));
                let ty = written.unwrap_or_else(|| self.ts(ty));
                json!({ "name": name, "optional": false, "rest": false, "type": ty })
            })
            .collect();
        let output = sig.output();
        let returns = match output.is_unit() {
            true => keyword("void"),
            false => self.ts(output),
        };
        json!({
            "kind": "function",
            "name": fn_name(self.tcx, def_id),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "params": params,
            "returns": returns,
        })
    }

    /// `export interface TagProps { variant: RouteTag; text?: string; }`
    fn structure(&mut self, def_id: DefId) -> Option<Value> {
        let adt = self.tcx.adt_def(def_id);
        let variant = adt.non_enum_variant();
        if variant.ctor.is_some() {
            return None;
        }
        // A flattened field's struct is what this one extends (ADR 0204):
        // one TypeScript can't see, another module's (`any`), is what a
        // `Rest` is. A name both have is this one's, which TypeScript's
        // `Omit` says (ADR 0205).
        let extends: Vec<Value> = match variant.fields.iter().find(|f| is_flatten(self.tcx, f)) {
            Some(field) => {
                let ty = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
                let own: Vec<String> = (variant.fields.iter())
                    .filter(|f| f.did != field.did)
                    .map(|f| field_key(self.tcx, f))
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
            if is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty()) {
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
            let (optional, ty) = match self.option(ty) {
                Some(inner) if is_nullable(self.tcx, field) => (
                    true,
                    json!({ "kind": "union", "types": [self.ts(inner), keyword("null")] }),
                ),
                Some(inner) => {
                    let alias = written
                        .and_then(|w| self.written_option(w))
                        .and_then(|w| self.written_alias(w));
                    (true, alias.unwrap_or_else(|| self.ts(inner)))
                }
                None => {
                    let alias = written.and_then(|w| self.written_alias(w));
                    (
                        field_default(self.tcx, field).is_some(),
                        alias.unwrap_or_else(|| self.ts(ty)),
                    )
                }
            };
            members.push(json!({
                "kind": "property",
                "name": field_key(self.tcx, field),
                "optional": optional,
                "readonly": false,
                "type": ty,
            }));
        }
        Some(json!({
            "kind": "interface",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "extends": extends,
            "members": members,
        }))
    }

    /// `export type RouteTag = "foundation" | "intermediate";`, of an enum
    /// whose variants hold nothing: each is its name (ADR 0013).
    /// `export type Toc = TocItem[];` of `pub type Toc = Vec<TocItem>;`.
    fn alias(&mut self, def_id: DefId) -> Value {
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": self.ts(ty),
        })
    }

    fn enumeration(&mut self, def_id: DefId) -> Value {
        let adt = self.tcx.adt_def(def_id);
        let untagged = is_untagged(self.tcx, def_id);
        let union = match adt.variants().iter().all(|v| v.fields.is_empty()) {
            // `export type Size = string | number;` of an untagged enum, its
            // payloads' union (ADR 0214).
            _ if untagged => {
                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
                let types: Vec<Value> = (adt.variants().iter())
                    .filter_map(|v| v.fields.iter().next())
                    .map(|field| self.ts(field.ty(self.tcx, args).skip_normalization()))
                    .collect();
                json!({ "kind": "union", "types": types })
            }
            true => json!({
                "kind": "union",
                "types": (adt.variants().iter())
                    .map(|v| json!({ "kind": "literal", "value": variant_name(self.tcx, v) }))
                    .collect::<Vec<_>>(),
            }),
            false => keyword("any"),
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

    /// `export const IconChevron: NamedExoticComponent<IconChevronProps>;`, a
    /// `thread_local!`'s value, or a `const`'s.
    fn constant(&mut self, def_id: DefId) -> Value {
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        let ty = match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::LocalKey) => args.type_at(0),
            _ => ty,
        };
        json!({
            "kind": "const",
            "name": fn_name(self.tcx, def_id),
            "exported": true,
            "declare": false,
            "type": self.ts(ty),
        })
    }

    fn option(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// `(event: MouseEvent<Element>) => void`: a function type, each
    /// parameter named by its type, as a person names one.
    fn function_type(&mut self, inputs: &[Ty<'tcx>], output: Ty<'tcx>) -> Value {
        let mut names: Vec<String> = Vec::new();
        let params: Vec<Value> = (inputs.iter())
            .map(|&input| {
                let ty = self.ts(input);
                let base = param_name(&ty);
                let taken = names
                    .iter()
                    .filter(|n| n.trim_end_matches(char::is_numeric) == base)
                    .count();
                let name = if taken == 0 {
                    base
                } else {
                    format!("{base}{}", taken + 1)
                };
                names.push(name.clone());
                json!({ "name": name, "optional": false, "rest": false, "type": ty })
            })
            .collect();
        let returns = match output.is_unit() {
            true => keyword("void"),
            false => self.ts(output),
        };
        json!({ "kind": "function", "typeParameters": [], "params": params, "returns": returns })
    }

    /// `ty` as a TypeScript type.
    fn ts(&mut self, ty: Ty<'tcx>) -> Value {
        let tcx = self.tcx;
        if let Some(num) = Num::of(ty) {
            return keyword(if num.big() { "bigint" } else { "number" });
        }
        match ty.kind() {
            ty::Bool => keyword("boolean"),
            ty::Char | ty::Str => keyword("string"),
            ty::Ref(_, inner, _) => self.ts(*inner),
            ty::Tuple(items) if items.is_empty() => keyword("undefined"),
            ty::Tuple(items) => {
                let elements: Vec<Value> = items.iter().map(|t| self.ts(t)).collect();
                json!({ "kind": "tuple", "elements": elements })
            }
            ty::Array(item, _) | ty::Slice(item) => {
                json!({ "kind": "array", "element": self.ts(*item), "readonly": false })
            }
            ty::Param(param) => match self.erased.get(&param.index) {
                Some(erased) => erased.clone(),
                // An `impl Trait` parameter has no name but rustc's, `impl
                // js::Defined + 'a`, which isn't TypeScript, and `generics`
                // leaves it out: a value of any type, as a person declares one.
                None if param.name.as_str().starts_with("impl ") => keyword("unknown"),
                None => reference(param.name.as_str(), Vec::new()),
            },
            // A function of what Rust says it takes and gives:
            // `dyn Fn(&event::MouseEvent)` is `(event: MouseEvent<Element>) => void`.
            ty::FnPtr(..) => {
                let sig = ty.fn_sig(tcx).skip_binder();
                self.function_type(sig.inputs(), sig.output())
            }
            ty::Dynamic(traits, ..)
                if traits
                    .principal_def_id()
                    .is_some_and(|t| tcx.fn_trait_kind_from_def_id(t).is_some()) =>
            {
                let inputs = match traits.principal().map(|p| p.skip_binder().args.type_at(0).kind()) {
                    Some(ty::Tuple(inputs)) => inputs.as_slice(),
                    _ => &[],
                };
                let output = (traits.projection_bounds())
                    .find_map(|output| output.skip_binder().term.as_type())
                    .unwrap_or(tcx.types.unit);
                self.function_type(inputs, output)
            }
            // `(...args: any[]) => any`: a closure's own type, which no
            // signature names, or a `dyn` of another trait.
            ty::Closure(..) | ty::Dynamic(..) => json!({
                "kind": "function",
                "typeParameters": [],
                "params": [{
                    "name": "args",
                    "optional": false,
                    "rest": true,
                    "type": { "kind": "array", "element": keyword("any"), "readonly": false },
                }],
                "returns": keyword("any"),
            }),
            ty::Adt(adt, args) => {
                let did = adt.did();
                // `None` is JS's `null` too, as rust-js reads it `!= null` (ADR 0030).
                if tcx.is_lang_item(did, LangItem::Option) {
                    return json!({ "kind": "union", "types": [self.ts(args.type_at(0)), keyword("null"), keyword("undefined")] });
                }
                if tcx.is_lang_item(did, LangItem::String) {
                    return keyword("string");
                }
                if is_std_def(tcx, did, StdItem::Vec) {
                    return json!({ "kind": "array", "element": self.ts(args.type_at(0)), "readonly": false });
                }
                if ty.is_box() {
                    return self.ts(args.type_at(0));
                }
                if is_rest(tcx, ty) {
                    return reference("Record", vec![keyword("string"), keyword("unknown")]);
                }
                // A binding's, as it says it's typed: React's `MemoExoticComponent<P>`,
                // `react#NamedExoticComponent`, is `NamedExoticComponent<P>`,
                // and `react#AnchorHTMLAttributes<HTMLAnchorElement>` that.
                if let Some(declared) = written_types(tcx, did) {
                    return self.written(did, &declared, args);
                }
                // Another crate's untagged enum, `js::Json`: declared here, as
                // this crate's are, the union of its payloads (ADR 0225).
                if !did.is_local() && adt.is_enum() && is_untagged(tcx, did) {
                    let name = tcx.item_name(did).to_string();
                    if !self.foreign.contains_key(&name) {
                        self.foreign.insert(name.clone(), None);
                        let inner = self.erased(did);
                        let outer = std::mem::replace(&mut self.erased, inner);
                        let mut declared = self.enumeration(did);
                        self.erased = outer;
                        declared["exported"] = json!(false);
                        self.foreign.insert(name.clone(), Some(declared));
                    }
                    let type_args = self.type_args(did, args);
                    return reference(&name, type_args);
                }
                // One of the crate's, declared: this module's by its name,
                // another's imported from it (ADR 0210). Any other is `any`.
                let Some(local) = did.as_local() else {
                    return keyword("any");
                };
                let declared = match tcx.def_kind(did) {
                    DefKind::Struct => adt.non_enum_variant().ctor.is_none(),
                    DefKind::Enum => true,
                    _ => false,
                };
                if !declared || !tcx.visibility(did).is_public() {
                    return keyword("any");
                }
                // Another module's is imported from its file; one of a module
                // without one, of types only, has nowhere it's declared.
                let home = tcx.parent_module_from_def_id(local);
                if home != self.module {
                    let Some(path) = self.files.get(&home) else {
                        return keyword("any");
                    };
                    self.module_imports
                        .insert((path.clone(), tcx.item_name(did).to_string()));
                }
                let type_args = self.type_args(did, args);
                reference(tcx.item_name(did).as_str(), type_args)
            }
            _ => keyword("any"),
        }
    }
}

/// The JS names of `ty`'s fields, a flattened struct's, and of those of
/// the flattened struct in it (ADR 0205).
fn flattened_keys<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<String> {
    let ty::Adt(adt, args) = ty.kind() else {
        return Vec::new();
    };
    if !adt.is_struct() {
        return Vec::new();
    }
    let mut keys = Vec::new();
    for field in &adt.non_enum_variant().fields {
        if is_flatten(tcx, field) {
            keys.extend(flattened_keys(tcx, field.ty(tcx, args).skip_normalization()));
        } else {
            keys.push(field_key(tcx, field));
        }
    }
    keys
}

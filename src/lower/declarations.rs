//! A module's TypeScript declarations, `Tag.d.ts` beside its JS, for the
//! TypeScript that imports it (ADR 0196): what it exports, as Rust types it,
//! each by the name it has in JS. An `Option` field is optional, a unit-only
//! enum its names, a `memo`'s component its props. What has no type here,
//! another module's type or a JS object's, is `any`, so a caller isn't held
//! to less than Rust holds it to. They're the model of @rust-js/typescript
//! (ADR 0206), which TypeScript prints (ADR 0207).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::program::Reexport;
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::{DefKind, Res};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::{DefId, LocalModId};
use serde_json::{Value, json};

use super::bindings::{
    declared_tag, field_default, field_key, fn_name, is_binding, is_flatten, is_mark, is_nullable, is_otherwise,
    is_rest, is_untagged, tag_key, variant_name,
};
use super::recognition::{StdItem, is_std_def};
use super::representation::Num;

mod written;

pub use written::validate_types;
use written::{Member, Signature};
pub(super) use written::{Written, written_types};

/// What a module's `.d.ts` declares, but its header: `None` where it
/// exports nothing.
#[allow(clippy::too_many_arguments)]
pub(super) fn module<'tcx>(
    tcx: TyCtxt<'tcx>,
    module: LocalModId,
    (default_export, default_declared): (Option<DefId>, bool),
    files: &HashMap<LocalModId, Vec<String>>,
    reexports: &[Reexport],
    aliases: &[(String, String)],
    (plain_ref_cells, counted): (&HashSet<Ty<'tcx>>, &super::analysis::Counted<'tcx>),
    (fns, held): (&HashMap<DefId, super::FnInfo>, &[super::bindings::HeldBinding]),
) -> Option<Value> {
    let mut out = Declarations {
        tcx,
        module,
        fns,
        files,
        plain_ref_cells,
        counted,
        imports: BTreeSet::new(),
        module_imports: BTreeSet::new(),
        unexported: BTreeMap::new(),
        erased: HashMap::new(),
        named: Vec::new(),
    };
    // What `js::export_default! { .. }` holds, a function or a constant, is
    // the module's default and nothing else, as its JS is, `export default
    // function noop`, whatever its visibility: declared where it is, as
    // the default, `export default function noop(): void;`, or a constant
    // declared, then exported, `declare const v: string; export default
    // v;`, as TypeScript has no default `const` (ADRs 0611, 0625).
    let held_default = default_export.filter(|_| default_declared);
    let mut items = Vec::new();
    for item in tcx.hir_module_free_items(module) {
        let def_id = item.owner_id.to_def_id();
        let held = held_default == Some(def_id);
        if !(held || tcx.visibility(def_id).is_public()) || is_mark(tcx, def_id) {
            continue;
        }
        out.erased = out.erased(def_id);
        let declaration = match tcx.def_kind(def_id) {
            DefKind::Fn if !is_binding(tcx, def_id) => Some(out.function(def_id)),
            DefKind::Struct if super::classes::is_class(tcx, def_id) => Some(out.class(def_id)),
            DefKind::Struct => out.structure(def_id),
            DefKind::Enum => Some(out.enumeration(def_id)),
            DefKind::Union => Some(out.union_type(def_id)),
            DefKind::TyAlias => Some(out.alias(def_id)),
            DefKind::Const { .. } | DefKind::Static { .. } if tcx.item_name(def_id).as_str() != "_" => {
                Some(out.constant(def_id))
            }
            _ => None,
        };
        let declaration = declaration.map(|mut declared| {
            if held {
                let function = tcx.def_kind(def_id) == DefKind::Fn;
                declared["exported"] = json!(function);
                declared["declare"] = json!(!function);
                if function {
                    declared["default"] = json!(true);
                }
            }
            declared
        });
        items.extend(declaration.map(|declaration| documented(declaration, tcx, def_id)));
        if tcx.def_kind(def_id) == DefKind::Enum {
            items.extend(out.variant_constants(def_id));
        }
    }
    // A binding it holds as a value, a global function's `typeof
    // Object.assign`, as TypeScript has it (ADR 0622), a static's its type
    // (ADR 0633); not one re-exported from another module that holds it.
    for held in held.iter().filter(|held| held.through.is_none()) {
        let mut declared = match tcx.def_kind(held.binding) {
            DefKind::Static { .. } => out.constant(held.binding),
            _ => json!({ "kind": "const", "type": { "kind": "typeof", "name": held.path } }),
        };
        declared["name"] = json!(held.name);
        declared["exported"] = json!(held.exported_by_name());
        declared["declare"] = json!(!held.exported_by_name());
        items.push(declared);
    }
    if let Some(def_id) = default_export {
        // One the naming form only exports, `js::export_default!(recap);`,
        // is declared, but exported by no name of its own, as `function
        // Recap() {..}` is in JS; a binding's is its `const`'s, declared
        // above.
        if held_default.is_none()
            && !tcx.visibility(def_id).is_public()
            && super::bindings::held_path(tcx, def_id).is_none()
        {
            out.erased = out.erased(def_id);
            let mut declared = match tcx.def_kind(def_id) {
                DefKind::Fn => out.function(def_id),
                _ => out.constant(def_id),
            };
            declared["exported"] = json!(false);
            declared["declare"] = json!(true);
            items.push(declared);
        }
        if held_default.is_none() || tcx.def_kind(def_id) != DefKind::Fn {
            items.push(json!({ "kind": "export-default", "name": fn_name(tcx, def_id) }));
        }
    }
    // What it names that it doesn't export, each declared here, not
    // exported, before what names it, as TypeScript has it: another crate's
    // untagged enums, `js::Json`, the union of their payloads, named, so it
    // can be recursive; and its own private types, as TypeScript declares an
    // interface an exported function names.
    items.splice(0..0, out.unexported.into_values().flatten());
    if items.is_empty() && reexports.is_empty() {
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
    // `export { Challenges } from "./Challenges.js";`, its `pub use` of another
    // module's function, as its JS has it (ADR 0240).
    // A JS module's by its specifier, which the output makes the file's too.
    for reexport in reexports {
        let from = match &reexport.from {
            crate::js::ImportFrom::Module(path) => json!({ "module": path }),
            crate::js::ImportFrom::Specifier(specifier) => json!({ "specifier": specifier }),
        };
        let mut declaration = json!({ "kind": "export", "names": reexport.named });
        declaration
            .as_object_mut()
            .expect("an object")
            .extend(from.as_object().expect("an object").clone());
        declarations.push(declaration);
    }
    declarations.extend(items);
    // `export { greetAll as unstable_greet };`, its `pub use` of its own
    // function by another name, after it, as its JS has it (ADR 0596).
    if !aliases.is_empty() {
        declarations.push(json!({ "kind": "export", "names": aliases }));
    }
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

/// Whether `did` is a JS value of any shape, TypeScript's `unknown` or
/// `any`, `js::Unknown`: what may be `0` or `""`, compared by `===` (ADR
/// 0612).
pub(super) fn any_shape(tcx: TyCtxt<'_>, did: DefId) -> bool {
    matches!(
        written_types(tcx, did).as_ref().and_then(Written::keyword),
        Some("unknown" | "any")
    )
}

/// What a type's name imports: the name, or a namespace's, `JSX` of
/// `JSX.Element` (ADR 0236).
fn imported(name: &str) -> String {
    name.split('.').next().unwrap_or(name).to_string()
}

/// A type the model names, with its arguments: `ReactNode`, `Omit<A, "b">`.
/// A keyword written, js's `Unknown`'s `unknown`, is the keyword, as
/// `#[rust_js::types]`'s reader has it (`written`).
fn reference(name: &str, args: Vec<Value>) -> Value {
    json!({ "kind": "reference", "name": name, "args": args })
}

fn keyword(keyword: &str) -> Value {
    json!({ "kind": "keyword", "keyword": keyword })
}

/// The union of `types`: TypeScript's `never` of none, a type with no
/// values, an enum of no variants, chrono's `enum Void {}`.
fn union(types: Vec<Value>) -> Value {
    if types.is_empty() {
        return keyword("never");
    }
    json!({ "kind": "union", "types": types })
}

/// `ty | undefined`, as @types/react writes an optional property; `any` and
/// `unknown` have it already.
fn or_undefined(ty: Value) -> Value {
    if ty["kind"] == "keyword" && (ty["keyword"] == "any" || ty["keyword"] == "unknown") {
        return ty;
    }
    let mut types = match ty["kind"].as_str() {
        Some("union") => ty["types"].as_array().cloned().unwrap_or_default(),
        _ => vec![ty],
    };
    types.push(keyword("undefined"));
    json!({ "kind": "union", "types": types })
}

fn is_any(ty: &Value) -> bool {
    ty["kind"] == "keyword" && ty["keyword"] == "any"
}

/// What a written name is of the type a `#[rust_js::types]` is of: one of
/// its type parameters, which it's given or not, or another type.
enum Given {
    Is(Value),
    Unknown,
    NotParam,
}

struct Declarations<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    module: LocalModId,
    /// Each function's and `const`'s JS name, as its JS has it: `new$` of a
    /// `new` (ADR 0546).
    fns: &'a HashMap<DefId, super::FnInfo>,
    /// The `RefCell`s a field holds as their value (ADR 0362).
    plain_ref_cells: &'a HashSet<Ty<'tcx>>,
    /// The `Rc`s counted, `{ value, strong, weak }`: every other, and its
    /// `Weak`, is the value it points at (ADRs 0320, 0843).
    counted: &'a super::analysis::Counted<'tcx>,
    /// The crate's modules that have a file, by their path: those whose
    /// types another's declarations import (ADR 0210).
    files: &'a HashMap<LocalModId, Vec<String>>,
    /// The types of JS modules' it names, `("react", "ReactNode")`, imported from them.
    imports: BTreeSet<(String, String)>,
    /// The types of the crate's other modules it names, by their path.
    module_imports: BTreeSet<(Vec<String>, String)>,
    /// What it names that it doesn't export, by name: another crate's
    /// untagged enums, and its own private types. Each one's declaration,
    /// `None` while it's being made, as a field or a payload names it.
    unexported: BTreeMap<String, Option<Value>>,
    /// The type parameters of the item being declared that are a type of
    /// TypeScript's, by index: `C: ReactNode`'s `ReactNode`.
    erased: HashMap<u32, Value>,
    /// The aliases written in the type being declared, each by its type,
    /// rustc's, which has them expanded, and what TypeScript names it
    /// (`as_written`).
    named: Vec<(Ty<'tcx>, Value)>,
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
        // An `impl Fn(&Item, usize) -> R` parameter is the function it says,
        // `(item: Item, index: number) => R`, as TypeScript writes a callback.
        let clauses = self.tcx.clauses_of(def_id).clauses;
        for (clause, _) in clauses.iter() {
            let Some(bound) = clause.as_trait_clause().map(|b| b.skip_binder()) else {
                continue;
            };
            let ty::Param(param) = bound.self_ty().kind() else {
                continue;
            };
            let synthetic = matches!(
                self.tcx
                    .generics_of(def_id)
                    .param_at(param.index as usize, self.tcx)
                    .kind,
                ty::GenericParamDefKind::Type { synthetic: true, .. }
            );
            if !synthetic
                || erased.contains_key(&param.index)
                || self.tcx.fn_trait_kind_from_def_id(bound.def_id()).is_none()
            {
                continue;
            }
            let ty::Tuple(inputs) = bound.trait_ref.args.type_at(1).kind() else {
                continue;
            };
            let output = (clauses.iter())
                .filter_map(|(clause, _)| clause.as_projection_clause())
                .map(|p| p.skip_binder())
                .find(|p| matches!(p.self_ty().kind(), ty::Param(q) if q.index == param.index))
                .and_then(|p| p.term.as_type())
                .unwrap_or(self.tcx.types.unit);
            let function = self.function_type(inputs.as_slice(), output);
            erased.insert(param.index, function);
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
    fn written(&mut self, did: DefId, declared: &Written, args: ty::GenericArgsRef<'tcx>) -> Value {
        // A name alone is of the Rust type's arguments: `MemoExoticComponent<P>`,
        // `react#NamedExoticComponent`, is a `NamedExoticComponent<P>`. Those
        // written are all of TypeScript's type's, `<>` none: react's
        // `JSX::Element<T>` is a `JSX.Element` whatever its tag's element
        // (ADR 0224).
        if let Written::Reference { from, name, args: None } = declared {
            self.imported_from(from.as_deref(), name);
            let type_args = args.types().map(|t| self.ts(t)).collect();
            return reference(name, type_args);
        }
        // A name written that's one of the Rust type's own type parameters
        // is what it's given: `Map<string, T>` of a `Dict<f64>` is
        // `Map<string, number>`, `{ [key: string]: T }` `{ [key: string]:
        // number }`.
        let params = &self.tcx.generics_of(did).own_params;
        let mut param = |out: &mut Self, name: &str| match params.iter().find(|p| p.name.as_str() == name) {
            Some(param) if (param.index as usize) < args.len() => Given::Is(out.ts(args.type_at(param.index as usize))),
            Some(_) => Given::Unknown,
            None => Given::NotParam,
        };
        // One of a parameter it isn't given, a trait's of a bound, is what a
        // type with no `#[rust_js::types]` is.
        (self.written_value(declared, &mut param, &mut Vec::new())).unwrap_or_else(|| keyword("any"))
    }

    /// `written` in the model, each name in it that's no type parameter of
    /// its own, `bound`'s, what `param` gives of it: `None` where it's a
    /// parameter `param` has no type for.
    fn written_value(
        &mut self,
        written: &Written,
        param: &mut dyn FnMut(&mut Self, &str) -> Given,
        bound: &mut Vec<String>,
    ) -> Option<Value> {
        Some(match written {
            Written::Keyword(word) => keyword(word),
            Written::Literal(value) => json!({ "kind": "literal", "value": value }),
            Written::Reference { from, name, args } => {
                if from.is_none() && args.is_none() && !bound.contains(name) {
                    match param(self, name) {
                        Given::Is(given) => return Some(given),
                        Given::Unknown => return None,
                        Given::NotParam => {}
                    }
                }
                self.imported_from(from.as_deref(), name);
                let args = (args.iter().flatten())
                    .map(|arg| self.written_value(arg, param, bound))
                    .collect::<Option<Vec<_>>>()?;
                reference(name, args)
            }
            Written::Union(types) => {
                let types = (types.iter())
                    .map(|ty| self.written_value(ty, param, bound))
                    .collect::<Option<Vec<_>>>()?;
                union(types)
            }
            Written::Array { element, readonly } => {
                let element = self.written_value(element, param, bound)?;
                json!({ "kind": "array", "element": element, "readonly": readonly })
            }
            Written::Function(signature) => {
                let (type_parameters, params, returns) = self.written_signature(signature, param, bound)?;
                json!({ "kind": "function", "typeParameters": type_parameters, "params": params, "returns": returns })
            }
            Written::Object(members) => {
                let mut values = Vec::new();
                for member in members {
                    values.push(match member {
                        Member::Property { name, optional, readonly, ty } => {
                            let ty = self.written_value(ty, param, bound)?;
                            json!({ "kind": "property", "name": name, "optional": optional, "readonly": readonly, "type": ty })
                        }
                        Member::Method { name, optional, signature } => {
                            let (type_parameters, params, returns) = self.written_signature(signature, param, bound)?;
                            json!({ "kind": "method", "name": name, "optional": optional,
                                "typeParameters": type_parameters, "params": params, "returns": returns })
                        }
                        Member::Index { parameter, key, ty } => {
                            let key = self.written_value(key, param, bound)?;
                            let ty = self.written_value(ty, param, bound)?;
                            json!({ "kind": "index", "parameter": parameter, "key": key, "type": ty })
                        }
                    });
                }
                json!({ "kind": "object", "members": values })
            }
        })
    }

    /// A written function's or method's type parameters, its own, which no
    /// name in it of theirs is the Rust type's, its parameters and result.
    fn written_signature(
        &mut self,
        signature: &Signature,
        param: &mut dyn FnMut(&mut Self, &str) -> Given,
        bound: &mut Vec<String>,
    ) -> Option<(Vec<Value>, Vec<Value>, Value)> {
        let outer = bound.len();
        bound.extend(signature.type_parameters.iter().cloned());
        let mut signed = || {
            let mut params = Vec::new();
            for p in &signature.params {
                let ty = self.written_value(&p.ty, param, bound)?;
                params.push(json!({ "name": p.name, "optional": p.optional, "rest": p.rest, "type": ty }));
            }
            let returns = self.written_value(&signature.returns, param, bound)?;
            Some((params, returns))
        };
        let signed = signed();
        bound.truncate(outer);
        let (params, returns) = signed?;
        let type_parameters = (signature.type_parameters.iter())
            .map(|name| json!({ "name": name }))
            .collect();
        Some((type_parameters, params, returns))
    }

    /// A type named of a module, `react#ReactNode`, imported from it; a
    /// namespace's member, `JSX.Element`, the namespace.
    fn imported_from(&mut self, from: Option<&str>, name: &str) {
        if let Some(from) = from {
            self.imports.insert((from.to_string(), imported(name)));
        }
    }

    /// A type as the Rust writes it, `of` making it of rustc's: each alias
    /// written in it by its name, as TypeScript has it, where rustc's type
    /// has it expanded, the crate's own, `Context<Toc>`, or one that says
    /// what it is to TypeScript, `MouseEventHandler<HTMLButtonElement>`.
    fn as_written(&mut self, written: Option<&hir::Ty<'tcx>>, of: impl FnOnce(&mut Self) -> Value) -> Value {
        let mut named = Vec::new();
        if let Some(written) = written {
            self.aliases_in(written, &mut named);
        }
        let outer = self.named.len();
        self.named.extend(named);
        let value = of(self);
        self.named.truncate(outer);
        value
    }

    /// Each alias `written` names that TypeScript has by its name, and that.
    fn aliases_in(&mut self, written: &hir::Ty<'tcx>, named: &mut Vec<(Ty<'tcx>, Value)>) {
        match &written.kind {
            hir::TyKind::Path(hir::QPath::Resolved(None, path)) => {
                if let Res::Def(DefKind::TyAlias, did) = path.res
                    && let Some(value) = self.alias_named(written, path, did)
                {
                    let ty = rustc_hir_analysis::lower_ty(self.tcx, written);
                    named.push((self.tcx.erase_and_anonymize_regions(ty), value));
                    return;
                }
                for arg in path.segments.iter().filter_map(|s| s.args).flat_map(|args| args.args) {
                    if let hir::GenericArg::Type(ty) = arg {
                        self.aliases_in(ty.as_unambig_ty(), named);
                    }
                }
            }
            hir::TyKind::Ref(_, inner) | hir::TyKind::Ptr(inner) => self.aliases_in(inner.ty, named),
            hir::TyKind::Slice(inner) | hir::TyKind::Array(inner, _) => self.aliases_in(inner, named),
            hir::TyKind::Tup(items) => {
                for item in *items {
                    self.aliases_in(item, named);
                }
            }
            _ => {}
        }
    }

    /// An alias written, `did`, by its name: one that says what it is to
    /// TypeScript, or one of the crate's it declares, this module's or
    /// another's, imported from it (ADR 0210), of its arguments as written.
    fn alias_named(&mut self, written: &hir::Ty<'tcx>, path: &hir::Path<'tcx>, did: DefId) -> Option<Value> {
        if written_types(self.tcx, did).is_some() {
            return self.written_alias(written);
        }
        let local = did.as_local()?;
        if !self.tcx.visibility(did).is_public() {
            return None;
        }
        let home = self.tcx.parent_module_from_def_id(local);
        let name = self.tcx.item_name(did).to_string();
        if home != self.module {
            let file = self.files.get(&home)?;
            self.module_imports.insert((file.clone(), name.clone()));
        }
        let given: Vec<&hir::Ty<'tcx>> = (path.segments.last()?.args.map(|args| args.args).unwrap_or_default())
            .iter()
            .filter_map(|arg| match arg {
                hir::GenericArg::Type(ty) => Some(ty.as_unambig_ty()),
                _ => None,
            })
            .collect();
        let args = (given.into_iter())
            .map(|given| {
                let ty = rustc_hir_analysis::lower_ty(self.tcx, given);
                self.as_written(Some(given), |out| out.ts(ty))
            })
            .collect();
        Some(reference(&name, args))
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
        let at = |name: &str| params.iter().position(|param| param.as_str() == name);
        // Each of its parameters is what's written of it; one left to its
        // default, none written.
        let mut param = |out: &mut Self, name: &str| match at(name) {
            Some(at) => match given.get(at) {
                Some(given) => {
                    let ty = rustc_hir_analysis::lower_ty(out.tcx, given);
                    Given::Is(out.as_written(Some(given), |out| out.ts(ty)))
                }
                None => Given::Unknown,
            },
            None => Given::NotParam,
        };
        // Its arguments end where one's left to its default, as TypeScript's
        // has the same: `MouseEventHandler` of `T = Element`. Where one's
        // left so in another place, it's as rustc has it, expanded.
        if let Written::Reference {
            from,
            name,
            args: Some(wanted),
        } = &declared
        {
            let mut args = Vec::new();
            for wanted in wanted {
                if let Written::Reference {
                    from: None,
                    name,
                    args: None,
                } = wanted
                    && at(name).is_some_and(|at| at >= given.len())
                {
                    break;
                }
                args.push(self.written_value(wanted, &mut param, &mut Vec::new())?);
            }
            self.imported_from(from.as_deref(), name);
            return Some(reference(name, args));
        }
        self.written_value(&declared, &mut param, &mut Vec::new())
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
    /// A function's or a `const`'s name, as its JS has it.
    fn js_name(&self, def_id: DefId) -> String {
        (self.fns.get(&def_id)).map_or_else(|| fn_name(self.tcx, def_id), |info| info.name.clone())
    }

    fn function(&mut self, def_id: DefId) -> Value {
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
        let is_option =
            |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.is_lang_item(adt.did(), LangItem::Option));
        // Its last, a slice marked `#[rust_js::rest]`, is a rest parameter,
        // `...args: T[]`, which those before it may still be left out before
        // (ADR 0664).
        let rest = super::bindings::takes_rest(self.tcx, def_id).then(|| sig.inputs().len() - 1);
        let before = &sig.inputs()[..rest.unwrap_or(sig.inputs().len())];
        let optional_from = before.len() - before.iter().rev().take_while(|&&ty| is_option(ty)).count();
        let params: Vec<Value> = (sig.inputs().iter().enumerate())
            .map(|(i, &ty)| {
                // As its JS spells it, a word JS keeps given apart, `new$`.
                let name = match idents.get(i).copied().flatten() {
                    // Its JS name's, without a `_` of one unused (ADR 0678).
                    Some(ident) => {
                        let named = super::parameter_name(self.tcx, def_id, ident.name.as_str());
                        crate::names::js_ident(&named)
                    }
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let ty = self.as_written(decl.and_then(|decl| decl.inputs.get(i)), |out| out.ts(ty));
                let rest = rest == Some(i);
                json!({ "name": name, "optional": i >= optional_from && !rest, "rest": rest, "type": ty })
            })
            .collect();
        let output = sig.output();
        let written = decl.and_then(|decl| match decl.output {
            hir::FnRetTy::Return(written) => Some(written),
            hir::FnRetTy::DefaultReturn(_) => None,
        });
        let returns = match output.is_unit() {
            true => keyword("void"),
            false => self.as_written(written, |out| out.ts(output)),
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

    /// `export declare class Component { constructor(props?: unknown);
    /// props?: unknown; setState(partialState?: unknown): void; }`, a JS
    /// class's (ADR 0761): its fields `this`'s, its constructor `new`'s,
    /// its constants and methods its prototype's, and its statics; each
    /// its impl's that's `pub`.
    fn class(&mut self, def_id: DefId) -> Value {
        use super::classes::{Member, member};
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
                    None => continue,
                };
                members.push(documented(declared, self.tcx, item));
            }
        }
        members.splice(0..0, constructor);
        json!({
            "kind": "class",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": true,
            "members": members,
        })
    }

    /// `export interface TagProps { variant: RouteTag; text?: string; }`
    fn structure(&mut self, def_id: DefId) -> Option<Value> {
        let adt = self.tcx.adt_def(def_id);
        let variant = adt.non_enum_variant();
        // A `#[repr(transparent)]` struct is its field (ADR 0584): a type of
        // that field's, `type Meters = number`.
        if let Some(at) = super::recognition::transparent_field(self.tcx, adt) {
            let field = self
                .tcx
                .type_of(variant.fields.raw[at].did)
                .instantiate_identity()
                .skip_normalization();
            return Some(json!({
                "kind": "type",
                "name": self.tcx.item_name(def_id).as_str(),
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
            // A nullable `Cell`'s, the `Option` it holds, the field (ADR 0772);
            // a `OnceCell`'s, absent until it's set (ADR 0847).
            let option = match is_nullable(self.tcx, field) {
                true => super::bindings::nullable_option(self.tcx, ty).and_then(|option| self.option(option)),
                false => self.option(ty).or_else(|| self.once_cell_item(ty)),
            };
            let (optional, ty) = match option {
                Some(inner) => {
                    let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                    match is_nullable(self.tcx, field) {
                        true => (true, json!({ "kind": "union", "types": [inner, keyword("null")] })),
                        false => (true, inner),
                    }
                }
                None => (
                    field_default(self.tcx, field).is_some(),
                    self.as_written(written, |out| out.field_ts(ty)),
                ),
            };
            // One a caller may leave out takes `undefined` too, as leaving it
            // out does, under `exactOptionalPropertyTypes` as without it.
            let ty = if optional { or_undefined(ty) } else { ty };
            let property = json!({
                "kind": "property",
                "name": field_key(self.tcx, field),
                "optional": optional,
                "readonly": false,
                "type": ty,
            });
            members.push(documented(property, self.tcx, field.did));
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
        let ty = self.as_written(self.written_type(def_id), |out| out.ts(ty));
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": ty,
        })
    }

    fn enumeration(&mut self, def_id: DefId) -> Value {
        let adt = self.tcx.adt_def(def_id);
        let untagged = is_untagged(self.tcx, def_id);
        let union = match untagged {
            // `export type Size = string | number;` of an untagged enum, its
            // payloads' union (ADR 0214).
            true => {
                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
                let types: Vec<Value> = (adt.variants().iter())
                    .map(|v| match v.fields.iter().next() {
                        Some(field) => self.ts(field.ty(self.tcx, args).skip_normalization()),
                        // One without fields, its name's string, or `null` or
                        // `undefined` (ADR 0676).
                        None => match super::untagged::unit_value(self.tcx, v).map(|e| e.kind) {
                            Some(crate::js::ExprKind::Null) => keyword("null"),
                            Some(crate::js::ExprKind::Undefined) => keyword("undefined"),
                            _ => json!({ "kind": "literal", "value": variant_name(self.tcx, v) }),
                        },
                    })
                    .collect();
                union(types)
            }
            // `{ TAG: "Circle"; _0: number } | "Empty"`, each variant as rust-js
            // makes it (ADR 0033), as ReScript's genType declares one, one of
            // no fields its name; of a discriminated union, each an object of
            // its tag, one of no fields too (ADR 0284).
            false => {
                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
                let key = tag_key(self.tcx, def_id);
                let tagged = declared_tag(self.tcx, def_id).is_some();
                let types: Vec<Value> = (adt.variants().iter())
                    .map(|v| {
                        // An `otherwise`, the object it holds (ADR 0284).
                        if tagged
                            && is_otherwise(self.tcx, v.def_id)
                            && let Some(field) = v.fields.iter().next()
                        {
                            return self.ts(field.ty(self.tcx, args).skip_normalization());
                        }
                        let name = self.variant_literal(v);
                        if v.fields.is_empty() && !tagged {
                            return name;
                        }
                        let tag = json!({ "kind": "property", "name": key, "optional": false, "readonly": false, "type": name });
                        let fields = v.fields.iter().enumerate().map(|(i, field)| {
                            let ty = self.field_ts(field.ty(self.tcx, args).skip_normalization());
                            let name = crate::lower::representation::variant_field(self.tcx, v, i);
                            json!({ "kind": "property", "name": name, "optional": false, "readonly": false, "type": ty })
                        });
                        json!({ "kind": "object", "members": std::iter::once(tag).chain(fields).collect::<Vec<_>>() })
                    })
                    .collect();
                union(types)
            }
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

    /// A variant's value as its literal type, `"Empty"`, `0` or `true`.
    fn variant_literal(&self, variant: &ty::VariantDef) -> Value {
        let value = match super::bindings::variant_tag(self.tcx, variant).kind {
            crate::js::ExprKind::Bool(b) => json!(b),
            crate::js::ExprKind::Num(n) => json!(n as i64),
            _ => json!(variant_name(self.tcx, variant)),
        };
        json!({ "kind": "literal", "value": value })
    }

    /// Each variant of a numbered enum, its module's `const`, `export const
    /// Pending: 0;`, as its JS declares it (ADR 0722, amended).
    fn variant_constants(&self, def_id: DefId) -> Vec<Value> {
        let adt = self.tcx.adt_def(def_id);
        if !super::bindings::numbered(self.tcx, adt) {
            return Vec::new();
        }
        (adt.variants().iter())
            .map(|v| {
                let declared = json!({
                    "kind": "const",
                    "name": self.js_name(v.def_id),
                    "exported": true,
                    "declare": false,
                    "type": self.variant_literal(v),
                });
                documented(declared, self.tcx, v.def_id)
            })
            .collect()
    }

    /// `export type Number = { int: number } | { float: number };` of a
    /// union, the object of the one field it holds (ADR 0406).
    fn union_type(&mut self, def_id: DefId) -> Value {
        let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);
        let fields: Vec<_> = self.tcx.adt_def(def_id).non_enum_variant().fields.iter().collect();
        let types: Vec<Value> = (fields.into_iter())
            .map(|field| {
                let ty = self.field_ts(field.ty(self.tcx, args).skip_normalization());
                let member = json!({ "kind": "property", "name": field_key(self.tcx, field), "optional": false, "readonly": false, "type": ty });
                json!({ "kind": "object", "members": [member] })
            })
            .collect();
        json!({
            "kind": "type",
            "name": self.tcx.item_name(def_id).as_str(),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "type": union(types),
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
        let ty = self.as_written(self.written_type(def_id), |out| out.ts(ty));
        json!({
            "kind": "const",
            "name": self.js_name(def_id),
            "exported": true,
            "declare": false,
            "type": ty,
        })
    }

    /// An item's type as the Rust writes it, a `const`'s or an alias's.
    fn written_type(&self, def_id: DefId) -> Option<&'tcx hir::Ty<'tcx>> {
        self.tcx.hir_node_by_def_id(def_id.as_local()?).ty()
    }

    /// What a `OnceCell` or a `OnceLock` holds: in a field, the property,
    /// absent until it's set (ADR 0847).
    fn once_cell_item(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::OnceCell) => Some(args.type_at(0)),
            _ => None,
        }
    }

    fn option(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.kind() {
            ty::Adt(adt, args) if self.tcx.is_lang_item(adt.did(), LangItem::Option) => Some(args.type_at(0)),
            _ => None,
        }
    }

    /// `(event: MouseEvent<Element>) => void`: a function type, each
    /// parameter named by its type, as a person names one.
    /// Its last parameter of rest arguments as a value, `&dyn js::Rest<T>`,
    /// a rest parameter of what each reads as, `...args: unknown[]` (ADR
    /// 0851).
    fn function_type(&mut self, inputs: &[Ty<'tcx>], output: Ty<'tcx>) -> Value {
        let mut names: Vec<String> = Vec::new();
        let count = inputs.len();
        let params: Vec<Value> = (inputs.iter().enumerate())
            .map(|(i, &input)| {
                if i + 1 == count
                    && let Some(items) = super::bindings::rest_items(self.tcx, input)
                {
                    return json!({ "name": "args", "optional": false, "rest": true, "type": self.ts(items) });
                }
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
    /// A field's type: of a `Cell`, what it holds, as a `Cell` in a field
    /// is the property set in place (ADR 0288), and of a `RefCell` never
    /// counted (ADR 0362); of a `OnceCell`, what it holds or `undefined`
    /// (ADR 0847).
    fn field_ts(&mut self, ty: Ty<'tcx>) -> Value {
        if let Some(item) = self.once_cell_item(ty) {
            let item = self.ts(item);
            return or_undefined(item);
        }
        match ty.kind() {
            ty::Adt(adt, args)
                if is_std_def(self.tcx, adt.did(), StdItem::Cell)
                    || super::drops::is_plain_ref_cell(self.plain_ref_cells, self.tcx, ty) =>
            {
                self.ts(args.type_at(0))
            }
            _ => self.ts(ty),
        }
    }

    fn ts(&mut self, ty: Ty<'tcx>) -> Value {
        let tcx = self.tcx;
        // An alias the Rust writes, by its name (`as_written`).
        let erased = tcx.erase_and_anonymize_regions(ty);
        if let Some((_, named)) = self.named.iter().rev().find(|(named, _)| *named == erased) {
            return named.clone();
        }
        if let Some(num) = Num::of(ty) {
            return keyword(if num.big() { "bigint" } else { "number" });
        }
        match ty.kind() {
            ty::Bool => keyword("boolean"),
            ty::Char | ty::Str => keyword("string"),
            ty::Ref(_, inner, _) => self.ts(*inner),
            // What a function that never returns returns (ADR 0386).
            ty::Never => keyword("never"),
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
            // A `dyn` of a trait that says what it is to TypeScript:
            // `dyn ReactNode` is a `ReactNode` (ADR 0242).
            ty::Dynamic(traits, ..)
                if let Some(t) = traits.principal_def_id()
                    && let Some(declared) = written_types(tcx, t) =>
            {
                self.written(t, &declared, ty::List::empty())
            }
            // An `impl Fn(A) -> R` a function gives: the function it says, as
            // a parameter's is, `cache`'s `(...args: unknown[]) => unknown`.
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) if let Some((inputs, output)) = opaque_fn_signature(tcx, *alias) => self.function_type(&inputs, output),
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
                // An uncounted `Rc` is what it points at, and its `Weak` that
                // or `null`, `Weak::new()`'s (ADRs 0023, 0843).
                if let Some(pointee) = super::recognition::rc_pointee(tcx, ty)
                    && !self.counted.counts(tcx, pointee)
                {
                    return self.ts(pointee);
                }
                if let Some(pointee) = super::recognition::weak_pointee(tcx, ty)
                    && !self.counted.counts(tcx, pointee)
                {
                    return json!({ "kind": "union", "types": [self.ts(pointee), keyword("null")] });
                }
                // A `Cell` of its own, or one lent, is `{ value }` (ADRs 0023, 0288).
                if is_std_def(tcx, did, StdItem::Cell) {
                    let value = json!({ "kind": "property", "name": "value", "optional": false, "readonly": false, "type": self.ts(args.type_at(0)) });
                    return json!({ "kind": "object", "members": [value] });
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
                    if !self.unexported.contains_key(&name) {
                        self.unexported.insert(name.clone(), None);
                        let inner = self.erased(did);
                        let outer = std::mem::replace(&mut self.erased, inner);
                        let mut declared = self.enumeration(did);
                        self.erased = outer;
                        declared["exported"] = json!(false);
                        self.unexported.insert(name.clone(), Some(declared));
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
                    DefKind::Struct => {
                        adt.non_enum_variant().ctor.is_none()
                            || super::recognition::transparent_field(tcx, *adt).is_some()
                    }
                    DefKind::Enum | DefKind::Union => true,
                    _ => false,
                };
                if !declared {
                    return keyword("any");
                }
                let home = tcx.parent_module_from_def_id(local);
                // This module's own private type, `struct DiagramGroupProps`,
                // which its exported function names: declared here, not
                // exported (ADR 0385). Another module's isn't its to declare.
                if !tcx.visibility(did).is_public() {
                    if home != self.module {
                        return keyword("any");
                    }
                    let name = tcx.item_name(did).to_string();
                    if !self.unexported.contains_key(&name) {
                        self.unexported.insert(name.clone(), None);
                        let inner = self.erased(did);
                        let outer = std::mem::replace(&mut self.erased, inner);
                        let declared = match tcx.def_kind(did) {
                            DefKind::Struct => self.structure(did),
                            DefKind::Union => Some(self.union_type(did)),
                            _ => Some(self.enumeration(did)),
                        };
                        self.erased = outer;
                        let Some(mut declared) = declared else {
                            self.unexported.remove(&name);
                            return keyword("any");
                        };
                        declared["exported"] = json!(false);
                        self.unexported.insert(name.clone(), Some(declared));
                    }
                    let type_args = self.type_args(did, args);
                    return reference(&name, type_args);
                }
                // Another module's is imported from its file; one of a module
                // without one, of types only, has nowhere it's declared.
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

/// What an `impl Fn(A, B) -> R` takes and gives, by its bounds, `Fn`'s and
/// its `Output`, of `alias`'s arguments: `None` of another `impl Trait`.
fn opaque_fn_signature<'tcx>(tcx: TyCtxt<'tcx>, alias: ty::AliasTy<'tcx>) -> Option<(Vec<Ty<'tcx>>, Ty<'tcx>)> {
    let mut inputs = None;
    let mut output = tcx.types.unit;
    let ty::Opaque { def_id } = alias.kind else { return None };
    let bounds = tcx
        .explicit_item_bounds(def_id)
        .iter_instantiated_copied(tcx, alias.args);
    for (clause, _) in bounds.map(|bound| bound.skip_normalization()) {
        if let Some(bound) = clause.as_trait_clause().map(|b| b.skip_binder())
            && tcx.fn_trait_kind_from_def_id(bound.def_id()).is_some()
            && let ty::Tuple(items) = bound.trait_ref.args.type_at(1).kind()
        {
            inputs = Some(items.to_vec());
        }
        if let Some(projection) = clause.as_projection_clause().map(|p| p.skip_binder())
            && tcx.is_lang_item(projection.def_id(), LangItem::FnOnceOutput)
            && let Some(given) = projection.term.as_type()
        {
            output = given;
        }
    }
    Some((inputs?, output))
}

/// The JS names of `ty`'s fields, a flattened struct's, or one's it
/// refers to, and of those of the flattened struct in it (ADR 0205).
fn flattened_keys<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<String> {
    let ty::Adt(adt, args) = ty.peel_refs().kind() else {
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

/// `declaration` with its item's doc comment, as JSDoc (ADR 0444), which a
/// TypeScript caller's editor shows: a function's, a type's, a field's.
fn documented(mut declaration: Value, tcx: TyCtxt<'_>, def_id: DefId) -> Value {
    use rustc_hir::attrs::AttributeKind;
    let docs: Vec<String> = (rustc_hir::attrs::HasAttrs::get_attrs(def_id, &tcx).iter())
        .filter_map(|attr| match attr {
            hir::Attribute::Parsed(AttributeKind::DocComment { comment, .. }) => Some(comment.to_string()),
            _ => None,
        })
        .flat_map(|doc| doc.lines().map(str::to_string).collect::<Vec<_>>())
        .collect();
    let lines: Vec<&str> = docs.iter().map(String::as_str).collect();
    if let Some(jsdoc) = crate::comments::jsdoc(&lines).filter(|_| !lines.is_empty()) {
        declaration["docs"] = jsdoc.into();
    }
    declaration
}

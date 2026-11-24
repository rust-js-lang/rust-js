//! A module's TypeScript declarations, `Tag.d.ts` beside its JS, for the
//! TypeScript that imports it (ADR 0196): what it exports, as Rust types it,
//! each by the name it has in JS. An `Option` field is optional, a unit-only
//! enum its names, a `memo`'s component its props. What has no type here,
//! another module's type or a JS object's, is `any`, so a caller isn't held
//! to less than Rust holds it to. They're the model of @rust-js/typescript
//! (ADR 0206), which TypeScript prints (ADR 0207).

use rust_js_program::collections::{IndexMap, IndexSet};
use std::collections::{BTreeMap, BTreeSet};

use rust_js_program::program::Reexport;
use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalModId};
use serde_json::{Value, json};

use super::bindings::{fn_name, is_binding, is_mark};
use super::recognition::{StdItem, is_std_def};

mod aliases;
mod bounds;
mod cells;
mod classes;
mod comments;
pub(crate) mod contract;
mod dictionaries;
pub(super) use dictionaries::trait_types;
mod enums;
mod functions;
mod globals;
mod imports;
mod namespaces;
mod parts;
mod projections;
mod reach;
pub(crate) use reach::{js_holds, js_names};
mod representation;
pub(super) mod std_types;
mod structures;
pub use type_maps::is_type_map;
mod type_maps;
mod written;

use super::representation::{SignatureForms, held_ts as held};
use bounds::{constraints, erasures};
use classes::exported_class;
use comments::documented;
pub(crate) use comments::jsdoc_of;
use contract::Contract;
pub(crate) use dictionaries::declared_methods;

pub use written::validate_types;
use written::{Member, Signature};
pub use written::{Written, written_types};

/// What a module's `.d.ts` declares, but its header: `None` where it
/// exports nothing.
#[allow(clippy::too_many_arguments)]
pub fn module<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &super::library::Foreign<'_, 'tcx>,
    module: LocalModId,
    (default_export, default_declared): (Option<DefId>, bool),
    files: &IndexMap<LocalModId, Vec<String>>,
    (reexports, forwarded): (&[(rustc_span::Span, Reexport)], &IndexMap<DefId, DefId>),
    aliases: &[(DefId, (String, String))],
    (ref_cells, counted, null_fields, flattened): (
        &super::drops::RefCells<'tcx>,
        &super::analysis::Counted<'tcx>,
        &IndexSet<DefId>,
        &super::bindings::Flattened,
    ),
    (fns, held, variables): (
        &IndexMap<DefId, super::FnInfo>,
        &[super::bindings::HeldBinding],
        &IndexMap<DefId, bool>,
    ),
    representation: SignatureForms<'_, 'tcx>,
) -> Option<Value> {
    let mut out = Declarations {
        tcx,
        representation,
        owner: module.to_def_id(),
        foreign,
        module,
        fns,
        variables,
        files,
        ref_cells,
        counted,
        null_fields,
        flattened,
        imports: BTreeSet::new(),
        library_imports: BTreeSet::new(),
        module_imports: BTreeSet::new(),
        namespaces: BTreeSet::new(),
        unexported: BTreeMap::new(),
        erased: IndexMap::default(),
        named: Vec::new(),
        function_names: Vec::new(),
        filled: None,
        notes: Vec::new(),
    };
    // What `js::export_default! { .. }` holds, a function or a constant, is
    // the module's default and nothing else, as its JS is, `export default
    // function noop`, whatever its visibility: declared where it is, as
    // the default, `export default function noop(): void;`, or a constant
    // declared, then exported, `declare const v: string; export default
    // v;`, as TypeScript has no default `const` (ADRs 0611, 0625).
    let held_default = default_export.filter(|_| default_declared);
    // What the naming form makes the default, `js::export_default!(page);`,
    // is its default alone too, declared below (ADR 0611, amended).
    let named_default = default_export.filter(|&d| !default_declared && super::bindings::held_path(tcx, d).is_none());
    let mut items = Vec::new();
    let mut objects = IndexSet::default();
    let mut left_out = IndexSet::default();
    for item in tcx.hir_module_free_items(module) {
        // The notes of what the item before left out, after it (#559).
        items.append(&mut out.notes);
        let def_id = item.owner_id.to_def_id();
        // A class whose struct is another module's, declared where its
        // constructor function is (ADR 0761, amended).
        if let Some(class) = super::classes::declared_by(tcx, def_id, module)
            .filter(|&class| class != def_id && tcx.visibility(class).is_public())
        {
            out.enter(class);
            items.extend(exported_class(tcx, documented(out.class(class), tcx, class), class));
            continue;
        }
        // An impl of another module's type, its object of methods here, as
        // its JS has it (ADR 0047, #507).
        if let Some(adt) = out.foreign_impl_type(def_id) {
            if objects.insert(adt) {
                items.extend(out.type_object(adt));
            }
            continue;
        }
        // An impl's dictionary its module exports, of a trait declared as
        // the type of its dictionaries (#550).
        if let Some(dictionary) = out.impl_dictionary(def_id) {
            items.push(dictionary);
            continue;
        }
        let held = held_default == Some(def_id);
        if !(held || tcx.visibility(def_id).is_public()) || is_mark(tcx, def_id) || named_default == Some(def_id) {
            continue;
        }
        out.enter(def_id);
        // A module's namespace, no type: its module's functions (ADR 1037).
        if super::classes::is_namespace(tcx, def_id) {
            items.extend(out.namespace_functions(def_id, forwarded));
            continue;
        }
        if super::classes::is_value_namespace(tcx, def_id) {
            items.extend(out.namespace_fields(def_id));
            continue;
        }
        // One whose JS takes what a JS caller can't give, `size(TSize)`, is
        // a note where it would be, not declared (#549, ADR 1077).
        if tcx.def_kind(def_id) == DefKind::Fn
            && !is_binding(tcx, def_id)
            && out.leave_out(def_id, &out.js_name(def_id))
        {
            items.append(&mut out.notes);
            left_out.insert(def_id);
            continue;
        }
        let declaration = match tcx.def_kind(def_id) {
            DefKind::Fn if !is_binding(tcx, def_id) => Some(out.function(def_id)),
            DefKind::Struct if super::classes::is_class(tcx, def_id) => {
                super::classes::declared_by(tcx, def_id, module).map(|class| out.class(class))
            }
            DefKind::Struct => out.struct_declaration(def_id),
            DefKind::Enum => Some(out.enumeration(def_id)),
            DefKind::Union => Some(out.union_type(def_id)),
            DefKind::TyAlias => Some(out.alias(def_id)),
            DefKind::Trait => out.trait_interface(def_id),
            DefKind::Const | DefKind::Static { .. } if tcx.item_name(def_id).as_str() != "_" => {
                Some(out.constant(def_id))
            }
            _ => None,
        };
        let declaration = declaration.map(|mut declared| {
            // What it exports by another name, by that alone, `declare
            // function getCurrentTime(): number;`, as its JS (ADR 0596,
            // amended).
            if !held && let Some((_, (_, export))) = aliases.iter().find(|&&(item, _)| item == def_id) {
                match by_own_name(fns, def_id, export) {
                    true => declared["name"] = json!(export),
                    false => {
                        declared["exported"] = json!(false);
                        declared["declare"] = json!(true);
                    }
                }
            }
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
        let declaration = declaration.map(|declaration| documented(declaration, tcx, def_id));
        match declaration {
            Some(class) if class["kind"] == "class" && !held => items.extend(exported_class(tcx, class, def_id)),
            declaration => items.extend(declaration),
        }
        if tcx.def_kind(def_id) == DefKind::Enum {
            items.extend(out.variant_constants(def_id));
        }
        // Its object of constants and methods (#498, #507), a value beside
        // its type.
        if matches!(tcx.def_kind(def_id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            items.extend(out.type_object(def_id));
        }
        // A trait's impls of named unit structs as one type (ADR 1164).
        if tcx.def_kind(def_id) == DefKind::Trait {
            items.extend(out.type_map(def_id));
        }
    }
    // A binding it holds as a value, a global function's `typeof
    // Object.assign`, as TypeScript has it (ADR 0622), a static's its type
    // (ADR 0633); not one re-exported from another module that holds it.
    // Not one a private `use` renames, `use js::array::SLICE as slice;`,
    // which no export names: no part of the module's type (rd517).
    for held in (held.iter()).filter(|held| held.through.is_none() && (held.export || held.default)) {
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
        // One the naming form exports, `js::export_default!(recap);`, is
        // declared, but exported by no name of its own, `pub` or not, as
        // `function Recap() {..}` is in JS (ADR 0611, amended); a binding's
        // is its `const`'s, declared above.
        if named_default == Some(def_id) {
            out.enter(def_id);
            let mut declared = match tcx.def_kind(def_id) {
                DefKind::Fn => out.function(def_id),
                // A class, its constructor (ADR 0761).
                DefKind::Struct => out.class(def_id),
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
    items.append(&mut out.notes);
    // What it names that it doesn't export, each declared here, not
    // exported, before what names it, as TypeScript has it: another crate's
    // untagged enums, `js::Json`, the union of their payloads, named, so it
    // can be recursive; and its own private types, as TypeScript declares an
    // interface an exported function names.
    items.splice(0..0, out.unexported.into_values().flatten());
    // What its `pub use`s of types re-export, `export type { .. } from`.
    let types = super::reexports::type_reexports(tcx, module, foreign, files);
    if items.is_empty() && reexports.is_empty() && types.is_empty() {
        return None;
    }
    let imported = (&out.imports, &out.library_imports, &out.module_imports);
    let mut declarations = imports::imports(tcx, imported, (module, reexports, &types));
    declarations.splice(0..0, imports::namespaces(&out.namespaces));
    declarations.extend(items);
    // `export { greetAll as unstable_greet };`, its `pub use` of its own
    // function by another name, after it, as its JS has it (ADR 0596).
    let names: Vec<&(String, String)> = (aliases.iter())
        .filter(|(item, (_, export))| !by_own_name(fns, *item, export) && !left_out.contains(item))
        .map(|(_, names)| names)
        .collect();
    if !names.is_empty() {
        declarations.push(json!({ "kind": "export", "names": names }));
    }
    Some(json!({ "declarations": declarations }))
}

/// Whether an export its module names apart, `console$` of a global its
/// JS reads, is declared by its own name, `export function console():
/// number;`, which a `.d.ts`, reading no global, may: but a word JS keeps,
/// `delete`, which no declaration may be named, exported by its alias
/// (#338).
fn by_own_name(fns: &IndexMap<DefId, super::FnInfo>, item: DefId, export: &str) -> bool {
    fns.get(&item).and_then(|info| info.export.as_deref()) == Some(export) && !rust_js_program::names::kept_word(export)
}

/// Whether the declarations of `did`'s module declare it, a struct, an
/// enum or a union of the crate's another module's or a library's consumer
/// names by its name (ADR 0210, amended): an enum, a union, a struct
/// (`structures::declared`, each that holds a value, ADR 1187); and an
/// alias, a library's plain one
/// too, as the crate's own is (rd642). One whose `#[rust_js::types]` names
/// a type is declared as well, but is that type where it's written
/// (`aliases::written_alias`).
pub(super) fn declares_type(tcx: TyCtxt<'_>, did: DefId) -> bool {
    match tcx.def_kind(did) {
        DefKind::Struct => structures::declared(tcx, did),
        DefKind::Enum | DefKind::Union => true,
        DefKind::TyAlias => true,
        _ => false,
    }
}

/// The name a crate's type is declared by, its export's and every
/// reference's: the one its `#[rust_js::name]` gives, a class's JS name,
/// `FiberNode` of `Fiber`, or an alias's, `#[rust_js::name = "ISODate"] pub
/// type IsoDate = String;`'s `ISODate` (#1829), else its Rust name.
pub(crate) fn type_name(tcx: TyCtxt<'_>, did: DefId) -> String {
    // A unit struct's `#[rust_js::name]` is its value, not its type's name.
    match named_unit(tcx, did) {
        Some(_) => tcx.item_name(did).to_string(),
        None => super::bindings::given_name(tcx, did).unwrap_or_else(|| tcx.item_name(did).to_string()),
    }
}

/// A unit struct named as a string or a boolean, `#[rust_js::name =
/// "trigger-press"]`, `#[rust_js::name = true]`, whose value and type are
/// that literal (ADRs 0013, 1164): the literal; not a class's or a
/// namespace's, whose name says what they are.
fn named_unit(tcx: TyCtxt<'_>, did: DefId) -> Option<super::bindings::UnitValue> {
    if tcx.def_kind(did) != DefKind::Struct || super::classes::is_class(tcx, did) {
        return None;
    }
    let variant = tcx.adt_def(did).non_enum_variant();
    (variant.ctor.is_some() && variant.fields.is_empty())
        .then(|| super::bindings::unit_value(tcx, did))
        .flatten()
}

/// A parameter's name, of its type's: `event` of a `MouseEvent`, `element`
/// of an `HTMLButtonElement`, its last word's, named apart where JS keeps
/// it, `function$` of a `ResolveFunction`; `value` of any other.
pub(super) fn param_name(ty: &Value) -> String {
    match ty["name"].as_str().filter(|_| ty["kind"] == "reference") {
        Some(name) => param_word(name),
        None => "value".to_string(),
    }
}

/// A parameter's name of a type's name, `name`: its last word, lowered,
/// `event` of `MouseEvent`, named apart where JS keeps it, `function$`.
pub(super) fn param_word(name: &str) -> String {
    let start = (name.char_indices())
        .filter(|&(i, c)| c.is_uppercase() && name[i + c.len_utf8()..].starts_with(|n: char| n.is_lowercase()))
        .map(|(i, _)| i)
        .next_back()
        .unwrap_or(0);
    let word = &name[start..];
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let word: String = first.to_lowercase().chain(chars).collect();
            if rust_js_program::names::kept_word(&word) {
                format!("{word}$")
            } else {
                word
            }
        }
        None => "value".to_string(),
    }
}

/// Whether `did` is a JS value of any shape, TypeScript's `unknown` or
/// `any`, `js::Unknown`: what may be `0` or `""`, compared by `===` (ADR
/// 0612).
pub fn any_shape(tcx: TyCtxt<'_>, did: DefId) -> bool {
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

/// `ty | undefined`, as @types/react writes an optional property, of an
/// `undefined` that alone; `any | undefined` and `unknown | undefined` are
/// printed `any` and `unknown` (#1850).
fn or_undefined(ty: Value) -> Value {
    let mut types = match ty["kind"].as_str() {
        Some("union") => ty["types"].as_array().cloned().unwrap_or_default(),
        _ => vec![ty],
    };
    // Once, where it's there already, `T | undefined`'s (#1851).
    if !types.contains(&keyword("undefined")) {
        types.push(keyword("undefined"));
    }
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
    fns: &'a IndexMap<DefId, super::FnInfo>,
    /// The thread-locals it exports as its variables, of what their cells
    /// hold, and whether it sets each, `export let`, or not, `export const`
    /// (ADR 0270, amended).
    variables: &'a IndexMap<DefId, bool>,
    /// The `RefCell`s a field holds as their value (ADR 0362).
    ref_cells: &'a super::drops::RefCells<'tcx>,
    /// The `Rc`s counted, `{ value, strong, weak }`: every other, and its
    /// `Weak`, is the value it points at (ADRs 0320, 0843).
    counted: &'a super::analysis::Counted<'tcx>,
    /// The fields a `js::null()` is written to, `| null` (ADR 0935, amended).
    null_fields: &'a IndexSet<DefId>,
    /// The enums told by their keys, each variant an object of its own
    /// keys, no tag (#504).
    flattened: &'a super::bindings::Flattened,
    /// The crate's modules that have a file, by their path: those whose
    /// types another's declarations import (ADR 0210).
    files: &'a IndexMap<LocalModId, Vec<String>>,
    /// What a library's contract says its declarations export (ADR 0210,
    /// amended).
    foreign: &'a super::library::Foreign<'a, 'tcx>,
    /// The types of JS modules' it names, `("react", "ReactNode")`, imported
    /// from them, and whether each is that module's default (rd678).
    imports: BTreeSet<(String, String, bool)>,
    /// The types of libraries' modules it names, by their file's specifier.
    library_imports: BTreeSet<(String, String, bool)>,
    /// The types of the crate's other modules it names, by their path.
    /// Each by its module's path, its name, and whether it's that module's
    /// default, a class (ADR 0761).
    module_imports: BTreeSet<(Vec<String>, String, bool)>,
    /// The modules whose namespace it names, each by where it's from and its name (ADR 1037).
    namespaces: BTreeSet<(Result<Vec<String>, String>, String)>,
    /// What it names that it doesn't export, by name: another crate's
    /// untagged enums, and its own private types. Each one's declaration,
    /// `None` while it's being made, as a field or a payload names it.
    unexported: BTreeMap<String, Option<Value>>,
    /// The type parameters of the item being declared that are a type of
    /// TypeScript's, by index: `C: ReactNode`'s `ReactNode`.
    erased: IndexMap<u32, Value>,
    /// The aliases written in the type being declared, each by its type,
    /// rustc's, which has them expanded, and what TypeScript names it
    /// (`as_written`).
    named: Vec<(Ty<'tcx>, Value)>,
    /// The function types in the type being declared whose parameters the
    /// Rust names, each by its type, rustc's, and its parameters' names:
    /// a fn pointer's as written, `fn(dep: &Node)`, and a field's `dyn Fn`
    /// as the crate fills it (#543).
    function_names: Vec<(Ty<'tcx>, Vec<Option<String>>)>,
    /// Each function-typed field's parameters as every literal of the
    /// crate fills it, made once, when first asked (`filled_names`).
    filled: Option<IndexMap<DefId, Vec<Option<String>>>>,
    /// The notes of the members the declaration just made left out
    /// (`leave_out`), which the module's items take after it.
    notes: Vec<Value>,
    /// What lowering makes of a function's signature, what it takes and
    /// gives, and of a `&mut` and a `dyn` in it.
    representation: SignatureForms<'a, 'tcx>,
    /// The item whose signature is declared, whose context `representation`
    /// is asked in: its bounds say what a `&mut` to one is (ADR 0099).
    owner: DefId,
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// `["C"]`: an item's type parameters, but those `impl Trait` stands
    /// for, those that are a type of TypeScript's, and a function's it's
    /// given a JS function of, declared as the function (`callables`).
    fn generics(&mut self, def_id: DefId) -> Vec<Value> {
        let erased = erasures(self.tcx, def_id);
        let callables = self.callables(def_id);
        let boundary = self.boundary_params(def_id);
        let mut indexed = self.indexed_params(def_id);
        // Of a bound that says what a type it bounds is, where its identity
        // shows, that type, `T extends Element` (#1866).
        for (index, bound) in constraints(self.tcx, def_id) {
            if let Some(declared) = written_types(self.tcx, bound.def_id) {
                let constraint = self.written(bound.def_id, &declared, bound.args);
                indexed.entry(index).or_default().insert(0, constraint);
            }
        }
        (self.tcx.generics_of(def_id).own_params.iter())
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { synthetic: false, .. }))
            .filter(|param| !erased.iter().any(|(index, _)| *index == param.index))
            .filter(|param| !callables.iter().any(|callable| callable.index == param.index))
            .filter(|param| !boundary.iter().any(|(index, _)| *index == param.index))
            .map(|param| {
                let mut declared = match indexed.swap_remove(&param.index) {
                    // Of a key a binding trait's projection is indexed by,
                    // `Name extends keyof EventMapOf<Target>` (#1817), and of
                    // a bound that says what it is, `T extends Element`.
                    Some(mut keys) => {
                        let constraint = match keys.len() {
                            1 => keys.pop().expect("a key"),
                            _ => json!({ "kind": "intersection", "types": keys }),
                        };
                        json!({ "name": param.name.as_str(), "constraint": constraint })
                    }
                    None => json!({ "name": param.name.as_str() }),
                };
                // A struct's default, `R = &'static str`, TypeScript's
                // `= string`, or a function's, its `js::DefaultsTo<True>`
                // bound's, `= true` (ADR 1164).
                if let ty::GenericParamDefKind::Type { has_default: true, .. } = param.kind {
                    let default = self
                        .tcx
                        .type_of(param.def_id)
                        .instantiate_identity()
                        .skip_normalization();
                    let written = param
                        .def_id
                        .as_local()
                        .and_then(|local| match self.tcx.hir_node_by_def_id(local) {
                            hir::Node::GenericParam(param) => match param.kind {
                                hir::GenericParamKind::Type { default, .. } => default,
                                _ => None,
                            },
                            _ => None,
                        });
                    declared["default"] = self.as_written(written, |out| out.ts(default));
                } else if let Some(default) = defaults_to(self.tcx, def_id, param.index) {
                    declared["default"] = self.ts(default);
                }
                // One a caller's literals are kept as, TypeScript's `const`
                // modifier, `#[rust_js::const_param]` (ADR 1164).
                if is_const_param(self.tcx, param.def_id) {
                    declared["const"] = json!(true);
                }
                declared
            })
            .collect()
    }

    /// A function's own type parameters it's given a JS function of, its
    /// `JsSignature`'s (#661); none of another item.
    fn callables(&self, def_id: DefId) -> Vec<super::signatures::Callable<'tcx>> {
        match self.tcx.def_kind(def_id) {
            DefKind::Fn | DefKind::AssocFn => (self.representation.signature)(def_id).callables,
            _ => Vec::new(),
        }
    }

    /// `def_id`'s declaration begun: what's asked of a `&mut` or a `dyn` in
    /// its signature asked of its own context, then its erased parameters,
    /// whose callables' types ask it too. Asked of the module's, the crate
    /// root's, its environment is no item's, a rustc ICE (#1029).
    fn enter(&mut self, def_id: DefId) {
        self.owner = def_id;
        self.erased = self.erased(def_id);
    }

    /// `declare`, of `def_id` entered (`enter`), the item before it entered
    /// again after: a private type declared where an item names it, its
    /// fields' forms asked of its own context (#2533).
    fn within<T>(&mut self, def_id: DefId, declare: impl FnOnce(&mut Self) -> T) -> T {
        let owner = self.owner;
        let erased = std::mem::take(&mut self.erased);
        self.enter(def_id);
        let declared = declare(self);
        self.owner = owner;
        self.erased = erased;
        declared
    }

    /// An item's type parameters that are a type of TypeScript's, by index,
    /// as the trait each is bound by says (`#[rust_js::types]`): `C: ReactNode`
    /// is a `ReactNode`, as a person writes `children: ReactNode`.
    fn erased(&mut self, def_id: DefId) -> IndexMap<u32, Value> {
        // Of its own parameters alone, not the item before's by their index.
        let outer = std::mem::take(&mut self.erased);
        let mut erased = IndexMap::default();
        for (index, bound) in erasures(self.tcx, def_id) {
            if !erased.contains_key(&index)
                && let Some(declared) = written_types(self.tcx, bound.def_id)
            {
                erased.insert(index, self.written(bound.def_id, &declared, bound.args));
            }
        }
        // A function's type parameter it's given a JS function of, `F:
        // Fn(&Item, usize) -> R` or an `impl Fn(..)`, is the function it
        // says, `(item: Item, index: number) => R`, as TypeScript writes a
        // callback (`JsSignature::callables`, #661).
        for callable in self.callables(def_id) {
            if let indexmap::map::Entry::Vacant(slot) = erased.entry(callable.index) {
                slot.insert(self.function_type(callable.inputs, callable.output, &[], true));
            }
        }
        // A function's type parameter bound by `Iterator` or its extensions,
        // which a JS caller gives: what it takes of JS, `Iterable<T>`, or
        // an array or the iterator that knows its back or its length
        // (`boundary`, #1994).
        for (index, boundary) in self.boundary_params(def_id) {
            if let indexmap::map::Entry::Vacant(slot) = erased.entry(index) {
                slot.insert(self.boundary_ts(boundary, true));
            }
        }
        self.erased = outer;
        erased
    }

    /// Of a function's own type parameters, each an iterator at the JS
    /// boundary, by its index (`boundary::of_param`); none of another item.
    fn boundary_params(&self, def_id: DefId) -> Vec<(u32, super::iterators::boundary::BoundaryIterator<'tcx>)> {
        if !matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
            return Vec::new();
        }
        (self.tcx.generics_of(def_id).own_params.iter())
            .filter(|param| matches!(param.kind, ty::GenericParamDefKind::Type { .. }))
            .filter_map(|param| {
                let of = ty::ParamTy::new(param.index, param.name);
                super::iterators::boundary::of_param(self.tcx, def_id, of).map(|b| (param.index, b))
            })
            .collect()
    }

    /// What TypeScript names an iterator at the JS boundary: of a
    /// parameter, `given`, what a JS caller gives, `Iterable<T>` of an
    /// `Iterator`, else an array or the iterator `@rust-js/std` declares
    /// that knows its back or its length; of what's given back, that
    /// iterator, `IterableIterator<T>` of an `Iterator` (#1994, #1999).
    fn boundary_ts(&mut self, boundary: super::iterators::boundary::BoundaryIterator<'tcx>, given: bool) -> Value {
        let item = self.ts(boundary.item);
        let mut knowing = Vec::new();
        if boundary.back {
            let name = self.imported_from(Some(std_types::PACKAGE), std_types::DOUBLE_ENDED, None);
            knowing.push(reference(&name, vec![item.clone()]));
        }
        if boundary.len {
            let name = self.imported_from(Some(std_types::PACKAGE), std_types::EXACT_SIZE, None);
            knowing.push(reference(&name, vec![item.clone()]));
        }
        let knowing = match knowing.len() {
            0 => None,
            1 => knowing.pop(),
            _ => Some(json!({ "kind": "intersection", "types": knowing })),
        };
        match (knowing, given) {
            (None, true) => reference("Iterable", vec![item]),
            (None, false) => reference("IterableIterator", vec![item]),
            (Some(knowing), true) => json!({
                "kind": "union",
                "types": [json!({ "kind": "array", "element": item, "readonly": true }), knowing],
            }),
            (Some(knowing), false) => knowing,
        }
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
        // Not one of its own parameters, `IsA<T>`'s `T`, which is what it's
        // given, below (#1866).
        let own_param = |name: &str| {
            self.tcx
                .generics_of(did)
                .own_params
                .iter()
                .any(|p| p.name.as_str() == name)
        };
        if let Written::Reference { from, name, args: None } = declared
            && (from.is_some() || !own_param(name))
        {
            let name = &self.imported_from(from.as_deref(), name, Some(did));
            // Of a trait's, a binding's class a class extends, not its
            // `Self`, which TypeScript's class has no parameter of:
            // `extends ModuleDependency` (rd680).
            let of_self = self.tcx.def_kind(did) == DefKind::Trait && args.len() == self.tcx.generics_of(did).count();
            let type_args = args.types().skip(usize::from(of_self)).map(|t| self.ts(t)).collect();
            return reference(name, type_args);
        }
        // A name written that's one of the Rust type's own type parameters
        // is what it's given: `Map<string, T>` of a `Dict<f64>` is
        // `Map<string, number>`, `{ [key: string]: T }` `{ [key: string]:
        // number }`.
        // A collection's value whose `None` it holds as `null`, a `Map`'s
        // `V`, is `T | null` of an `Option` (ADR 0884, amended).
        let params = &self.tcx.generics_of(did).own_params;
        let nullable = super::bindings::nullable_values(self.tcx, did);
        let mut param = |out: &mut Self, name: &str| match params.iter().find(|p| p.name.as_str() == name) {
            Some(param) if (param.index as usize) < args.len() => {
                let given = args.type_at(param.index as usize);
                match given.kind() {
                    // One whose `None` is `undefined`, of any value, is
                    // `unknown` as `unknown | null` is (#1850).
                    ty::Adt(adt, of)
                        if nullable.contains(&param.index) && out.tcx.is_lang_item(adt.did(), LangItem::Option) =>
                    {
                        Given::Is(json!({ "kind": "union", "types": [out.ts(of.type_at(0)), keyword("null")] }))
                    }
                    _ => Given::Is(out.ts(given)),
                }
            }
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
                let name = &self.imported_from(from.as_deref(), name, None);
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
            Written::Intersection(types) => {
                let types = (types.iter())
                    .map(|ty| self.written_value(ty, param, bound))
                    .collect::<Option<Vec<_>>>()?;
                json!({ "kind": "intersection", "types": types })
            }
            Written::Array { element, readonly } => {
                let element = self.written_value(element, param, bound)?;
                json!({ "kind": "array", "element": element, "readonly": readonly })
            }
            Written::Indexed { object, index } => {
                let object = self.written_value(object, param, bound)?;
                let index = self.written_value(index, param, bound)?;
                json!({ "kind": "indexed", "object": object, "index": index })
            }
            Written::Conditional {
                check,
                extends,
                when_true,
                when_false,
            } => json!({
                "kind": "conditional",
                "check": self.written_value(check, param, bound)?,
                "extends": self.written_value(extends, param, bound)?,
                "whenTrue": self.written_value(when_true, param, bound)?,
                "whenFalse": self.written_value(when_false, param, bound)?,
            }),
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
                        Member::Index { readonly, parameter, key, ty } => {
                            let key = self.written_value(key, param, bound)?;
                            let ty = self.written_value(ty, param, bound)?;
                            json!({ "kind": "index", "readonly": readonly, "parameter": parameter, "key": key, "type": ty })
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
    /// A module's default, `webpack/lib/dependencies/ModuleDependency#default`,
    /// is imported as its default, by the name of `item`, the Rust type it
    /// types, or of its module, as its JS imports it, `import type
    /// ModuleDependency from ".."` (rd678): the name written, its first
    /// part that one, `NullDependency.Template` of `default.Template`.
    fn imported_from(&mut self, from: Option<&str>, name: &str, item: Option<DefId>) -> String {
        let Some(from) = from else {
            return name.to_owned();
        };
        let first = imported(name);
        if first != "default" {
            self.imports.insert((from.to_string(), first, false));
            return name.to_owned();
        }
        let local = match item {
            Some(item) => self.tcx.item_name(item).to_string(),
            None => rust_js_program::casing::namespace_name(super::bindings::file_stem(from)),
        };
        self.imports.insert((from.to_string(), local.clone(), true));
        format!("{local}{}", &name[first.len()..])
    }

    /// `export function Tag(props: TagProps): ReactNode;`
    /// A function's or a `const`'s name, as its JS has it.
    fn js_name(&self, def_id: DefId) -> String {
        (self.fns.get(&def_id)).map_or_else(|| fn_name(self.tcx, def_id), |info| info.name.clone())
    }

    /// `export const render: (..) => void;`, each field of a namespace
    /// thread-local's value, its module's exports (ADR 1037, amended).
    fn namespace_fields(&mut self, key: DefId) -> Vec<Value> {
        let ty::Adt(_, args) = self.tcx.type_of(key).instantiate_identity().skip_normalization().kind() else {
            return Vec::new();
        };
        let ty::Adt(adt, held) = args.type_at(0).kind() else {
            return Vec::new();
        };
        let names = super::classes::value_namespace_exports(self.tcx, key).unwrap_or_default();
        (adt.non_enum_variant().fields.iter().zip(names))
            .map(|(field, name)| {
                let ty = self.ts(field.ty(self.tcx, held).skip_normalization());
                json!({ "kind": "const", "name": name, "exported": true, "declare": false, "type": ty })
            })
            .collect()
    }

    /// `export const IconChevron: NamedExoticComponent<IconChevronProps>;`, a
    /// `thread_local!`'s value, or a `const`'s; of a thread-local that's
    /// its module's variable, what its cell holds, `export let enabled:
    /// boolean;` where its module sets it (ADR 0270, amended).
    fn constant(&mut self, def_id: DefId) -> Value {
        let ty = self.tcx.type_of(def_id).instantiate_identity().skip_normalization();
        let variable = self.variables.get(&def_id).copied();
        let ty = match ty.kind() {
            ty::Adt(adt, args) if is_std_def(self.tcx, adt.did(), StdItem::LocalKey) => match args.type_at(0).kind() {
                ty::Adt(_, held) if variable.is_some() => held.type_at(0),
                _ => args.type_at(0),
            },
            _ => ty,
        };
        let written = self.written_type(def_id);
        // Named as the functions it may be name them (#630).
        let outer = self.fill_value_names(def_id, ty);
        let ty = match variable {
            // A nullable one's `None` is `null`, by the aliases its type is written of (ADR 0586, amended).
            Some(_)
                if super::representation::none_form(self.tcx, super::representation::NonePlace::Local(def_id))
                    .stores_null()
                    && let Some(inner) = self.option(ty) =>
            {
                json!({ "kind": "union", "types": [self.as_written(written, |out| out.ts(inner)), keyword("null")] })
            }
            // One that holds the `Option` itself, a `const`, a nullable `const` too.
            None if super::representation::none_form(self.tcx, super::representation::NonePlace::Local(def_id))
                .stores_null()
                && let Some(inner) = self.option(ty) =>
            {
                json!({ "kind": "union", "types": [self.as_written(written, |out| out.ts(inner)), keyword("null")] })
            }
            _ => self.as_written(written, |out| out.ts(ty)),
        };
        self.function_names.truncate(outer);
        // A symbol a field is named by is a `unique symbol`, what TypeScript
        // takes a computed key of (ADR 1051).
        let ty = match super::bindings::names_a_field(self.tcx, def_id) {
            true => json!({ "kind": "unique-symbol" }),
            false => ty,
        };
        let mut declared = json!({
            "kind": "const",
            "name": self.js_name(def_id),
            "exported": true,
            "declare": false,
            "type": ty,
        });
        if variable == Some(true) {
            declared["mutable"] = json!(true);
        }
        declared
    }

    /// An item's type as the Rust writes it, a `const`'s or an alias's.
    fn written_type(&self, def_id: DefId) -> Option<&'tcx hir::Ty<'tcx>> {
        self.tcx.hir_node_by_def_id(def_id.as_local()?).ty()
    }

    /// `ty` as a TypeScript type.
    /// A field's type, a struct's `field`'s: of a `Cell`, what it holds, as
    /// a `Cell` in a field is the property set in place (ADR 0288), and of
    /// a struct's `RefCell`, which holds its value or the object it is by
    /// its declaration (ADR 0863, amended); of a `OnceCell`, what it holds
    /// or `undefined` (ADR 0847).
    fn field_ts(&mut self, ty: Ty<'tcx>, field: Option<&ty::FieldDef>) -> Value {
        if let Some(item) = self.once_cell_item(ty) {
            let item = self.ts(item);
            return or_undefined(item);
        }
        match self.cell_item(ty, field) {
            Some(item) => self.ts(item),
            None => self.ts(ty),
        }
    }

    fn ts(&mut self, ty: Ty<'tcx>) -> Value {
        let tcx = self.tcx;
        // An alias the Rust writes, by its name (`as_written`).
        let erased = tcx.erase_and_anonymize_regions(ty);
        if let Some((_, named)) = self.named.iter().rev().find(|(named, _)| *named == erased) {
            return named.clone();
        }
        // What it is to JS is its contract's (#2403, ADR 1190): what's
        // printed of each kind, and imported or declared on the way, is
        // this one's.
        let facts = contract::Facts {
            tcx,
            counted: self.counted,
            ref_cells: self.ref_cells,
            foreign: self.foreign,
        };
        match Contract::of(&facts, ty) {
            Contract::Num(num) => keyword(if num.big() { "bigint" } else { "number" }),
            Contract::Bool => keyword("boolean"),
            Contract::Char | Contract::Str => keyword("string"),
            // A box or a handle, `bump(n: { value: number })`, as lowering
            // has it (`MutRefForm::ts_type`, #619); one to an object is the
            // object.
            Contract::MutRef(inner) => {
                let value = self.ts(inner);
                self.mut_ref_form(inner).ts_type(value)
            }
            // A shared borrow is what it refers to, lent to be read: of an
            // array, a slice or a `Vec`, `readonly T[]`, shallow (ADR 1189).
            Contract::Shared(inner) => {
                let mut referred = self.ts(inner);
                if referred["kind"] == "array" {
                    referred["readonly"] = json!(true);
                }
                referred
            }
            // What a function that never returns returns (ADR 0386).
            Contract::Never => keyword("never"),
            Contract::Unit => keyword("undefined"),
            Contract::Tuple(items) => {
                let elements: Vec<Value> = items.iter().map(|t| self.ts(t)).collect();
                json!({ "kind": "tuple", "elements": elements })
            }
            Contract::Items(item) | Contract::Vec(item) => {
                json!({ "kind": "array", "element": self.ts(item), "readonly": false })
            }
            Contract::Param(param) => match self.erased.get(&param.index) {
                Some(erased) => erased.clone(),
                // An `impl Trait` parameter has no name but rustc's, `impl
                // js::Defined + 'a`, which isn't TypeScript, and `generics`
                // leaves it out: a value of any type, as a person declares one.
                None if param.name.as_str().starts_with("impl ") => keyword("unknown"),
                None => reference(param.name.as_str(), Vec::new()),
            },
            // A function of what Rust says it takes and gives:
            // `dyn Fn(&event::MouseEvent)` is `(event: MouseEvent<Element>) => void`.
            Contract::FnPtr => {
                let sig = ty.fn_sig(tcx).skip_binder();
                let named = self.function_names(erased);
                self.function_type(sig.inputs(), sig.output(), &named, true)
            }
            Contract::DynFn { inputs, output } => {
                let named = self.function_names(erased);
                self.function_type(inputs, output, &named, true)
            }
            // A `dyn` of a trait that says what it is to TypeScript:
            // `dyn ReactNode` is a `ReactNode` (ADR 0242).
            Contract::DynWritten(t, declared) => self.written(t, &declared, ty::List::empty()),
            // A type map's associated type, of a reason `R`, its lookup's
            // `EventOf<R>` (ADR 1164).
            Contract::TypeMapProjection(def_id, args) => self.type_map_projection(def_id, args),
            // A binding trait's associated type that says what it is, of
            // the projection's `Self` and arguments: `<Target as
            // Listen<Name>>::Event`, `EventMapOf<Target>[Name]` (#1817).
            Contract::WrittenProjection(def_id, args) => self
                .projection(def_id, args)
                .expect("a projection that says what it is"),
            // An `impl Future`, an `async fn`'s result: the `Promise` its JS
            // gives, of what it resolves to, `Promise<string[]>`, of nothing
            // `Promise<void>` (ADR 0029, #1769).
            Contract::Future(output) => {
                let payload = if output.is_unit() {
                    keyword("void")
                } else {
                    self.ts(output)
                };
                reference("Promise", vec![payload])
            }
            // An `impl Iterator` a function gives, or one of its extensions:
            // the JS iterator it gives back, which knows its back or its
            // length where its bounds say (`boundary::of_opaque`, #1999).
            Contract::Iterator(boundary) => self.boundary_ts(boundary, false),
            // An `impl Fn(A) -> R` a function gives: the function it says, as
            // a parameter's is, `cache`'s `(...args: unknown[]) => unknown`.
            Contract::OpaqueFn(inputs, output) => self.function_type(&inputs, output, &[], true),
            // Any other `dyn`, its pair or `any` (#620).
            Contract::Dyn => self.dyn_value(ty),
            // `(...args: any[]) => any`: a closure's own type, which no
            // signature names.
            Contract::Closure => json!({
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
            // `None` is JS's `null` too, as rust-js reads it `!= null` (ADR 0030).
            Contract::Option(payload) => self.option_ts(payload),
            // The js crate's literal types, `js::True`, the literal it is,
            // `true`, which no module of it declares (ADR 1164).
            Contract::JsLiteral(value) => json!({ "kind": "literal", "value": value.literal() }),
            // A `HashMap` or a `BTreeMap` is a `Map`, a `HashSet` or a
            // `BTreeSet` a `Set`, each class the runtime keeps one in
            // extending it (ADRs 0059, 0121, 0876).
            Contract::Map(key, value) => reference("Map", vec![self.ts(key), self.ts(value)]),
            Contract::Set(item) => reference("Set", vec![self.ts(item)]),
            // A `Box`, an uncounted `Rc` and a counted one of a class, the
            // instance itself (#1228), and a `RefCell` of an object, are what
            // they point at (ADRs 0023, 0843, 0863); an uncounted `Rc`'s
            // `Weak` that or `null`, `Weak::new()`'s.
            Contract::Pointee(pointee) => self.ts(pointee),
            Contract::Weak(pointee) => json!({ "kind": "union", "types": [self.ts(pointee), keyword("null")] }),
            // A `Cell` of its own, or one lent, is `{ value }` (ADRs 0023, 0288),
            // and so is a `RefCell` that isn't what it holds (ADRs 0328, 0863).
            Contract::Held(inner) => {
                let value = self.ts(inner);
                held(value)
            }
            Contract::Rest => reference("Record", vec![keyword("string"), keyword("unknown")]),
            // A typed class, `Class<fn(A) -> T>`, its constructor's type,
            // `new (a: A) => T` (ADR 1009).
            Contract::JsClass(signature) => {
                let sig = signature.fn_sig(tcx).skip_binder();
                let named = self.function_names(tcx.erase_and_anonymize_regions(signature));
                let mut constructor = self.function_type(sig.inputs(), sig.output(), &named, false);
                constructor["construct"] = json!(true);
                constructor
            }
            // One of std's opaque to TypeScript, `LazyLock<Point>`, as
            // `@rust-js/std` declares it, which only rust-js's JS gives
            // a value of (ADR 1149).
            Contract::StdOpaque(opaque, args) => {
                let name = self.imported_from(Some(std_types::PACKAGE), opaque.name, None);
                let held = if opaque.holds() {
                    vec![self.ts(args.type_at(0))]
                } else {
                    Vec::new()
                };
                reference(&name, held)
            }
            // A binding's, as it says it's typed: React's `MemoExoticComponent<P>`,
            // `react#NamedExoticComponent`, is `NamedExoticComponent<P>`,
            // and `react#AnchorHTMLAttributes<HTMLAnchorElement>` that.
            Contract::Written(did, declared, args) => {
                self.declare_global(did, &declared);
                self.written(did, &declared, args)
            }
            // A `Cow`, what it borrowed or owns, `{ TAG: "Borrowed"; _0:
            // string } | { TAG: "Owned"; _0: string }`, each variant as
            // an enum's is (ADR 0033), its `Owned` what `ToOwned` makes
            // of what it borrows (#1748).
            // A `Result` too, the value JS reads its `TAG` of, `{ TAG:
            // "Ok"; _0: string[] } | { TAG: "Err"; _0: any }` (#1835).
            Contract::Tagged(did, args) => union(self.tagged_variants(did, args, true)),
            // Another crate's untagged enum, `js::Json`: declared here, as
            // this crate's are, the union of its payloads (ADR 0225).
            Contract::ForeignUntagged(did, args) => {
                let name = type_name(tcx, did);
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
                reference(&name, type_args)
            }
            // A module's namespace, its module imported whole (ADR 1037).
            Contract::Namespace(did) => self.namespace_type(did),
            // A library's its declarations export, imported from them
            // (ADR 0210, amended).
            Contract::Library(did, args) => {
                let (from, name, default) = self.foreign.declared_type(did).expect("a library's declared type");
                self.library_imports.insert((from.clone(), name.clone(), *default));
                let type_args = self.type_args(did, args);
                reference(name, type_args)
            }
            Contract::Local(did, args) => self.local_ts(did, args),
            Contract::Any => keyword("any"),
        }
    }

    /// One of the crate's own types, `did` of `args`: declared, this
    /// module's by its name, another's imported from it (ADR 0210). Any
    /// other is `any`.
    fn local_ts(&mut self, did: DefId, args: ty::GenericArgsRef<'tcx>) -> Value {
        let tcx = self.tcx;
        let local = did.expect_local();
        if !declares_type(tcx, did) {
            return keyword("any");
        }
        // A class's where its constructor function is.
        let home = super::classes::home(tcx, did).unwrap_or_else(|| tcx.parent_module_from_def_id(local));
        // This module's own private type, `struct DiagramGroupProps`,
        // which its exported function names: declared here, not
        // exported (ADR 0385). Another module's isn't its to declare.
        if !tcx.visibility(did).is_public() {
            if home != self.module {
                return keyword("any");
            }
            let name = type_name(tcx, did);
            if !self.unexported.contains_key(&name) {
                self.unexported.insert(name.clone(), None);
                // In its own context, not the item's that named it: a field's
                // `Option` asks it, which of a module's is a rustc ICE (#2533).
                let declared = self.within(did, |out| match tcx.def_kind(did) {
                    DefKind::Struct => out.struct_declaration(did),
                    DefKind::Union => Some(out.union_type(did)),
                    _ => Some(out.enumeration(did)),
                });
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
        // Another module's is imported from its file, which a module
        // of a public type has, of types alone its `.d.ts` (ADR 1005).
        if home != self.module {
            self.imported(self.files[&home].clone(), did, type_name(tcx, did));
        }
        let type_args = self.type_args(did, args);
        reference(&type_name(tcx, did), type_args)
    }
}

/// The type `def_id`'s type parameter `index` defaults to in TypeScript: the
/// argument of its bound `js::DefaultsTo<T>`, which rustc resolved (ADR
/// 1164).
fn defaults_to<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, index: u32) -> Option<ty::Ty<'tcx>> {
    (tcx.clauses_of(def_id).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .map(|clause| clause.skip_binder().trait_ref)
        .find(|tr| {
            super::recognition::is_js_defaults_to(tcx, tr.def_id)
                && matches!(tr.self_ty().kind(), ty::Param(param) if param.index == index)
        })
        .map(|tr| tr.args.type_at(1))
}

/// Whether a type parameter is marked `#[rust_js::const_param]`, TypeScript's
/// `const` one, whose caller's literals are kept as literals (ADR 1164).
fn is_const_param(tcx: TyCtxt<'_>, param: DefId) -> bool {
    let path = [
        rustc_span::Symbol::intern("rust_js"),
        rustc_span::Symbol::intern("const_param"),
    ];
    // Another crate's generic parameter has no attributes in its metadata,
    // and isn't declared here.
    param.is_local() && tcx.get_attrs_by_path(param, &path).next().is_some()
}

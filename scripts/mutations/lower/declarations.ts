// Mutations of crates/rust-js-lower/src/declarations.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/declarations.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "const-param-unread",
    breaks: "a `#[rust_js::const_param]` type parameter isn't TypeScript's `const` one, so a caller's `{ reason: \"escape-key\" }` is kept as a `string`, where upstream's `const Given` keeps the literal (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                if is_const_param(self.tcx, param.def_id) {\n",
    replace: "                if false && is_const_param(self.tcx, param.def_id) {\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "a const type parameter"],
  },
  {
    name: "defaults-to-unread",
    breaks: "a function's `js::DefaultsTo<js::True>` bound isn't its default, so `Cancelable extends boolean` has no `= true` (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                } else if let Some(default) = defaults_to(self.tcx, def_id, param.index) {\n",
    replace: "                } else if false && let Some(default) = defaults_to(self.tcx, def_id, param.index) {\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "a const type parameter"],
  },
  {
    name: "struct-type-default-undeclared",
    breaks: "a struct's type parameter's default isn't declared, so a TypeScript caller must give `CommitDetails` its reason, where Rust's `CommitDetails` is `CommitDetails<&'static str>` (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                if let ty::GenericParamDefKind::Type { has_default: true, .. } = param.kind {\n",
    replace: "                if false && let ty::GenericParamDefKind::Type { has_default: true, .. } = param.kind {\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"],
  },
  {
    name: "trait-type-map-undeclared",
    breaks: "a `#[rust_js::type_map]` trait declares nothing, so `ReasonVocabulary`, `ChangeReason` and `EventOf` are no types (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            items.extend(out.type_map(def_id));\n",
    replace: "",
    tests: ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"],
  },
  {
    name: "named-unit-type-its-value",
    breaks: "a unit struct named as a string is named by that string as a type, `export type trigger-press = ..`, which no TypeScript reads (ADR 1164)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        Some(_) => tcx.item_name(did).to_string(),\n",
    replace: "        Some(_) => super::bindings::given_name(tcx, did).unwrap_or_default(),\n",
    tests: ["test/declarations-type-maps.test.ts", "-t", "its keys and its lookup"],
  },
  {
    name: "item-erased-asked-of-the-module",
    breaks: "a library's `front`, whose `impl FnMut(&mut U)` asks a `&mut`'s form, is declared in the crate root's context, a rustc ICE, `generics_of: unexpected node kind` (#1029)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        self.owner = def_id;\n        self.erased = self.erased(def_id);\n",
    replace: "        self.erased = self.erased(def_id);\n        self.owner = def_id;\n",
    tests: ["test/crates-pairs-1.test.ts", "-t", "library_signatures_asked_of_their_items"],
  },
  {
    name: "kept-word-parameter-declared",
    breaks:
      "a function type's parameter named after a type whose last word JS keeps is `function`, a `.d.ts` TypeScript can't parse (rd673)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            if rust_js_program::names::kept_word(&word) {\n",
    replace: "            if false && rust_js_program::names::kept_word(&word) {\n",
    tests: ["test/declarations-function-types.test.ts", "-t", "apart from a word JS keeps"],
  },
  {
    name: "default-class-declared-const",
    breaks:
      "a default class is declared `declare const Complex: Complex;`, not a class",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                DefKind::Struct => out.class(def_id),\n",
    replace: "",
    tests: [
      "test/classes.test.ts",
      "-t",
      "a class is its module's default export",
    ],
  },
  {
    name: "declaration-erased-of-the-last-item",
    breaks:
      "an `impl Fn(A) -> T` parameter is declared of what the item before erased by `T`'s index, `(a: A) => () => T`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        let outer = std::mem::take(&mut self.erased);\n",
    replace: "        let outer = self.erased.clone();\n",
    tests: ["test/declarations-impl-fn.test.ts"],
  },
  {
    name: "weak-declared-never-null",
    breaks: "an uncounted `Weak` is declared what it points at, though `Weak::new()` is `null` (ADR 0845)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Contract::Weak(pointee) => json!({ \"kind\": \"union\", \"types\": [self.ts(pointee), keyword(\"null\")] }),",
    replace: "            Contract::Weak(pointee) => self.ts(pointee),",
    tests: ["test/declarations.test.ts","-t","an Rc is declared"],
  },
  {
    name: "numbered-variant-consts-undeclared",
    breaks:
      "a numbered enum's `.d.ts` has no `export const Empty: 0;`, so TypeScript finds no `Empty` its JS exports",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            items.extend(out.variant_constants(def_id));\n",
    replace: "",
    tests: [
      "test/lowering-repr-enums.test.ts",
      "-t",
      "a repr enum's variant is its number",
    ],
  },
  {
    name: "alias-undeclared",
    breaks:
      "a `.d.ts` has no `export { currentTime as unstable_now };`, so TypeScript finds no `unstable_now`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "    if !names.is_empty() {\n",
    replace: "    if false && !names.is_empty() {\n",
    tests: ["test/modules.test.ts", "-t", "another name of the module's own"],
  },
  {
    name: "written-param-unsubstituted",
    breaks:
      "a name in a `types` that's the Rust type's parameter isn't what it's given: `Dict<f64>`, `{ [key: string]: T }`, isn't `{ [key: string]: number }`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Some(param) if (param.index as usize) < args.len() => {\n",
    replace: "            Some(param) if false => {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "written-method-parameter-substituted",
    breaks:
      "a method's own type parameter written in a `types`, `map<T>(f: (value: T) => T)`, is taken for the Rust type's `T`, `(value: string) => string`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                if from.is_none() && args.is_none() && !bound.contains(name) {",
    replace: "                if from.is_none() && args.is_none() {",
    tests: ["test/declarations.test.ts", "-t", "a binding's object type"],
  },
  {
    name: "written-object-members-dropped",
    breaks:
      "an object type written in a `types`, `ConsoleTask`'s `{ run<T>(f: () => T): T }`, is declared `{}`, and its `run` is TypeScript's error",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '                json!({ "kind": "object", "members": values })',
    replace:
      '                json!({ "kind": "object", "members": values.drain(..0).collect::<Vec<_>>() })',
    tests: ["test/declarations.test.ts", "-t", "a binding's object type"],
  },
  {
    name: "types-args-appended",
    breaks:
      "a `types` written with its arguments, `react#ReactNode<>`, takes the Rust type's too: `ReactNode<any>`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        if let Written::Reference { from, name, args: None } = declared\n",
    replace:
      "        if let Written::Reference { from, name, args: _ } = declared\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "generic-type-imported-whole",
    breaks:
      "a namespace's member is imported by its whole name, `JSX.Element`, not as its namespace, `JSX`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        let first = imported(name);\n",
    replace: "        let first = name.to_string();\n",
    tests: tests,
  },
  {
    name: "module-type-not-imported",
    breaks: "another module's type is named, `RouteItem[]`, but not imported",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            self.imported(self.files[&home].clone(), did, type_name(tcx, did));\n",
    replace: "",
    tests: ["test/declarations.test.ts"],
  },
  {
    name: "type-alias-undeclared",
    breaks:
      "a `pub type` isn't declared, so TypeScript importing it, react.dev's Toc its `Toc`, finds no such export",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            DefKind::TyAlias => Some(out.alias(def_id)),\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "type alias"],
  },
  {
    name: "private-default-undeclared",
    breaks:
      "a component only `js::export_default! { .. }` exports isn't declared, so its `.d.ts` exports no default, which TypeScript importing it refuses",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        let held = held_default == Some(def_id);\n",
    replace: "        let held = false;\n",
    tests: ["test/declarations.test.ts", "-t", "private default"],
  },
  {
    name: "private-default-exported",
    breaks:
      "a component only `js::export_default! { .. }` exports is declared `export function Callout`, which its JS doesn't export by that name",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '                if function {\n                    declared["default"] = json!(true);\n',
    replace:
      '                if false {\n                    declared["default"] = json!(true);\n',
    tests: ["test/declarations.test.ts", "-t", "private default"],
  },
  {
    name: "pub-default-exported-by-name",
    breaks:
      "a `pub fn` `js::export_default! { .. }` holds is declared `export function noop`, where its JS is `export default function noop`, so TypeScript imports a name the JS doesn't export",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            if held {\n",
    replace: "            if held && !tcx.visibility(def_id).is_public() {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "pub default export the macro holds",
    ],
  },
  {
    name: "named-default-undeclared",
    breaks:
      "a thread-local `js::export_default!(Memoized)` names isn't declared, so its `.d.ts` says `export default Memoized;` of no `Memoized`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        if named_default == Some(def_id) {\n",
    replace: "        if false {\n",
    tests: ["test/declarations.test.ts", "-t", "thread-local default export"],
  },
  {
    name: "pub-named-default-declared-by-name",
    breaks:
      "a `pub fn` `js::export_default!(page)` names is declared `export function page`, where its JS exports it as the default alone, so TypeScript imports a name the JS doesn't export",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: " || is_mark(tcx, def_id) || named_default == Some(def_id) {\n",
    replace: " || is_mark(tcx, def_id) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "naming form names as the default alone",
    ],
  },
  {
    name: "node-param-kept",
    breaks: "a type parameter bound by react's `Node` stays a parameter, `children: C`, where a person writes `children: ReactNode`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Contract::Param(param) => match self.erased.get(&param.index) {",
    replace: "            Contract::Param(param) => match None::<&Value> {",
    tests: ["test/declarations.test.ts","-t","typed as React types them"],
  },
  {
    name: "erased-param-declared",
    breaks:
      "a type parameter that's `ReactNode` is still declared, `ButtonLinkProps<C>`, of a `C` nothing uses",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            .filter(|param| !erased.iter().any(|(index, _)| *index == param.index))\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "typed as React types them"],
  },
  {
    name: "erased-arg-given",
    breaks:
      "a reference gives an argument for a type parameter its declaration doesn't have, `ButtonLinkProps<ReactNode>` of `ButtonLinkProps`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            .filter(|(index, _)| !erased.iter().any(|(e, _)| *e as usize == *index))\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "typed as React types them"],
  },
  {
    name: "impl-trait-param-named",
    breaks:
      "an `impl Trait` parameter is declared by rustc's name for it, `impl js::Defined + 'a`, which isn't TypeScript",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '                None if param.name.as_str().starts_with("impl ") => keyword("unknown"),\n',
    replace: "",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "impl Trait parameter of no TypeScript type",
    ],
  },
  {
    name: "namespace-member-imported-whole",
    breaks:
      '`JSX.Element` is imported as `import type { JSX.Element } from "react"`, which isn\'t TypeScript',
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "name.split('.').next().unwrap_or(name).to_string()",
    replace: "name.to_string()",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations declare a private default export",
    ],
  },
  {
    name: "default-export-const-as-function",
    breaks:
      "a private `thread_local!` default export is declared a function, not `declare const Memoized: ..`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                _ => out.constant(def_id),\n",
    replace: "                _ => out.function(def_id),\n",
    tests: ["test/declarations.test.ts", "-t", "thread-local default"],
  },
  {
    name: "impl-fn-unknown",
    breaks:
      "an `impl Fn(&Item, usize) -> R` parameter is declared `unknown`, not its function",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                slot.insert(self.function_type(callable.inputs, callable.output, &[], true));\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "impl Fn parameter"],
  },
  {
    name: "declarations-private-type-any",
    breaks: "a module's private props struct, which its exported component names, is `any`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        // exported (ADR 0385). Another module's isn't its to declare.\n        if !tcx.visibility(did).is_public() {",
    replace: "        // exported (ADR 0385). Another module's isn't its to declare.\n        if !tcx.visibility(did).is_public() {\n            return keyword(\"any\");\n        }\n        if false {",
    tests: ["test/declarations.test.ts","-t","private struct"],
  },
  {
    name: "declarations-private-type-exported",
    breaks: "a module's private struct is exported from its `.d.ts`, where its Rust doesn't export it",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                };\n                declared[\"exported\"] = json!(false);\n",
    replace: "                };\n",
    tests: ["test/declarations.test.ts","-t","private struct"],
  },
  {
    name: "declarations-unexported-last",
    breaks:
      "a type declared but not exported comes after what names it, not before, as TypeScript has it",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "    items.splice(0..0, out.unexported.into_values().flatten());",
    replace: "    items.extend(out.unexported.into_values().flatten());",
    tests: ["test/declarations.test.ts", "-t", "private struct"],
  },
  {
    name: "union-declared-any",
    breaks:
      "an exported union is TypeScript's `any`, not the fields it may hold",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            DefKind::Union => Some(out.union_type(def_id)),\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "union"],
  },
  {
    name: "optional-union-nested",
    breaks:
      "an optional field of a union holds it whole, `(string | null) | undefined`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '        Some("union") => ty["types"].as_array().cloned().unwrap_or_default(),\n',
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "empty-union-unprinted",
    breaks:
      "an enum of no variants is declared `type Void = ;`, which isn't TypeScript, where it's `never`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '    if types.is_empty() {\n        return keyword("never");\n    }\n',
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "no variants"],
  },
  {
    name: "written-alias-expanded",
    breaks:
      "an alias the Rust writes is what it stands for, `TocItem[]`, where it's `Toc`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        if let Some((_, named)) = self.named.iter().rev().find(|(named, _)| *named == erased) {",
    replace:
      "        if let Some((_, named)) = self.named.iter().rev().find(|_| false) {",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "written-const-expanded",
    breaks:
      "a `const`'s type written with an alias, `Context<Toc>`, is what it stands for",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            _ => self.as_written(written, |out| out.ts(ty)),\n",
    replace: "            _ => self.ts(ty),\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations name a type alias",
    ],
  },
  {
    name: "declaration-named-as-rust",
    breaks:
      "a function named as JS keeps a word apart is declared as `new`, not its JS's `new$`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        (self.fns.get(&def_id)).map_or_else(|| fn_name(self.tcx, def_id), |info| info.name.clone())\n",
    replace: "        fn_name(self.tcx, def_id)\n",
    tests: ["test/declarations.test.ts", "-t", "as its JS does"],
  },
  {
    name: "bound-global-undeclared",
    breaks:
      "a module's `const assign = Object.assign` isn't declared, so its `.d.ts`'s `export default assign` names nothing",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "    for held in (held.iter()).filter(|held| held.through.is_none() && (held.export || held.default)) {\n",
    replace:
      "    for held in (held.iter()).filter(|held| held.through.is_none() && (held.export || held.default)).take(0) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "a global binding a module names",
    ],
  },
  {
    name: "held-static-typeof",
    breaks:
      "a binding's static a module holds is declared `typeof Symbol.iterator`, not its type",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            DefKind::Static { .. } => out.constant(held.binding),\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "static a module holds"],
  },
  {
    name: "default-constant-declared-as-default",
    breaks:
      "a default constant's `.d.ts` is `export default const`, which TypeScript can't read",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                let function = tcx.def_kind(def_id) == DefKind::Fn;\n",
    replace: "                let function = true;\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "the constant's value where the macro holds it",
    ],
  },
  {
    name: "own-alias-declared-exported",
    breaks:
      "a `.d.ts` exports `getCurrentTime` by its name too, where its JS exports it as `unstable_now` alone (ADR 0596, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            if !held && let Some((_, (_, export))) = aliases.iter().find(|&&(item, _)| item == def_id) {\n",
    replace:
      "            if false && let Some((_, (_, export))) = aliases.iter().find(|&&(item, _)| item == def_id) {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a pub use by another name of the module's own function exports it by that",
    ],
  },
  {
    name: "variable-declared-const",
    breaks:
      "an exported variable its module sets is declared `export const enabled: boolean;` (ADR 0270, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '            declared["mutable"] = json!(true);\n',
    replace: "",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "variable-declared-cell",
    breaks:
      "an exported variable is declared as its cell, `export let enabled: { value: boolean };` (ADR 0270, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                ty::Adt(_, held) if variable.is_some() => held.type_at(0),\n",
    replace:
      "                ty::Adt(_, held) if variable.is_some() && held.is_empty() => held.type_at(0),\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an exported cell its module alone sets",
    ],
  },
  {
    name: "nullable-local-declared-undefined",
    breaks:
      "a nullable thread-local's variable is declared `Fiber | null | undefined`, where its `None` is `null` (ADR 0586, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                if super::representation::none_form(self.tcx, super::representation::NonePlace::Local(def_id))\n",
    replace: "                if false && super::representation::none_form(self.tcx, super::representation::NonePlace::Local(def_id))\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable thread-local's None is null",
    ],
  },
  {
    name: "nullable-local-declared-unaliased",
    breaks:
      "a nullable thread-local of an `Option<CallDestroy>`, and a variable's `Cell<Option<ConsoleTask>>`, are declared by what the alias is, `((fiber: Fiber, value: number) => number) | null`, not `CallDestroy | null` (ADR 0586, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        let written = self.written_type(def_id);\n        // Named as the functions it may be name them (#630).\n",
    replace: "        let written = None;\n        // Named as the functions it may be name them (#630).\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a nullable thread-local of an Option is null where it's made",
    ],
  },
  {
    name: "nullable-option-local-declared-undefined",
    breaks:
      "a nullable thread-local of an `Option`, and a nullable `const`, is declared `T | undefined`, not `T | null` (ADR 0586, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            None if super::representation::none_form(self.tcx, super::representation::NonePlace::Local(def_id))\n                .stores_null()\n                && let Some(inner) = self.option(ty) =>\n",
    replace: "            None if false\n                && let Some(inner) = self.option(ty) =>\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a nullable thread-local of an Option is null where it's made|a nullable const's None is null",
    ],
  },

  {
    name: "class-of-another-modules-struct-undeclared",
    breaks:
      "a class whose struct is another module's is declared nowhere, though its references import it from where its constructor is",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            items.extend(exported_class(tcx, documented(out.class(class), tcx, class), class));\n",
    replace: "",
    tests: ["test/class-constructors.test.ts", "-t", "constructor's statements set this"],
  },
  {
    name: "typed-class-declared-a-function",
    breaks: "a typed `js::Class` is declared as a function, `(value: string) => unknown`, which TypeScript calls, not `new`s",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                constructor[\"construct\"] = json!(true);\n",
    replace: "",
    tests: ["test/js-constructors.test.ts","-t","a typed class is declared as its constructor's type"],
  },
  {
    name: "export-named-apart-declared-apart",
    breaks:
      "the `.d.ts` of an export named apart of a global, `console`, is `declare function console$()` and its alias, not `export function console()` (#338)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '                    true => declared["name"] = json!(export),\n',
    replace:
      '                    true if false => declared["name"] = json!(export),\n                    true => {\n                        declared["exported"] = json!(false);\n                        declared["declare"] = json!(true);\n                    }\n',
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "kept-word-declared-by-its-name",
    breaks:
      "the `.d.ts` of `pub fn delete()` is `export function delete(): number;`, a declaration no `.d.ts` may name (#338)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: " == Some(export) && !rust_js_program::names::kept_word(export)\n",
    replace: " == Some(export)\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "intersection-declared-union",
    breaks:
      "a `RegExpMatch`'s `.d.ts` is `string[] | { .. }`, a union, not the intersection it's declared (#303)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: '                json!({ "kind": "intersection", "types": types })\n',
    replace: "                union(types)\n",
    tests: [
      "test/untagged-enums.test.ts",
      "-t",
      "an untagged enum's js::RegExpMatch is an array",
    ],
  },
  {
    name: "nullable-value-declared-undefined",
    breaks:
      "a map of `Option` values is declared `Map<string, string | null | undefined>`, where it holds `None` as `null` alone, `Map<string, string | null>` (ADR 0884, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                        if nullable.contains(&param.index) && out.tcx.is_lang_item(",
    replace: "                        if false && nullable.contains(&param.index) && out.tcx.is_lang_item(",
    tests: ["test/js-collections.test.ts", "-t", "a Map's value is any value"],
  },
  {
    name: "namespace-type-any",
    breaks: "a namespace's type is `any`, not `typeof Reconciler` (ADR 1037)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Contract::Namespace(did) => self.namespace_type(did),",
    replace: "            Contract::Namespace(_) => keyword(\"any\"),",
    tests: ["test/namespaces.test.ts","-t","a namespace's methods are its module's functions"],
  },
  {
    name: "private-renamed-global-declared",
    breaks:
      "a private `use` that renames a global, `use js::array::SLICE as slice;`, is declared in the module's .d.ts, `declare const slice: ..`, which no export names (rd517)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "    for held in (held.iter()).filter(|held| held.through.is_none() && (held.export || held.default)) {\n",
    replace: "    for held in (held.iter()).filter(|held| held.through.is_none()) {\n",
    tests: ["test/lowering-binding-paths.test.ts", "-t", "private use renaming a global"],
  },
  {
    name: "namespace-value-undeclared",
    breaks: "a namespace thread-local's fields aren't declared in its `.d.ts`, so TypeScript finds none of its module's exports (rd293, ADR 1037, amended)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            items.extend(out.namespace_fields(def_id));\n",
    replace: "            items.extend(Vec::<Value>::new());\n",
    tests: ["test/namespace-values.test.ts", "-t", "fields are its module's exports"],
  },
  {
    name: "written-alias-unexported-to-consumers",
    breaks:
      "a library's alias whose `#[rust_js::types]` writes a union isn't in its contract's declared types, so a consumer writes it out, its names unimported (rd538)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        DefKind::TyAlias => true,",
    replace: "        DefKind::TyAlias => false,",
    tests: ["test/declarations-libraries.test.ts", "-t", "declared as written"],
  },
  {
    name: "library-plain-alias-undeclared",
    breaks: "a library's alias without `#[rust_js::types]` isn't among its contract's declared types, so a consumer writes it out and can't re-export it (rd642)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        DefKind::TyAlias => true,",
    replace: "        DefKind::TyAlias => aliases::declares_written_alias(tcx, did),",
    tests: ["test/declarations-libraries.test.ts", "-t", "type-only re-export"],
  },
  {
    name: "class-constructor-optional",
    breaks: "a JS class's constructor's trailing `Option` is declared optional, though rust-js gives it `undefined` and the class is JS's (ADR 0330)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                let mut constructor = self.function_type(sig.inputs(), sig.output(), &named, false);",
    replace: "                let mut constructor = self.function_type(sig.inputs(), sig.output(), &named, true);",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "class-constructor-names-unread",
    breaks: "a JS class's constructor's parameters, `Class<fn(name: &str, size: Option<f64>)>`, are named by their types",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                let named = self.function_names(tcx.erase_and_anonymize_regions(signature));",
    replace: "                let named = Vec::new();",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "dyn-fn-names-unread",
    breaks: "a `dyn Fn` field's parameters are named by their types, though its literals name them (#543)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                let named = self.function_names(erased);\n                self.function_type(inputs, output, &named, true)\n",
    replace: "                let named = Vec::new();\n                self.function_type(inputs, output, &named, true)\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "fn-pointer-names-unread",
    breaks: "a fn pointer's parameters are named by their types, `(value, value2)`, though it writes `(count, _sum)`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                let named = self.function_names(erased);\n                self.function_type(sig.inputs(), sig.output(), &named, true)\n",
    replace: "                let named = Vec::new();\n                self.function_type(sig.inputs(), sig.output(), &named, true)\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "ungiven-function-declared",
    breaks: "a free function whose JS takes what a JS caller can't give is declared, not a note (#549)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            && out.leave_out(def_id, &out.js_name(def_id))\n",
    replace: "            && false\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "ungiven-alias-exported",
    breaks: "a function left out, `named`, is still exported by its other name, `export { named as unstable_named }`, which TypeScript refuses of a name it doesn't declare (#549)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        .filter(|(item, (_, export))| !by_own_name(fns, *item, export) && !left_out.contains(item))\n",
    replace: "        .filter(|(item, (_, export))| !by_own_name(fns, *item, export))\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "ungiven-notes-at-the-end",
    breaks: "a member's note is at the module's end, not after what it's of (#559)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        // The notes of what the item before left out, after it (#559).\n        items.append(&mut out.notes);\n",
    replace: "",
    tests: ["test/declarations-ungiven.test.ts"],
  },
  {
    name: "default-type-imported-by-name",
    breaks: "a type a module exports as its default, `ModuleDependency#default`, is imported as a name, `import type { default } from ..`, which TypeScript refuses (rd678)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        if first != \"default\" {\n",
    replace: "        if true {\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "its module's default is imported as its default"],
  },
  {
    name: "default-type-named-after-its-module",
    breaks: "a module's default a Rust type types is imported by its module's name, `Request`, not its own, `WebRequest`, as its JS imports it (rd678)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Some(item) => self.tcx.item_name(item).to_string(),\n",
    replace: "            Some(_) => rust_js_program::casing::namespace_name(super::bindings::file_stem(from)),\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "its module's default is imported as its default"],
  },
  {
    name: "trait-self-a-type-argument",
    breaks: "a class that extends a binding's class whose trait names it alone, `webpack#ModuleDependency`, is declared `extends ModuleDependency<ClientReferenceDependency>`, the trait's `Self` a type argument of a class of none (rd680)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            let type_args = args.types().skip(usize::from(of_self)).map(|t| self.ts(t)).collect();\n",
    replace: "            let type_args = args.types().skip(usize::from(of_self && false)).map(|t| self.ts(t)).collect();\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "is its class's getter"],
  },
  {
    name: "impl-dictionary-undeclared",
    breaks: "an impl's dictionary its module exports, `circleShape`, isn't declared, so TypeScript can't import it to give it (#550)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        if let Some(dictionary) = out.impl_dictionary(def_id) {\n",
    replace: "        if let Some(dictionary) = out.impl_dictionary(def_id).filter(|_| false) {\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "trait-undeclared",
    breaks: "a trait isn't declared as the type of its dictionaries, so what names it, `Shape<S>`, names nothing (#550)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            DefKind::Trait => out.trait_interface(def_id),\n",
    replace: "",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "mut-box-declared-value",
    breaks: "a `&mut` its JS has as a box is declared as its value, `bump(n: number)`, so a TypeScript caller gives a number, whose `value` its JS can't write (#619)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                self.mut_ref_form(inner).ts_type(value)",
    replace: "                value",
    tests: ["test/declarations-ungiven.test.ts","-t","box is declared"],
  },
  {
    name: "dyn-declared-function",
    breaks: "a `dyn` of a trait that isn't a `Fn`'s is declared a function, `(...args: any[]) => any`, where its JS is a pair, `{ value, impl }` (#620)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Contract::Dyn => self.dyn_value(ty),",
    replace: "            Contract::Dyn => json!({ \"kind\": \"function\", \"typeParameters\": [], \"params\": [], \"returns\": keyword(\"any\") }),",
    tests: ["test/declarations-ungiven.test.ts","-t","box is declared"],
  },
  {
    name: "value-names-unfilled",
    breaks: "a static's `dyn Fn` is named by its parameters' types, `(value: number, value2: string, ..)`, not as the function it is names them (#630)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        let outer = self.fill_value_names(def_id, ty);\n",
    replace: "        let outer = self.function_names.len();\n",
    tests: ["test/declarations-function-types.test.ts", "-t", "the functions its value may be"],
  },
  {
    name: "callable-type-parameter-declared",
    breaks: "a type parameter declared as the function it's bound by stays a type parameter too, `twice<F>(f: (value: number) => number)` (#661)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            .filter(|param| !callables.iter().any(|callable| callable.index == param.index))\n",
    replace: "            .filter(|param| callables.is_empty() || param.index != u32::MAX)\n",
    tests: ["test/declarations-impl-fn.test.ts", "-t", "bound by Fn"],
  },
  {
    name: "cow-owned-unnormalized",
    breaks: "a `Cow`'s `Owned` is declared of `<B as ToOwned>::Owned` as written, a projection TypeScript has no type of, not the `String` it is (#1748)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            Contract::Tagged(did, args) => union(self.tagged_variants(did, args, true)),",
    replace: "            Contract::Tagged(did, args) => union(self.tagged_variants(did, args, false)),",
    tests: ["test/declarations-collections.test.ts","-t","a Cow is declared the tagged union"],
  },
  {
    name: "type-name-given-unread",
    breaks:
      "a type's `#[rust_js::name]` is left unread, so `#[rust_js::name = \"ISODate\"] pub type IsoDate = String;` is declared `type IsoDate`, which upstream's consumer, importing `ISODate`, can't name (#1829)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        None => super::bindings::given_name(tcx, did).unwrap_or_else(|| tcx.item_name(did).to_string()),\n",
    replace: "        None => tcx.item_name(did).to_string(),\n",
    tests: ["test/declarations-named.test.ts"],
  },
  {
    name: "bound-constraint-never",
    breaks: "a bound whose parameter's identity shows prints no constraint, `selected<T>(value: T): T`, which accepts a string where Rust's `IsA<Element>` refuses it",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                indexed.entry(index).or_default().insert(0, constraint);\n",
    replace: "                let _ = (index, constraint);\n",
    tests: ["test/declarations-bounds.test.ts", "-t", "an IsA bound is TypeScript's constraint"],
  },
  {
    name: "bound-erased-of-no-arguments",
    breaks: "an erased bound's type is read of none of its arguments, so `IsA<Element>`'s `T` isn't `Element`",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                erased.insert(index, self.written(bound.def_id, &declared, bound.args));\n",
    replace: "                erased.insert(index, self.written(bound.def_id, &declared, ty::List::empty()));\n",
    tests: ["test/declarations-bounds.test.ts", "-t", "an IsA bound is TypeScript's constraint"],
  },
  {
    name: "written-own-param-taken-for-a-type",
    breaks: "a trait's written type that names its own parameter, `IsA<T>`'s `T`, is a type of that name, `T<Element>`, not what it's given",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            && (from.is_some() || !own_param(name))\n",
    replace: "            && (from.is_some() || true)\n",
    tests: ["test/declarations-bounds.test.ts", "-t", "an IsA bound is TypeScript's constraint"],
  },
  {
    name: "boundary-param-undeclared",
    breaks: "a parameter bound by `Iterator` or its extensions isn't declared what a JS caller gives: `impl` ones print `unknown` (#1994)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        for (index, boundary) in self.boundary_params(def_id) {\n",
    replace: "        for (index, boundary) in self.boundary_params(def_id).into_iter().filter(|_| false) {\n",
    tests: ["test/declarations-iterators.test.ts"],
  },
  {
    name: "boundary-param-still-generic",
    breaks: "a named `I: Iterator` parameter, declared what a JS caller gives, stays a type parameter too, `total<I>(items: Iterable<number>)` (#1994)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "            .filter(|param| !boundary.iter().any(|(index, _)| *index == param.index))\n",
    replace: "",
    tests: ["test/declarations-iterators.test.ts"],
  },
  {
    name: "generic-type-default-resolved",
    breaks: "a written default type alias is expanded to its underlying Rust type",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                    declared[\"default\"] = self.as_written(written, |out| out.ts(default));",
    replace: "                    declared[\"default\"] = self.ts(default);",
    tests: ["test/declarations-type-maps.test.ts", "-t", "written event aliases"],
  },
  {
    name: "sealed-tuple-referenced-any",
    breaks: "what names a struct no other crate can make names it `any`, `welcome(email: any)`, though it's declared (#2356, ADR 1187)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        DefKind::Struct => structures::declared(tcx, did),\n",
    replace: "        DefKind::Struct => structures::declared(tcx, did) && !structures::sealed(tcx, did),\n",
    tests: ["test/declarations-sealed.test.ts", "-t", "a tuple struct of a private field"],
  },
  {
    name: "shared-borrow-array-mutable",
    breaks: "a shared borrow of an array, `&[String]`, is declared `string[]`, so TypeScript can't pass a `readonly` array where Rust only reads (#2392)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "                    referred[\"readonly\"] = json!(true);\n",
    replace: "                    referred[\"readonly\"] = json!(false);\n",
    tests: ["test/declarations-readonly.test.ts"],
  },
  {
    name: "private-type-declared-in-the-namers-context",
    breaks: "a private struct declared where an impl names it asks its `Option` field's form in the module's context, the crate root's, a rustc ICE, `generics_of: unexpected node kind Crate` (#2533)",
    file: "crates/rust-js-lower/src/declarations.rs",
    find: "        self.enter(def_id);\n        let declared = declare(self);\n",
    replace: "        let declared = declare(self);\n",
    tests: ["test/corpus.test.ts", "-t", "^private_struct_of_an_option_declared_where_an_impl_names_it.rs$"],
  },
];

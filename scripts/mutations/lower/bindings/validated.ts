// Mutations of src/lower/bindings/validated.rs (ADR 0093), split from bindings.ts with its source, by concept.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "numbered-given-name-unchecked",
    breaks:
      "a numbered enum's variant named `\"un-terminated\"`, which can't name a `const`, is written `const un-terminated = 0;`, JS that doesn't parse, where it's refused",
    file: "src/lower/bindings/validated.rs",
    find: "                    && !crate::names::is_identifier(&given)\n",
    replace: "                    && given.is_empty()\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a repr enum's variants are its module's consts",
    ],
  },
  {
    name: "tagged-tuple-variant-accepted",
    breaks:
      "a `#[rust_js::tag]` enum's tuple variant is accepted, its fields `_0` beside the tag",
    file: "src/lower/bindings/validated.rs",
    find: "                if tuple || clash {",
    replace: "                if clash {",
    tests: ["test/diagnostics.test.ts", "-t", "tagged enum"],
  },
  {
    name: "nullable-param-unchecked",
    breaks:
      "`#[rust_js::nullable(n)]` naming no `Option` parameter is accepted",
    file: "src/lower/bindings/validated.rs",
    find: "                if !at.is_some_and(",
    replace: "                if false && !at.is_some_and(",
    tests: ["test/diagnostics.test.ts", "-t", "nullable parameter"],
  },
  {
    name: "tagged-otherwise-not-last",
    breaks:
      "an `otherwise` variant before another is taken, so the one after it is never matched",
    file: "src/lower/bindings/validated.rs",
    find: "                    if index + 1 != variants.len() || !tuple || variant.fields.len() != 1 {\n",
    replace: "                    if !tuple || variant.fields.len() != 1 {\n",
    tests: ["test/diagnostics.test.ts", "-t", "otherwise variant not last"],
  },
  {
    name: "open-union-derive-taken",
    breaks:
      "`#[derive(Clone)]` of a union with an `otherwise` copies its object as a variant of its own",
    file: "src/lower/bindings/validated.rs",
    find: "            && adt.variants().iter().any(|v| is_tagged_otherwise(tcx, adt.did(), v))\n",
    replace: "            && false\n",
    tests: ["test/diagnostics.test.ts", "-t", "derive of an open union"],
  },
  {
    name: "bool-name-of-untagged-accepted",
    breaks:
      "a boolean `rust_js::name` of an enum without a tag is accepted, its variant `true`",
    file: "src/lower/bindings/validated.rs",
    find: "                    && declared_tag(tcx, tcx.parent(named)).is_some()\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "boolean name of"],
  },
  {
    name: "variant-names-unchecked",
    breaks: "a variant's `rust_js::name` isn't checked, only an item's",
    file: "src/lower/bindings/validated.rs",
    find: "        for named in std::iter::once(def.to_def_id()).chain(variants) {",
    replace: "        for named in std::iter::once(def.to_def_id()) {",
    tests: ["test/diagnostics.test.ts", "-t", "name of"],
  },
  {
    name: "nullable-result-not-an-option",
    breaks:
      "`#[rust_js::nullable]` on a function whose result isn't an `Option` is let through, saying nothing",
    file: "src/lower/bindings/validated.rs",
    find: "            if let Some(message) = nullable::refused_result(tcx, def.to_def_id()) {\n",
    replace:
      "            if let Some(message) = nullable::refused_result(tcx, def.to_def_id()).filter(|_| false) {\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a nullable function's None is null",
    ],
  },
  {
    name: "nullable-field-unchecked",
    breaks:
      "`#[rust_js::nullable]` of a field that isn't an `Option` compiles, and does nothing it says",
    file: "src/lower/bindings/validated.rs",
    find: "                if is_nullable(tcx, field)\n                    && nullable_option(tcx, ty).is_none()\n",
    replace:
      "                if false\n                    && nullable_option(tcx, ty).is_none()\n",
    tests: ["test/diagnostics.test.ts", "-t", "nullable field"],
  },
  {
    name: "lazy-param-unchecked",
    breaks:
      "`#[rust_js::lazy(..)]` naming no parameter compiles, and makes nothing lazy",
    file: "src/lower/bindings/validated.rs",
    find: "                if !idents.iter().any(|ident| ident.is_some_and(|ident| ident.name == name)) {\n",
    replace: "                if false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "lazy parameter"],
  },
  {
    name: "nullable-cell-refused",
    breaks:
      "`#[rust_js::nullable]` of a `Cell<Option<T>>` field, React's shared internals', is refused, where its `None` can be `null`",
    file: "src/lower/bindings/validated.rs",
    find: "                    && nullable_option(tcx, ty).is_none()\n                    && nullable_items(tcx, ty).is_none()\n",
    replace:
      "                    && nullable_option(tcx, ty) != Some(ty)\n                    && nullable_items(tcx, ty).is_none()\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "nullable Cell field's None is null",
    ],
  },
  {
    name: "optional-unnullable-unrefused",
    breaks:
      "`#[rust_js::optional]` of a field that isn't nullable, which says nothing, compiles (ADR 0810)",
    file: "src/lower/bindings/validated.rs",
    find: "                } else if is_optional(tcx, field) && !is_nullable(tcx, field) && !once && !cell_of_option {\n",
    replace: "                } else if false {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "an optional field that isn't nullable",
    ],
  },
  {
    name: "nullable-items-refused",
    breaks:
      "`#[rust_js::nullable]` of a field of a `Vec<Option<T>>` is refused, as of what isn't an `Option`",
    file: "src/lower/bindings/validated.rs",
    find: "                    && nullable_items(tcx, ty).is_none()\n                    && !of_type_parameter(tcx, ty)\n",
    replace: "                    && !of_type_parameter(tcx, ty)\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "Vec of Options holds null",
    ],
  },
  {
    name: "nullable-type-parameter-refused",
    breaks:
      "a binding's type parameter `#[rust_js::nullable(..)]` names is refused, React's `createCursor<T>(defaultValue)` (ADR 0586, amended)",
    file: "src/lower/bindings/validated.rs",
    find: "                    ty::Param(_) => true,\n",
    replace: "                    ty::Param(_) => false,\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "nullable type parameter",
    ],
  },
  {
    name: "nullable-generic-field-refused",
    breaks:
      "`#[rust_js::nullable]` of a generic struct's field of a type parameter's value is refused, React's `StackCursor<T>`'s `current: RefCell<T>` (ADR 0586, amended)",
    file: "src/lower/bindings/validated.rs",
    find: "                    && !of_type_parameter(tcx, ty)\n",
    replace: "",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a generic struct's nullable field",
    ],
  },
  {
    name: "optional-once-cell-refused",
    breaks:
      "a `#[rust_js::optional]` `OnceCell` field is refused, so React's lazy payload keeps `_ioInfo: undefined` (ADR 0280, amended)",
    file: "src/lower/bindings/validated.rs",
    find: "                } else if is_optional(tcx, field) && !is_nullable(tcx, field) && !once && !cell_of_option {\n",
    replace:
      "                } else if is_optional(tcx, field) && !is_nullable(tcx, field) && (!once || true) && !cell_of_option {\n",
    tests: [
      "test/lowering-optional-fields.test.ts",
      "-t",
      "an optional OnceCell field",
    ],
  },
  {
    name: "keyed-method-key-unchecked",
    breaks: "a method's key that's no global's path, `[1]`, is taken and printed `this[1]()`, not refused (rd585)",
    file: "src/lower/bindings/validated.rs",
    find: "                    true => paths::global_refused(&key),\n",
    replace: "                    true => None,\n",
    tests: ["test/js-globals.test.ts", "-t", "an async iterable's"],
  },
  {
    name: "keyed-function-unrefused",
    breaks:
      "a key, `[Symbol.iterator]`, of a function of no receiver is taken, where it has no `this` to key, not refused (rd585)",
    file: "src/lower/bindings/validated.rs",
    find: "                    false => Some(\"a key, `[Symbol.asyncIterator]`, is a method's, of `&self` or `this`\".to_string()),\n",
    replace: "                    false => None,\n",
    tests: ["test/js-globals.test.ts", "-t", "an async iterable's"],
  },
  {
    name: "binding-path-unchecked",
    breaks:
      "a binding's path that isn't JS's, `window.$R-T`, is taken and printed as a subtraction, not refused naming its quoted key",
    file: "src/lower/bindings/validated.rs",
    find: "        if let Some(message) = path_refused(tcx, def.to_def_id()) {\n",
    replace:
      "        if let Some(message) = path_refused(tcx, def.to_def_id()).filter(|_| false) {\n",
    tests: ["test/lowering-binding-paths.test.ts"],
  },
  {
    name: "method-path-quoted",
    breaks:
      "a method's quoted key, `['matchMedia']`, is taken as its property's name and printed `this[\"['matchMedia']\"]`",
    file: "src/lower/bindings/validated.rs",
    find: "        true => path.contains(['[', '\\'']).then(|| {\n",
    replace: "        true => path.contains(['\\0']).then(|| {\n",
    tests: ["test/lowering-binding-paths.test.ts"],
  },
  {
    name: "import-path-unchecked",
    breaks:
      "an import's path past its export, `./shell.js#times.shell-time`, isn't checked, and prints a subtraction",
    file: "src/lower/bindings/validated.rs",
    find: "            Some((_, rest)) => paths::steps(rest).err(),\n",
    replace: "            Some(_) => None,\n",
    tests: ["test/lowering-binding-paths.test.ts"],
  },
  {
    name: "optional-cell-refused",
    breaks: "`#[rust_js::optional]` of a `Cell<Option<T>>` is refused, which react-art's Text's `_text` needs to be absent until its ref sets it (rd331g)",
    file: "src/lower/bindings/validated.rs",
    find: " && !once && !cell_of_option {",
    replace: " && !once {",
    tests: ["test/lowering-optional-fields.test.ts", "-t", "optional Cell of an Option"],
  },
  {
    name: "field-symbol-unvalidated",
    breaks: "no symbol name is refused, a derived `Clone`'s or a class's field's lowered as if named by text",
    file: "src/lower/bindings/validated.rs",
    find: "    valid &= symbol_names::validate(tcx);\n",
    replace: "",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a symbol name where it can't be one is refused"
    ]
  },
  {
    name: "const-link-name-refused",
    breaks:
      "a `#[rust_js::link_name]` on a constant, webapi's `Node.ELEMENT_NODE`, is refused as on no function (rd618)",
    file: "src/lower/bindings/validated.rs",
    find: "                || !matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn | DefKind::Const)\n",
    replace: "                || !matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn)\n",
    tests: ["test/corpus.test.ts", "-t", "^const_read_by_its_js_name.rs$"],
  },
  {
    name: "value-field-optional-unrefused",
    breaks:
      "a `#[rust_js::optional]` nullable field of what may be `undefined` gets the plain `optional` error, not its own (rd645)",
    file: "src/lower/bindings/validated.rs",
    find: "                if is_optional(tcx, field) && holds_value(tcx, field) {\n",
    replace: "                if false {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "refuses what would box it"],
  },
  {
    name: "optional-of-nested-misnamed",
    breaks: "`#[rust_js::optional]` of a nested nullable field is refused by the general message, which asks for a nullable field, which it is (#959)",
    file: "src/lower/bindings/validated.rs",
    find: "                } else if is_optional(tcx, field) && holds_nested(tcx, field) {\n",
    replace: "                } else if is_optional(tcx, field) && holds_nested(tcx, field) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^optional_of_a_nested_nullable_field_refused.rs$"],
  },
];

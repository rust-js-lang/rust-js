// Mutations of src/lower/declarations/structures.rs (ADR 0093): a
// struct's declaration, its fields, each by its JS name, an `Option`'s
// optional, a `#[repr(transparent)]` one its field's type.
import type { Mutation } from "../../../mutations";

const tests = ["test/declarations.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "untagged-records-declared-extended",
    breaks: "a struct that flattens an untagged enum of records extends it, which an interface can't, not `{ key: string } & CompositionInput` (#1818)",
    file: "src/lower/declarations/structures.rs",
    find: "                ty::Adt(..) => self.flattened.enum_of_keys(*ty),\n",
    replace: "                ty::Adt(adt, _) => self.flattened.keyed.contains(&adt.did()),\n",
    tests: ["test/flattened-untagged.test.ts", "-t", "declared an intersection"],
  },  {
    name: "cell-option-field-required",
    breaks:
      "a plain `Cell<Option<f64>>` field is declared `byteSize: number | null | undefined`, its cell's `Option` not seen, where it's `byteSize?: number | undefined` as an `Option` field is (rd415)",
    file: "src/lower/declarations/structures.rs",
    find: "                    .or_else(|| self.cell_item(ty, Some(field)).and_then(|item| self.option(item))),\n",
    replace: "                    .or(None),\n",
    tests: ["test/declarations-cells.test.ts"],
  },
  {
    name: "nested-field-declared-undefined",
    breaks: "a nullable `Option<Option<T>>` field is declared `T | undefined`, where JS holds `null` too (#470)",
    file: "src/lower/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || value || self.null_fields.contains(&field.did) {\n",
    tests: ["test/declarations.test.ts", "-t", "declarations take JS's null"],
  },
  {
    name: "bound-methods-undeclared",
    breaks:
      "a struct's methods its impl binds of the JS object, `isPersistent()`, aren't in its interface, narrower than upstream's type (rd144)",
    file: "src/lower/declarations/structures.rs",
    find: "        for (method, name) in crate::lower::bindings::bound_methods(self.tcx, def_id) {\n",
    replace: "        for (method, name) in crate::lower::bindings::bound_methods(self.tcx, def_id).into_iter().take(0) {\n",
    tests: ["test/declarations-bound-methods.test.ts"],
  },
  {
    name: "declared-option-required",
    breaks:
      "an `Option` field is a required prop to TypeScript, `text: string`, which a caller leaving it out is an error of",
    file: "src/lower/declarations/structures.rs",
    find: "                        false => (true, inner),\n",
    replace: "                        false => (false, inner),\n",
    tests,
  },
  {
    name: "declared-rest-dropped",
    breaks:
      "props a struct doesn't name, `...rest`, are an error to TypeScript, `aria-label` of a link",
    file: "src/lower/declarations/structures.rs",
    find: "            if is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty() && intersected.is_none()) {",
    replace: "            if false && (is_rest(self.tcx, ty) || (is_flatten(self.tcx, field) && extends.is_empty() && intersected.is_none())) {",
    tests,
  },
  {
    name: "flattened-declared-nested",
    breaks:
      "a flattened field is declared a field, `anchor: Anchor`, not `extends Anchor`",
    file: "src/lower/declarations/structures.rs",
    find: "                    (false, 0) => vec![inner],",
    replace: "                    (false, 0) => Vec::new(),",
    tests,
  },
  {
    name: "flattened-extends-any",
    breaks:
      "a flattened field of another module's type of the crate's own, `pub(crate) struct Far`, is `extends any`, which TypeScript refuses",
    file: "src/lower/declarations/structures.rs",
    find: "                    (true, _) => Vec::new(),",
    replace: "                    (true, _) => vec![inner],",
    tests: ["test/declarations-flatten.test.ts"],
  },
  {
    name: "shadowed-names-not-omitted",
    breaks:
      'props extend what they shadow, not `Omit<Linked, "href" | "className">`',
    file: "src/lower/declarations/structures.rs",
    find: '                    (false, _) => vec![reference(\n                        "Omit",',
    replace:
      '                    (false, _) => vec![inner.clone(), reference(\n                        "Omit",',
    tests,
  },
  {
    name: "defaulted-field-required",
    breaks:
      "a field with a default is declared required, `count: number`, where a caller may leave it out",
    file: "src/lower/declarations/structures.rs",
    find: "                    field_default(self.tcx, field).is_some(),\n                    self.as_written(written, |out| out.field_ts(ty, Some(field))),",
    replace: "                    false,\n                    self.as_written(written, |out| out.field_ts(ty, Some(field))),",
    tests,
  },
  {
    name: "option-field-without-null",
    breaks:
      "a `#[rust_js::nullable]` field is declared `?: T`, so a TypeScript caller's `{ label: null }`, which rust-js reads as `None`, doesn't typecheck",
    file: "src/lower/declarations/structures.rs",
    find: '                        true => (!present, json!({ "kind": "union", "types": [inner, keyword("null")] })),\n',
    replace: "                        true => (!present, inner),\n",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "every-option-field-nullable",
    breaks:
      "every `Option` field is declared `?: T | null`, so a TypeScript caller reading one passes `null` where a default takes only `undefined`",
    file: "src/lower/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match true || nullable {\n",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "optional-field-refuses-undefined",
    breaks:
      "a field a caller may leave out is `?: T`, which refuses `undefined` under `exactOptionalPropertyTypes`, where Rust takes it as `None`",
    file: "src/lower/declarations/structures.rs",
    find: "            let ty = if optional { or_undefined(ty) } else { ty };\n",
    replace: "",
    tests: ["test/declarations.test.ts", "-t", "null for an Option"],
  },
  {
    name: "transparent-declared-as-struct",
    breaks:
      "a `#[repr(transparent)]` struct is declared an interface of its fields, where its JS is its field",
    file: "src/lower/declarations/structures.rs",
    find: "        if let Some(at) = crate::lower::recognition::transparent_field(self.tcx, adt) {\n",
    replace: "        if let Some(at) = crate::lower::recognition::transparent_field(self.tcx, adt).filter(|_| false) {\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "transparent struct is declared",
    ],
  },
  {
    name: "nullable-cell-declared-undefined",
    breaks:
      "a nullable `Cell` field is declared `T | undefined`, where JS has its `None` `null`",
    file: "src/lower/declarations/structures.rs",
    find: "                true => crate::lower::bindings::nullable_option(self.tcx, ty).and_then(|option| self.option(option)),\n",
    replace: "                true => self.option(ty),\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "nullable Cell field's None is null",
    ],
  },
  {
    name: "once-field-declared-a-cell",
    breaks:
      "a `OnceCell` field is declared as one that's always there, where it's absent until it's set, `ioInfo?: string | undefined`",
    file: "src/lower/declarations/structures.rs",
    find: "                false => (self.option(ty).or_else(|| self.once_cell_item(ty)))\n",
    replace: "                false => (self.option(ty))\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "object-methods-undeclared",
    breaks:
      "an object of methods' interface declares its fields alone, so TypeScript has no `scope.containsNode(..)` (ADR 0943)",
    file: "src/lower/declarations/structures.rs",
    find: "            if !self.tcx.visibility(method).is_public() {\n",
    replace: "            if true {\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "an object's methods are its literal's own",
    ],
  },
  {
    name: "js-null-fields-unread",
    breaks:
      "a plain `Option` field a `js::null()` is written to is declared `?: T | undefined`, though it holds `null` (ADR 0935, amended)",
    file: "src/lower/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || nested || value {\n",
    tests: ["test/lowering-nullable-constants.test.ts", "-t", "is declared"],
  },
  {
    name: "field-symbol-private-undeclared",
    breaks: "a module's own symbol a declared field is named by isn't declared, `[RESPONSE_SYMBOL]` of nothing TypeScript knows",
    file: "src/lower/declarations/structures.rs",
    find: "                self.unexported.insert(name.clone(), Some(declared));\n            }\n            return json!({ \"computed\": name });",
    replace: "            }\n            return json!({ \"computed\": name });",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a field a symbol names is that symbol's key"
    ]
  },
  {
    name: "field-symbol-declared-by-text",
    breaks: "a declared field a symbol names is declared by the symbol's text, a string key",
    file: "src/lower/declarations/structures.rs",
    find: "            return json!({ \"computed\": crate::lower::bindings::js_name(self.tcx, item) });",
    replace: "            return json!(crate::lower::bindings::js_name(self.tcx, item));",
    tests: [
      "test/field-symbol-names.test.ts",
      "-t",
      "a closure a symbol names is its method"
    ]
  },
  {
    name: "constructed-declared-optional",
    breaks: "a nullable field each constructor of its struct sets first is declared optional, `moduleLoading?: unknown | null | undefined`, though it's present (rd628)",
    file: "src/lower/declarations/structures.rs",
    find: "                        true => (!present, json!({ \"kind\": \"union\", \"types\": [inner, keyword(\"null\")] })),\n",
    replace: "                        true => (!present || true, json!({ \"kind\": \"union\", \"types\": [inner, keyword(\"null\")] })),\n",
    tests: ["test/js-constructors.test.ts", "-t", "a nullable field each constructor sets first"],
  },
  {
    name: "value-field-declared-optional",
    breaks:
      "a nullable field of what may be `undefined` is declared optional, `serverModuleMap?: ServerManifest | null | undefined` (rd645)",
    file: "src/lower/declarations/structures.rs",
    find: "                    let present = matches!(\n                        declared,\n                        crate::lower::representation::NoneForm::Null | crate::lower::representation::NoneForm::Value\n                    );\n",
    replace: "                    let present = matches!(declared, crate::lower::representation::NoneForm::Null);\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "value-field-declared-without-null",
    breaks:
      "a nullable field of what may be `undefined` is declared without `| null` (rd645)",
    file: "src/lower/declarations/structures.rs",
    find: "                    match nullable || nested || value || self.null_fields.contains(&field.did) {\n",
    replace: "                    match nullable || nested || self.null_fields.contains(&field.did) {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "flattened-param-extended",
    breaks: "a struct of a flattened type parameter is declared an interface extending it, which TypeScript refuses (#504)",
    file: "src/lower/declarations/structures.rs",
    find: "                ty::Param(_) => true,\n                ty::Adt(..) => self.flattened.enum_of_keys(*ty),\n",
    replace: "                ty::Param(_) => false,\n                ty::Adt(..) => self.flattened.enum_of_keys(*ty),\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "intersection"],
  },
  {
    name: "js-given-nullable-field-declared-optional",
    breaks:
      "a nullable field of a struct JS may give is declared optional, `moduleLoading?: unknown | null | undefined`, where `nullable` says JS gives `null`, never `undefined`, as upstream's `moduleLoading: ModuleLoading` does (ADR 1072, amended)",
    file: "src/lower/declarations/structures.rs",
    find: "                    let present = matches!(\n                        declared,\n                        crate::lower::representation::NoneForm::Null | crate::lower::representation::NoneForm::Value\n                    );\n",
    replace: "                    let present = matches!(declared, crate::lower::representation::NoneForm::Value);\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "only the crate makes its struct"],
  },
  {
    name: "struct-field-unfilled",
    breaks: "a struct's `dyn Fn` field is named by its types, though its literals name it",
    file: "src/lower/declarations/structures.rs",
    find: "            let outer = self.function_names.len();\n            self.fill_names(field.did, ty);\n",
    replace: "            let outer = self.function_names.len();\n",
    tests: ["test/declarations-function-types.test.ts"],
  },
  {
    name: "ungiven-object-method-declared",
    breaks: "an object of methods' method whose JS takes a fact, `scope.named(TName)`, is declared `named<T>(): string` (#559)",
    file: "src/lower/declarations/structures.rs",
    find: "            if self.leave_out(method, &name) {\n",
    replace: "            if false && self.leave_out(method, &name) {\n",
    tests: ["test/declarations-ungiven.test.ts"],
  },
];

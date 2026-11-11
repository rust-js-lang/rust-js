// Mutations of src/lower/declarations/enums.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const tests = ["test/declarations.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "numbered-declared-names",
    breaks:
      'a numbered enum is declared `"Empty" | ..`, its names, where its JS is its numbers',
    file: "src/lower/declarations/enums.rs",
    find: "            crate::js::ExprKind::Num(n) => json!(n as i64),\n",
    replace: "",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "a repr enum's variant is its number",
    ],
  },
  {
    name: "untagged-unit-undeclared",
    breaks:
      "a variant without fields isn't its literal in the union, `boolean | undefined`",
    file: "src/lower/declarations/enums.rs",
    find: '                            _ => json!({ "kind": "literal", "value": variant_name(self.tcx, v) }),',
    replace:
      '                            _ => json!({ "kind": "keyword", "name": "undefined" }),',
    tests: ["test/declarations.test.ts"],
  },
  {
    name: "untagged-null-declared-literal",
    breaks:
      "an untagged enum's `#[rust_js::null]` variant is declared its name's string, not `null`",
    file: "src/lower/declarations/enums.rs",
    find: '                            Some(crate::js::ExprKind::Null) => keyword("null"),\n',
    replace: "",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "enum-fields-declared-any",
    breaks:
      "an enum with fields is declared `any`, not its tagged objects' union",
    file: "src/lower/declarations/enums.rs",
    find: "            false => {\n                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);",
    replace:
      '            false if true => keyword("any"),\n            false => {\n                let args = ty::GenericArgs::identity_for_item(self.tcx, def_id);',
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "declared-tagged-unit-variant-named",
    breaks:
      "a tagged enum's variant of no fields is declared its name, where JS has an object of its tag",
    file: "src/lower/declarations/enums.rs",
    find: "                        if v.fields.is_empty() && !tagged && !keyed {",
    replace: "                        if v.fields.is_empty() {",
    tests,
  },
  {
    name: "untagged-declared-any",
    breaks:
      "an untagged enum is declared as a tagged one's objects, not TS's union of its payloads",
    file: "src/lower/declarations/enums.rs",
    find: "        let union = match untagged {",
    replace: "        let union = match false {",
    tests: ["test/declarations.test.ts", "-t", "type what a module exports"],
  },
  {
    name: "otherwise-declared-tagged",
    breaks:
      "a discriminated union's `otherwise` is declared `{ kind: \"Other\"; _0: any }`, where it's the object",
    file: "src/lower/declarations/enums.rs",
    find: "                            && is_otherwise(self.tcx, v.def_id)\n",
    replace: "                            && false\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "declarations type what a module exports",
    ],
  },
  {
    name: "bool-tag-declared-string",
    breaks: 'a boolean tag is declared the string `"true"`',
    file: "src/lower/declarations/enums.rs",
    find: "            crate::js::ExprKind::Bool(b) => json!(b),\n",
    replace:
      "            crate::js::ExprKind::Bool(b) => json!(b.to_string()),\n",
    tests: ["test/declarations.test.ts"],
  },
  {
    name: "keyed-declared-tagged",
    breaks: "a keyed enum is declared with its `TAG`, which its objects haven't (#504)",
    file: "src/lower/declarations/enums.rs",
    find: "                let keyed = self.flattened.keyed.contains(&def_id);\n",
    replace: "                let keyed = false;\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "intersection"],
  },
  {
    name: "variant-refcell-declared-box",
    breaks: "a variant's `RefCell` field that holds its value is declared `{ value: T }`, where it's the `T` (#503)",
    file: "src/lower/declarations/enums.rs",
    find: "                            let ty = self.field_ts(ty, Some(field));\n",
    replace: "                            let ty = self.field_ts(ty, None);\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "an enum variant's RefCell field"],
  },
  {
    name: "variant-field-unfilled",
    breaks: "a variant's `dyn Fn` field, `getter: Box<dyn Fn(Option<u32>) -> u32>`, is named by its types, though its literal names it `previous_value`",
    file: "src/lower/declarations/enums.rs",
    find: "                            self.fill_names(field.did, ty);\n",
    replace: "",
    tests: ["test/declarations-function-types.test.ts"],
  },
];

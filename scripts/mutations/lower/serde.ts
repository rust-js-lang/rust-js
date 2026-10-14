// Mutations of src/lower/serde.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const examples = ["test/snapshots.test.ts", "-t", "wire|inbox|api"];
const renaming = ["test/serde.test.ts", "-t", "renaming rules of variants"];

export const mutations: Mutation[] = [
  {
    name: "serde-snake-unseparated",
    breaks: "`rename_all = \"snake_case\"` writes `TextMessage` as `textmessage`, not `text_message`",
    file: "src/lower/serde.rs",
    find: "                        snake.push('_');\n",
    replace: "",
    tests: renaming,
  },
  {
    name: "serde-uppercase-kept",
    breaks: "`rename_all = \"UPPERCASE\"` writes a variant as it's named",
    file: "src/lower/serde.rs",
    find: "            Rule::Upper => variant.to_ascii_uppercase(),\n",
    replace: "            Rule::Upper => variant.to_owned(),\n",
    tests: renaming,
  },
  {
    name: "serde-struct-tag-unwritten",
    breaks: "a struct's `#[serde(tag = \"kind\")]` writes no `kind` key",
    file: "src/lower/serde.rs",
    find: "                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n            self.emit(json, \"key\", vec![Expr::str(tag.as_str())], out);\n",
    replace: "                .unwrap_or_else(|| self.tcx.item_name(adt.did()).to_string());\n",
    tests: renaming,
  },
  {
    name: "serde-camel-case-unknown",
    breaks: "`rename_all = \"camelCase\"` isn't known, and its fields are written as they're named",
    file: "src/lower/serde.rs",
    find: "            \"camelCase\" => Rule::Camel,\n",
    replace: "",
    tests: examples,
  },
];

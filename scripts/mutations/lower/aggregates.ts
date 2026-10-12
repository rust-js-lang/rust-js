// Mutations of src/lower/aggregates.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "replaced-node-default-made",
    breaks: "an update's base makes the default of the children it gives, `C::default()`, which needs a dictionary",
    file: "src/lower/aggregates.rs",
    find: "            let value = if replaced && self.is_node_param(field_ty) {",
    replace: "            let value = if false && replaced && self.is_node_param(field_ty) {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "replaced-default-kept",
    breaks: "an update's base keeps the default of a field it gives, `tags: []`, so the base is kept, `base.id`",
    file: "src/lower/aggregates.rs",
    find: "                if replaced && !value.has_effects() {",
    replace: "                if false && replaced && !value.has_effects() {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "untagged-constructor-tagged",
    breaks: "`.map(Src::Number)` makes tagged objects of an untagged enum's values",
    file: "src/lower/aggregates.rs",
    find: "        let value = if self.untagged(ty).is_some() || bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {\n",
    replace: "        let value = if bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {\n",
    tests: ["test/compiler.test.ts", "test/lowering.test.ts", "-t", "untagged enum"],
  },
  {
    name: "otherwise-constructor-tagged",
    breaks: "`.map(Message::Other)` makes `{ type: \"Other\", _0: value }` of each, not the object",
    file: "src/lower/aggregates.rs",
    find: "        let value = if self.untagged(ty).is_some() || bindings::is_tagged_otherwise(self.tcx, adt_def.did(), variant) {\n",
    replace: "        let value = if self.untagged(ty).is_some() {\n",
    tests: ["test/lowering.test.ts", "-t", "otherwise variant is any other"],
  },
  {
    name: "flattened-made-spread-literal",
    breaks: "a flattened struct made in place is spread, `{ ...{ line: 1 } }`, not its fields",
    file: "src/lower/aggregates.rs",
    find: "                    js::ExprKind::Object(inner) => flat.extend(inner),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","flattened field made outside JSX"],
  },
  {
    name: "adt-value-ordering-tag",
    breaks: "an `Ordering` MIR makes is its variant's name, not -1, 0 or 1",
    file: "src/lower/aggregates.rs",
    find: "        if let Some(n) = ordering_value(self.tcx, adt_def.did(), variant.name) {\n",
    replace: "        if false && let Some(n) = ordering_value(self.tcx, adt_def.did(), variant.name) {\n",
    tests: ["test/corpus.test.ts","-t","collection_methods"],
  },
  {
    name: "adt-value-fmt-ok-thrown",
    breaks: "a `fmt::Result`'s `Ok` from MIR is thrown as an `Err` is",
    file: "src/lower/aggregates.rs",
    find: "                \"Ok\" => Ok(Expr::undefined()),\n                _ if self.is_fmt_result(ty) => {\n",
    replace: "                \"Ok\" if false => Ok(Expr::undefined()),\n                _ if self.is_fmt_result(ty) => {\n",
    tests: ["test/corpus.test.ts","-t","write_to_string"],
  },
  {
    "name": "nulled-field-coalesced",
    "breaks": "what MIR says is `null` or a value already is `?? null` given to a `#[rust_js::nullable]` field",
    "file": "src/lower/aggregates.rs",
    "find": " || nulled.get(i) == Some(&true) => made,\n",
    "replace": " => made,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a nullable field's None is null"
    ]
  },
];

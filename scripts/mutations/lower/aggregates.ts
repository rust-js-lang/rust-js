// Mutations of src/lower/aggregates.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "named-unit-struct-undefined",
    breaks: "a unit struct named `#[rust_js::name]`, `webapi`'s `Click`, is `undefined`, not its string",
    file: "src/lower/aggregates.rs",
    find: "            return Ok(bindings::unit_name(self.tcx, adt.adt_def.did()).map_or_else(Expr::undefined, Expr::str));",
    replace: "            return Ok(Expr::undefined());",
    tests: ["test/compiler.test.ts", "-t", "named unit struct"],
  },
  {
    name: "failing-user-writer-allowed",
    breaks: "a `fmt::Write` of the crate's that fails is compiled, given each `write!`'s text whole where Rust fails at a piece",
    file: "src/lower/aggregates.rs",
    find: "                        .is_some_and(|tr| is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    replace: "                        .is_some_and(|tr| false && is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    tests: ["test/diagnostics.test.ts","-t","a writer of the crate's that fails"],
  },
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
    name: "derived-default-base-lowered",
    breaks: "an update's derived `Default` base is lowered whole, its children's default too",
    file: "src/lower/aggregates.rs",
    find: "                    let derived = match self.derived_default_fields(fru.base, ty) {",
    replace: "                    let derived = match None::<Vec<(String, Ty<'tcx>)>> {",
    tests: ["test/jsx.test.ts","-t","and the component no dictionary"],
  },
  {
    name: "update-defaults-in-base",
    breaks: "an update's derived defaults are kept in a base, `const base = { n: 0, .. }`, and read from it, `base.items`",
    file: "src/lower/aggregates.rs",
    find: "                            if !kept {",
    replace: "                            if false && !kept {",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
    snapshots: true,
  },
  {
    name: "untagged-variant-tagged",
    breaks: "an untagged enum's variant is a tagged object, `{ TAG: \"Text\", _0: s }`, which no JS API takes",
    file: "src/lower/aggregates.rs",
    find: "        // An untagged enum's variant is its payload (ADR 0214).\n        if self.untagged(ty).is_some() {\n",
    replace: "        // An untagged enum's variant is its payload (ADR 0214).\n        if false {\n",
    tests: ["test/compiler.test.ts", "-t", "untagged enum"],
  },
  {
    name: "untagged-constructor-tagged",
    breaks: "`.map(Src::Number)` makes tagged objects of an untagged enum's values",
    file: "src/lower/aggregates.rs",
    find: "        let value = if self.untagged(ty).is_some() {\n",
    replace: "        let value = if false {\n",
    tests: ["test/compiler.test.ts", "-t", "untagged enum"],
  },
  {
    name: "tuple-field-named",
    breaks: "a tuple struct's field read first is named `0`, `const 0 = ..`, which no JS variable is",
    file: "src/lower/aggregates.rs",
    find: "        false => \"tmp\".to_string(),",
    replace: "        false => field.to_string(),",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "drop_functions"
    ]
  },
  {
    name: "ref-base-field-by-field",
    breaks: "`..*props` through a reference is each field read, `title={props.title}`, not `{...props}`, and a key its type doesn't name is lost",
    file: "src/lower/aggregates.rs",
    find: "        if tag.is_none()\n            && let (Some(base), Shape::Object(fields)) = (&base, &shape)",
    replace: "        if false\n            && let (Some(base), Shape::Object(fields)) = (&base, &shape)",
    tests: ["test/jsx.test.ts","-t","updated from a reference are spread"],
  },
  {
    name: "pure-defaults-wait-for-fields",
    breaks: "a field made by a call, beside defaults that do nothing, is made first in a `const`, `const class_name = classes()`",
    file: "src/lower/aggregates.rs",
    find: "let value = if defaults_act && value.has_effects() {",
    replace: "let value = if value.has_effects() {",
    tests: ["test/jsx.test.ts", "-t", "flattened prop made by a call"],
  },
];

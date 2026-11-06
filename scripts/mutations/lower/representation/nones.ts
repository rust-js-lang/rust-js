// Mutations of src/lower/representation/nones.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

// The corpus's cases of how a place holds `None`.
const nones = ["test/corpus.test.ts", "-t", "null|none|nested_option|optional"];

export const mutations: Mutation[] = [
  {
    name: "nullable-place-stores-undefined",
    breaks: "a `#[rust_js::nullable]` place stores what's written as it's given, a `None` `undefined`, which its reads' `=== null` miss (#819)",
    file: "src/lower/representation/nones.rs",
    find: "            NoneForm::Null | NoneForm::Optional => true,\n",
    replace: "            NoneForm::Optional => true,\n            NoneForm::Null => false,\n",
    tests: nones,
  },
  {
    name: "optional-field-none-as-null",
    breaks: "a nullable optional field is taken for a nullable one, so a literal's `None` is `null`, where it's left out, `undefined` (ADR 0810, #819)",
    file: "src/lower/representation/nones.rs",
    find: "        (true, true) => NoneForm::Optional,\n",
    replace: "        (true, true) => NoneForm::Null,\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "an optional nullable field"],
  },
  {
    name: "held-value-field-none-undefined",
    breaks: "a nullable field of a value never `null` that may be `undefined` is taken for a plain one, so its default is `undefined`, not `null` (rd645, #819)",
    file: "src/lower/representation/nones.rs",
    find: "        return NoneForm::Value;\n",
    replace: "        return NoneForm::Undefined;\n",
    tests: nones,
  },
  {
    name: "nested-some-none-undefined",
    breaks: "a nested `Option`'s `Some(None)` is stored `undefined`, its `None`, not `null` (ADR 1017, #819)",
    file: "src/lower/representation/nones.rs",
    find: "            NoneForm::Nested => NoneForm::Null,\n",
    replace: "            NoneForm::Nested => NoneForm::Undefined,\n",
    tests: nones,
  },
  {
    name: "nullable-binding-arg-undefined",
    breaks: "a binding's argument it names `#[rust_js::nullable(..)]` is given its `None` `undefined`, not `null` (ADR 0275, #819)",
    file: "src/lower/representation/nones.rs",
    find: "    NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i) && bindings::holds_none_as_null(tcx, payload))\n",
    replace: "    NoneForm::null_if(false && bindings::holds_none_as_null(tcx, payload))\n",
    tests: nones,
  },
  {
    name: "nullable-param-undefined",
    breaks: "a parameter its function names `#[rust_js::nullable(..)]` is set `undefined` for `None`, which its body's `=== null` misses (ADR 0861, #819)",
    file: "src/lower/representation/nones.rs",
    find: "    NoneForm::null_if(bindings::nullable_params(tcx, def_id).contains(&name))\n",
    replace: "    NoneForm::null_if(false && bindings::nullable_params(tcx, def_id).contains(&name))\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "own nullable parameter"],
  },
  {
    name: "any-value-set-nulled",
    breaks: "a map of any value's `set` writes `value ?? null` (#443)",
    file: "src/lower/representation/nones.rs",
    find: "    NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i) && bindings::holds_none_as_null(tcx, payload))\n",
    replace: "    NoneForm::null_if(bindings::takes_nullable(tcx, def_id, i))\n",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "a Map of any value holds undefined as it is",
    ],
  },
];

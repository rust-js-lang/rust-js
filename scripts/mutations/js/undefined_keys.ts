// Mutations of src/js/undefined_keys.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "kept-none-left-out",
    breaks: "a `None` field whose key its object keeps, a comment's or a cell's, is left out, `v: undefined, // why` and `destroy: undefined` gone",
    file: "src/js/undefined_keys.rs",
    find: " && !layout.kept.contains(key))",
    replace: ")",
    tests: ["test/lowering-field-cells.test.ts", "-t", "a cell field made None keeps its key"],
  },
  {
    name: "undefined-fields-kept",
    breaks: "`{ code, hidden: true, read_only: undefined }`, a `None`'s key",
    file: "src/js/undefined_keys.rs",
    find: "    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {\n",
    replace: "    if true {\n",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
  {
    name: "undefined-after-spread-dropped",
    breaks: "`{ ...base, read_only: undefined }` is `{ ...base }`, base's `Some`",
    file: "src/js/undefined_keys.rs",
    find: "    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {\n",
    replace: "    if false {\n",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
];

// Mutations of src/lower/pipeline/holes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "item-hole-filled-once",
    breaks:
      "an item written in a body is declared in the first copy of its function alone, so a copy its crate's Rust calls, `evensRevealed`, reads a nested fn nothing declares (#2018)",
    file: "src/lower/pipeline/holes.rs",
    find: "            if let Some(function) = holes.functions.get(&index).cloned() {\n",
    replace: "            if let Some(function) = holes.functions.swap_remove(&index) {\n",
    tests: ["test/corpus.test.ts", "-t", "^items_written_in_a_function_compiled_twice.rs$"],
  },
  {
    name: "written-fn-comment-lost",
    breaks:
      "a function written in a body is declared with no span, so the comment before it is lost",
    file: "src/lower/pipeline/holes.rs",
    find: "                stmt.span = function.span;\n",
    replace: "",
    tests: ["test/js-object.test.ts", "-t", "a constructor is a fn item"],
  },
  {
    name: "local-const-hole-unfilled",
    breaks:
      "a `const` written in a function's body is never filled into its hole, which the printer refuses",
    file: "src/lower/pipeline/holes.rs",
    find: "            } else if let Some(constant) = holes.consts.get(&index).cloned() {\n",
    replace:
      "            } else if let Some(constant) = holes.consts.swap_remove(&index).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
];

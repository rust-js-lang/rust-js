// Mutations of src/lower/pipeline/holes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
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
    find: "            } else if let Some(constant) = holes.consts.remove(&index) {\n",
    replace:
      "            } else if let Some(constant) = holes.consts.remove(&index).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^local_consts.rs$"],
  },
];

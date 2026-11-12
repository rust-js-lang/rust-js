// Mutations of src/lower/mir/assign/rewritten.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inner-object-not-put-back",
    breaks: "an object inside a place written whole is written over but not put back in the value, so the write over the whole replaces it (#1484)",
    file: "src/lower/mir/assign/rewritten.rs",
    find: "                            out.push(StmtKind::Assign(of_value, inner_js).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
];

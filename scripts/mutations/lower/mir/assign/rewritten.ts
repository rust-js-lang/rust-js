// Mutations of src/lower/mir/assign/rewritten.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inner-object-not-put-back",
    breaks: "an object inside a place written whole is written over but not put back in the value, so the write over the whole replaces it (#1484)",
    file: "src/lower/mir/assign/rewritten.rs",
    find: "                    written.push(StmtKind::Assign(of_value, inner_js).at(js_span));\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^address_kept_when_assigned_again.rs$"],
  },
  {
    name: "unmade-left-unmade",
    breaks: "a place whose address is taken, assigned where it may be unmade, is left `undefined` where it is (#1500)",
    file: "src/lower/mir/assign/rewritten.rs",
    find: "                    vec![StmtKind::Assign(target, value).at(js_span)],\n",
    replace: "                    vec![],\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_local_moved_out.rs$"],
  },
];

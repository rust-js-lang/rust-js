// Mutations of src/lower/mir/tried_ints.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tried-int-not-held",
    breaks:
      "a `try_from` of what may change, `counter.at`, is read at its test and again where its `Ok` is, not held once, `const value = counter.at` (#366)",
    file: "src/lower/mir/tried_ints.rs",
    find: "        n.visit_vars(&mut |name| plain &= self.plain_local(name));\n",
    replace:
      "        n.visit_vars(&mut |name| plain &= self.plain_local(name) || true);\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
    snapshots: true,
  },
];

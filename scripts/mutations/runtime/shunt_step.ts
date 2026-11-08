// Mutations of src/runtime/shunt_step.js (ADR 0093): a collect into a
// `Result` or an `Option` stepped as std's `GenericShunt` steps it (#1004).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shunt-steps-by-next",
    breaks: "a collect into an `Option` or a `Result` steps its source by `next`, so a `skip(1)` of the crate's iterator asks its `nth(1)` where std's `GenericShunt` asks `nth(0)` through its `try_fold` (#1004)",
    file: "src/runtime/shunt_step.js",
    find: '  if (typeof it.tryFold !== "function") return it.next();\n',
    replace: "  return it.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^shunt_steps_by_try_fold.rs$"],
    snapshots: true,
  },
  {
    name: "shunt-end-left-open",
    breaks: "a collect into an `Option` or a `Result` whose source a `take_while` stopped leaves it open, so what it holds isn't dropped where the collect's temporary is (#1004)",
    file: "src/runtime/shunt_step.js",
    find: "  if (!it.held) it.return?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^shunt_steps_by_try_fold.rs$"],
  },
];

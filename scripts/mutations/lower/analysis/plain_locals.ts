// Mutations of src/lower/analysis/plain_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-locals-boxed",
    breaks: "a thread-local only read and set in its own module keeps its `{ value }`, `const COUNT = { value: 0 }`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "keys.filter(|&key| !tcx.visibility(key).is_public())",
    replace: "keys.filter(|_| false)",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "fn-given-never-quiet",
    breaks: "a function given to `with_borrow` keeps its thread-local a cell, React's scheduler's `CURRENT_TASK.with_borrow(Clone::clone)` a `$withBorrow`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                        _ => fn_def(thir[args[1]].ty).map(|(f, f_args)| Given::Fn(f, f_args)),\n",
    replace: "                        _ => None,\n",
    tests: ["test/lowering-cells.test.ts", "-t", "given a function to run"],
  },
];

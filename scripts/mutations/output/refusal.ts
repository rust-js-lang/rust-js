// Mutations of src/output/refusal.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tree-unvalidated",
    breaks: "a module's tree isn't checked before it's printed: a read nothing declares is printed, a `ReferenceError` where it runs, not refused as rust-js's mistake (#1681)",
    file: "src/output/refusal.rs",
    find: "    let problem = js::validate::module(module).into_iter().next()?;\n",
    replace: "    let problem = js::validate::module(module).into_iter().next().filter(|_| false)?;\n",
    unitTests: "output::refusal::tests::a_read_nothing_declares_is_refused_naming_what_broke_it",
  },
];

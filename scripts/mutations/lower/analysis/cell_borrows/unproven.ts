// Mutations of crates/rust-js-lower/src/analysis/cell_borrows/unproven.rs (#2490).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unproven-call-of-the-crate-passed",
    breaks: "`CellForm`'s proof alone passes a call of the crate's function while a guard is held, as if it ran std's code alone (#2490)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/unproven.rs",
    find: "            Called::Body(..) | Called::Bodies(_) => Some(Unproven::CrateFn),\n",
    replace: "            Called::Body(..) | Called::Bodies(_) => None,\n",
    tests: ["test/cell-form-shadow.test.ts", "-t", "the summaries prove quiet"],
  },
];

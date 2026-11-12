// Mutations of src/lower/mir/tidy/fired.rs (#1461).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tidy-fired-every-pass",
    breaks: "`--tidy-fired` says each pass that ran fired, whether it changed a body or not, so no pass is ever seen to change nothing (#1461)",
    file: "src/lower/mir/tidy/fired.rs",
    find: "    if was.is_some_and(|was| was != body) {\n",
    replace: "    if was.is_some() {\n",
    tests: ["test/tidy-stages.test.ts", "-t", "tidy-fired"],
  },
];

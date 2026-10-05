// Mutations of src/lower/aggregates.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "failing-user-writer-allowed",
    breaks: "a `fmt::Write` of the crate's that fails is compiled, given each `write!`'s text whole where Rust fails at a piece",
    file: "src/lower/aggregates.rs",
    find: "                        .is_some_and(|tr| is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    replace: "                        .is_some_and(|tr| false && is_std_def(self.tcx, tr, StdItem::FmtWrite)) =>",
    tests: ["test/diagnostics.test.ts","-t","a writer of the crate's that fails"],
  },
];

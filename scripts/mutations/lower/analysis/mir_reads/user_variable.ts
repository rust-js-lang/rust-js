// crates/rust-js-lower/src/analysis/mir_reads/user_variable.rs's mutations (#2299 moved them with their code).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "user-variable-of-a-cleared-body-asked",
    breaks: "a local of a body whose locals' kinds rustc cleared, a `const fn` evaluated for a const generic's argument, is asked if the program named it, and rust-js panics, \"unwrapping cross-crate data\" (#1281)",
    file: "crates/rust-js-lower/src/analysis/mir_reads/user_variable.rs",
    find: "    matches!(decl.local_info, mir::ClearCrossCrate::Set(_)) && decl.is_user_variable()\n",
    replace: "    decl.is_user_variable()\n",
    tests: ["test/corpus.test.ts", "-t", "const_fn_evaluated_for_a_const_generic"],
  },
];

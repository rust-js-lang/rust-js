// Mutations of src/std_sysroot.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-core-aborts",
    breaks: "std's bodies are lowered from a `core` built to abort, whose calls unwind nowhere, so its compiled bodies drop nothing a panic unwinds past (#424)",
    file: "src/std_sysroot.rs",
    find: "        \"-Cpanic=unwind\".to_string(),\n",
    replace: "        \"-Cpanic=abort\".to_string(),\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },

];

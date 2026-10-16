// Mutations of src/panic_runtime.rs (ADR 0090).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "panic-runtime-aborts",
    breaks: "the panic runtime a test harness is given aborts, as the session's panics don't, and rustc refuses it",
    file: "src/panic_runtime.rs",
    find: "        \"-Cpanic=unwind\".to_string(),\n",
    replace: "        \"-Cpanic=abort\".to_string(),\n",
    tests: ["test/manifest.test.ts", "-t", "where its tests' runner is"],
  },
];

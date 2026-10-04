// Mutations of src/runtime/checked_rem.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "checked-rem-min",
    breaks: "`i32::MIN.checked_rem(-1)` is `Some(0)`, where it overflows",
    file: "src/runtime/checked_rem.js",
    find: "  if (b == 0 || (a === min && b == -1)) return undefined;",
    replace: "  if (b == 0) return undefined;",
    tests: ["test/corpus.test.ts", "-t", "integer_families"],
  },
];

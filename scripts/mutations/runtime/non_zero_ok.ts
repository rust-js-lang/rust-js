// Mutations of src/runtime/non_zero_ok.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "non-zero-of-zero-ok",
    breaks: "`\"0\".parse::<NonZeroU8>()` and `NonZeroU8::try_from(0)` are `Ok(0)`",
    file: "src/runtime/non_zero_ok.js",
    find: '  return result.TAG === "Ok" && result._0 == 0 ? { TAG: "Err", _0: zero } : result;',
    replace: "  return result;",
    tests: ["test/corpus.test.ts", "-t", "nonzero"],
  },
];

// Mutations of src/runtime/try_from_int.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-from-int-one-message",
    breaks: "a `TryFromIntError` below its target's range says it's too large, where 1.99 says too small",
    file: "src/runtime/try_from_int.js",
    find: '  if (x < lo) return { TAG: "Err", _0: "number too small to fit in target type" };',
    replace: '  if (x < lo) return { TAG: "Err", _0: "number too large to fit in target type" };',
    tests: ["test/compiler.test.ts", "-t", "wide.report"],
  },
  {
    name: "u8-try-from-char-any",
    breaks: "`u8::try_from(c)` of a `char` past 255 is `Ok`",
    file: "src/runtime/try_from_int.js",
    find: "  return n > 255 ? { TAG: \"Err\", _0: \"unicode code point out of range\" } : { TAG: \"Ok\", _0: n };\n",
    replace: "  return { TAG: \"Ok\", _0: n };\n",
    tests: ["test/corpus.test.ts", "-t", "^char_try_from.rs$"],
  },
];

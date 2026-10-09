// Mutations of src/runtime/debug_str.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "debug-escapes-halfwidth-voiced-marks",
    breaks: "`{:?}` of `'\\u{ff9e}'` escapes it, as 1.98 did, where 1.99 shows it as it is",
    file: "src/runtime/debug_str.js",
    find: '  if (c === "\\uff9e" || c === "\\uff9f") return c;\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "matrix_chars"],
  },
  {
    name: "debug-shows-default-ignorable",
    breaks: "`{:?}` of U+FFA0, a letter Unicode says to ignore, shows it as it is, where Rust escapes it",
    file: "src/runtime/debug_str.js",
    find: "\\p{Zp}\\p{Default_Ignorable_Code_Point}]",
    replace: "\\p{Zp}]",
    tests: ["test/corpus.test.ts", "-t", "matrix_chars"],
  },
];

// Mutations of src/runtime/duration.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "duration-new-unchecked",
    breaks: "`Duration::new` past `MAX` doesn't panic",
    file: "src/runtime/duration.js",
    find: "  if (duration > 18446744073709551615999999999n) throw new Error(\"overflow in Duration::new\");\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","duration_overflow"],
  },
  {
    name: "duration-debug-zeros-kept",
    breaks: "`{:?}` of `1.25s` keeps its fraction's trailing zeros",
    file: "src/runtime/duration.js",
    find: ".replace(/0+$/, \"\")",
    replace: "",
    tests: ["test/corpus.test.ts","-t","duration"],
  },
  {
    name: "duration-tie-up",
    breaks: "`{:.3?}` of a `Duration` at a tie rounds up, where std's rounds to even",
    file: "src/runtime/duration.js",
    find: "(rest > half || (rest === half && last % 2 === 1))",
    replace: "(rest >= half)",
    tests: ["test/corpus.test.ts", "-t", "^format_options_reached.rs$"],
  },
];

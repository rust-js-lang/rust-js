// Mutations of src/runtime/pretty.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "pretty-not-indented",
    breaks: "a nested part of `{:#?}`'s lines aren't indented",
    file: "src/runtime/pretty.js",
    find: "  const lines = items.map((item) => \"    \" + item.replaceAll(\"\\n\", \"\\n    \") + \",\\n\").join(\"\");",
    replace: "  const lines = items.map((item) => \"    \" + item + \",\\n\").join(\"\");",
    tests: ["test/corpus.test.ts","-t","pretty_debug"],
  },
  {
    name: "pretty-empty-rest-unspaced",
    breaks: "`{:#?}` of a non-exhaustive struct given no field is `Open {..}`, not `Open { .. }`",
    file: "src/runtime/pretty.js",
    find: "    return open + (rest ? \" .. \" : \"\") + close;",
    replace: "    return open + (rest ? \"..\" : \"\") + close;",
    tests: ["test/corpus.test.ts", "-t", "^debug_builder_held.rs$"],
  },
];

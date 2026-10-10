// Mutations of src/runtime/extract_if.js (ADR 0344).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extract-if-end-kept",
    breaks: "`extract_if` runs past its range as it takes items out",
    file: "src/runtime/extract_if.js",
    find: "          end--;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","extract_if"],
  },
  {
    name: "extract-if-unchecked",
    breaks: "`extract_if` past the end is made, and never panics",
    file: "src/runtime/extract_if.js",
    find: "  $checkRange(start, end, v.length);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","extract_if_past_len"],
  },
  {
    name: "extract-if-no-handle",
    breaks: "`extract_if`'s filter is given a number, not a handle on it",
    file: "src/runtime/extract_if.js",
    find: "        if (f(handle ? $mutAt(v, at) : v[at])) {",
    replace: "        if (f(v[at])) {",
    tests: ["test/corpus.test.ts","-t","extract_if"],
  },
  {
    name: "map-extract-taken-stale",
    breaks: "a map's `extract_if` gives the value from before its filter changed it",
    file: "src/runtime/extract_if.js",
    find: "        const taken = m.get(key);",
    replace: "        const taken = value;",
    tests: ["test/corpus.test.ts","-t","extract_if"],
  },
  {
    name: "map-extract-kept",
    breaks: "a map's `extract_if` leaves what it gives in the map",
    file: "src/runtime/extract_if.js",
    find: "        const taken = m.get(key);\n        m.delete(key);",
    replace: "        const taken = m.get(key);",
    tests: ["test/corpus.test.ts","-t","extract_if"],
  },
  {
    name: "set-extract-kept",
    breaks: "a set's `extract_if` leaves what it gives in the set",
    file: "src/runtime/extract_if.js",
    find: "          m.delete(step.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","extract_if"],
  },
];

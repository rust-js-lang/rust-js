// Mutations of src/runtime/str_edits.js (ADR 0323).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-range-end-unchecked",
    breaks: "`replace_range` ending inside a `char` cuts it, not std's panic",
    file: "src/runtime/str_edits.js",
    find: "  if (to === undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "replace_range_panic"],
  },
  {
    name: "str-split-off-swapped",
    breaks: "`split_off` keeps the tail and gives the head",
    file: "src/runtime/str_edits.js",
    find: "  return [s.slice(0, unit), s.slice(unit)];\n",
    replace: "  return [s.slice(unit), s.slice(0, unit)];\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "str-drain-keeps",
    breaks: "`drain` leaves what it takes in the string",
    file: "src/runtime/str_edits.js",
    find: "  return [s.slice(0, from) + s.slice(to), Array.from(s.slice(from, to))];\n",
    replace: "  return [s, Array.from(s.slice(from, to))];\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "str-extend-within-whole",
    breaks: "`extend_from_within` appends the whole string",
    file: "src/runtime/str_edits.js",
    find: "  return s + s.slice(from, to);\n",
    replace: "  return s + s;\n",
    tests: ["test/corpus.test.ts", "-t", "text_edits"],
  },
  {
    name: "str-range-unchecked",
    breaks: "a `String`'s `drain(a..)` with `a` past the end says it's inside a `char`",
    file: "src/runtime/str_edits.js",
    find: "  $checkRange(start, end, length);\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","string_drain_from_past_len"],
  },
];

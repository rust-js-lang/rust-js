// Mutations of src/runtime/str_part.js (ADR 0334).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-part-unchecked",
    breaks: "`&mut s[a..b]` is checked only when it's used, not made",
    file: "src/runtime/str_part.js",
    find: "  $strSlice(cell.value, start, end);\n  return {",
    replace: "  return {",
    tests: ["test/corpus.test.ts","-t","str_part_boundary"],
  },
  {
    name: "str-part-write-whole",
    breaks: "writing a part replaces all of the string",
    file: "src/runtime/str_part.js",
    find: "      cell.value = $strSlice(cell.value, 0, start) + text + $strSlice(cell.value, end);",
    replace: "      cell.value = text;",
    tests: ["test/corpus.test.ts","-t","str_mut"],
  },
  {
    name: "str-get-mut-unchecked",
    breaks: "`get_mut(range)` panics where it's `None`",
    file: "src/runtime/str_part.js",
    find: "  return $strGet(cell.value, start, end) === undefined ? undefined : $strPart(cell, start, end);",
    replace: "  return $strPart(cell, start, end);",
    tests: ["test/corpus.test.ts","-t","str_mut"],
  },
  {
    name: "str-split-at-mut-checked-ignored",
    breaks: "`split_at_mut_checked` panics where it's `None`",
    file: "src/runtime/str_part.js",
    find: "  if (checked && $charBoundary(cell.value, at) === undefined) return undefined;\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","str_mut"],
  },
];

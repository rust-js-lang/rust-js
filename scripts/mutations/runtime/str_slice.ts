// Mutations of src/runtime/str_slice.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-slice-units",
    breaks: "`&s[a..b]` slices UTF-16 units, not UTF-8 bytes",
    file: "src/runtime/str_slice.js",
    find: "  const part = s.slice(from, $unitAt(s, end, \"end\"));",
    replace: "  const part = s.slice(start, end);",
    tests: ["test/corpus.test.ts", "-t", "string_bytes"],
  },
  {
    name: "str-slice-inside-char",
    breaks: "slicing inside a character gives part of it, where Rust panics",
    file: "src/runtime/str_slice.js",
    find: "  if (unit !== undefined) return unit;\n",
    replace: "  if (true) return unit ?? at;\n",
    tests: ["test/corpus.test.ts", "-t", "string_slice_boundary"],
  },
  {
    name: "str-slice-past-end",
    breaks: "slicing past a string's end gives what's there, where Rust panics",
    file: "src/runtime/str_slice.js",
    find: "  if (end > length) throw new Error(`end byte index",
    replace: "  if (false) throw new Error(`end byte index",
    tests: ["test/corpus.test.ts", "-t", "string_slice_bounds"],
  },
  {
    name: "str-slice-part-its-texts-length",
    breaks: "a long part's length in bytes is taken to be where it ends in its text, not its own, so `s[i..].len()` is `s.len()`",
    file: "src/runtime/str_slice.js",
    find: "$textPart(part, s, start, from, end - start);",
    replace: "$textPart(part, s, start, from, end);",
    tests: ["test/corpus.test.ts", "-t", "^long_text_read_byte_by_byte.rs$"],
  },
];

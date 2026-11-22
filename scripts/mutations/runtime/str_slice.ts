// Mutations of crates/rust-js-program/src/runtime/str_slice.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "str-slice-units",
    breaks: "`&s[a..b]` slices UTF-16 units, not UTF-8 bytes",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "  const part = s.slice(from, $unitAt(s, end, \"end\", raisedAt));",
    replace: "  const part = s.slice(start, end);",
    tests: ["test/corpus.test.ts", "-t", "string_bytes"],
  },
  {
    name: "str-slice-inside-char",
    breaks: "slicing inside a character gives part of it, where Rust panics",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "  if (unit !== undefined) return unit;\n",
    replace: "  if (true) return unit ?? at;\n",
    tests: ["test/corpus.test.ts", "-t", "string_slice_boundary"],
  },
  {
    name: "str-slice-past-end",
    breaks: "slicing past a string's end gives what's there, where Rust panics",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "  if (end > length) throw $stringPanic(`end byte index",
    replace: "  if (false) throw $stringPanic(`end byte index",
    tests: ["test/corpus.test.ts", "-t", "string_slice_bounds"],
  },
  {
    name: "str-slice-part-its-texts-length",
    breaks: "a long part's length in bytes is taken to be where it ends in its text, not its own, so `s[i..].len()` is `s.len()`",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "$textPart(part, s, start, from, end - start);",
    replace: "$textPart(part, s, start, from, end);",
    tests: ["test/corpus.test.ts", "-t", "^long_text_read_byte_by_byte.rs$"],
  },
  {
    name: "str-slice-inclusive-end-past",
    breaks: "`s[..=3]` of \"abc\" says \"end byte index 4\", not core's 3 (#1935)",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "  if (past) end -= 1;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_inclusive_range_panics.rs$"],
  },
  {
    name: "str-slice-inclusive-end-at-length-sliced",
    breaks: "`s[..=3]` of \"abc\" is all of it, not a panic, its end `b` checked as an exclusive end (#1935)",
    file: "crates/rust-js-program/src/runtime/str_slice.js",
    find: "  if (past) throw $stringPanic(`end byte index ${end} is out of bounds for string of length ${length}`, raisedAt);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_inclusive_range_panics.rs$"],
  },
];

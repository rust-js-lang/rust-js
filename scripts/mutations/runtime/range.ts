// Mutations of crates/rust-js-program/src/runtime/range.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "range-collect-room-of-its-end",
    breaks: "a range's `collect()` asks room for its end, not its length",
    file: "crates/rust-js-program/src/runtime/range.js",
    find: "  if (size !== undefined) $roomFor(length, size);\n",
    replace: "  if (size !== undefined) $roomFor(end, size);\n",
    tests: ["test/runtime-package.test.ts", "-t", "ends where wasm32"],
  },
  {
    name: "range-past-usize-at-raw-vec",
    breaks: "a collect of `0..=usize::MAX`, whose hint has no upper bound, panics at raw_vec's line, where std's `TrustedLen` collect raises it, `spec_from_iter_nested.rs:57:18` (#2488)",
    file: "crates/rust-js-program/src/runtime/range.js",
    find: "  if (size !== undefined && length > 2 ** 32 - 1) throw $panic(\"capacity overflow\", $stdSites.vecFromTrustedLen[0]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^collect_of_a_range_past_usize_panics_at_trusted_len.rs$"],
  },
];

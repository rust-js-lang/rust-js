// Mutations of crates/rust-js-program/src/runtime/ilog.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "ilog-off-at-power",
    breaks: "`1000u32.ilog10()` is 2: an exact power isn't counted",
    file: "crates/rust-js-program/src/runtime/ilog.js",
    find: "    for (; x >= base; x = Math.floor(x / base)) log++;",
    replace: "    for (; x > base; x = Math.floor(x / base)) log++;",
    tests: ["test/corpus.test.ts", "-t", "number_methods"],
  },
  {
    name: "checked-ilog-panics",
    breaks: "`checked_ilog10()` of zero panics, not `None`",
    file: "crates/rust-js-program/src/runtime/ilog.js",
    find: "  return x <= 0 || base < 2 ? undefined : $ilog(x, base, undefined);",
    replace: "  return $ilog(x, base, undefined);",
    tests: ["test/corpus.test.ts", "-t", "^number_methods.rs$"],
  },
  {
    name: "ilog-base-unchecked",
    breaks: "`ilog(1)` loops or counts, not panicking, and `checked_ilog(1)` isn't none",
    file: "crates/rust-js-program/src/runtime/ilog.js",
    find: "  if (base < 2) throw $panic(\"base of integer logarithm must be at least 2\", raisedAt);\n",
    replace: "",
    tests: ["test/binaries.test.ts", "-t", "program's own code"],
  },
];

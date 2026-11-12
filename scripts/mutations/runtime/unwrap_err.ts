// Mutations of src/runtime/unwrap_err.js (ADR 0179, #1505).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unwrap-err-panicked-over-undropped",
    breaks: "`$unwrapErr` given the `Ok`'s drop never runs it, so `ok(9).unwrap_err()` of an `Ok(Loud(9))` drops nothing as it panics (#1505)",
    file: "src/runtime/unwrap_err.js",
    find: "    if (drop !== undefined) $dropsUnwinding([() => drop(result._0)]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unwrap_panic_drops_what_it_panics_over.rs$"],
  },
];

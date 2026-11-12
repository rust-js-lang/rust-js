// Mutations of src/runtime/unwrap_ok.js (ADR 0179, #1505).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "unwrap-ok-panicked-over-undropped",
    breaks: "`$unwrapOk` given the `Err`'s drop never runs it, so `get(7).unwrap()` of an `Err(Loud(7))` drops nothing as it panics (#1505)",
    file: "src/runtime/unwrap_ok.js",
    find: "    if (drop !== undefined) $dropsUnwinding([() => drop(result._0)]);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unwrap_panic_drops_what_it_panics_over.rs$"],
  },
];

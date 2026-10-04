// Mutations of src/runtime/parse_error_dyn.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-from-int-dyn-shows-kind",
    breaks: "`{}` of a `TryFromIntError` as a `dyn Error` is its kind, `PosOverflow`, not Rust's message",
    file: "src/runtime/parse_error_dyn.js",
    find: '      fmt: (value) => (name === "TryFromIntError" ? "out of range integral type conversion attempted" : value),',
    replace: "      fmt: (value) => value,",
    tests: ["test/corpus.test.ts", "-t", "std_errors_boxed"],
  },
];

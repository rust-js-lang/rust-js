// Mutations of crates/rust-js-lower/src/std_types/location.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [  {
    name: "location-debug-file-unquoted",
    breaks: "`{:?}` of a `Location` shows its file unquoted",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: '"Location {{ file: {file:?}, line: {}, column: {} }}",',
    replace:
      "\"Location {{ file: {file}, line: {}, column: {} }}\",",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
];

// Mutations of crates/rust-js-lower/src/std_types/location.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-shim-named-where-this-machine-keeps-it",
    breaks: "a std shim's location names the file where this machine keeps std's source, not `/rustc/<commit>/library/..`, so the JS differs from machine to machine (#2335, #2343)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "        let std = crate::recognition::std_crate(self.tcx, at.file.cnum).is_some();\n",
    replace: "        let std = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_panic.rs$"],
    snapshots: true,
  },
  {
    name: "location-debug-file-unquoted",
    breaks: "`{:?}` of a `Location` shows its file unquoted",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: '"Location {{ file: {file:?}, line: {}, column: {} }}",',
    replace:
      "\"Location {{ file: {file}, line: {}, column: {} }}\",",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
];

// Mutations of crates/rust-js-lower/src/std_types/location.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "linked-untold-unrefused",
    breaks: "a binary that links a library whose body makes an untold `#[track_caller]` value isn't refused, and writes the library's caller's line, a wrong location (#2335)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "    if !crate::recognition::prints_panic_locations(tcx) {\n        return;\n    }\n    for library in dependencies.libraries.values() {\n",
    replace: "    if crate::recognition::prints_panic_locations(tcx) {\n        return;\n    }\n    for library in dependencies.libraries.values() {\n",
    tests: ["test/binaries.test.ts", "-t", "links a library whose body"],
  },  {
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
  {
    name: "tracked-declaration-untaken",
    breaks: "a trait's tracked method without a default takes the location only where the crate's own impls read it, so a library's impl that reads it, called through an app's generic code, is given none (#2350)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "    caller_locations.contains(&id) || foreign.takes_location(id) || tracked_without_default(tcx, id)\n",
    replace: "    caller_locations.contains(&id) || foreign.takes_location(id)\n",
    tests: ["test/crates-pairs-2.test.ts", "-t", "two crates: track_caller$"],
  },
  {
    name: "tracked-default-taken-as-a-declaration",
    breaks: "a trait's tracked default, which has a body that decides, takes the location in every crate whether it reads it or not (#2350)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "        && !tcx.defaultness(id).has_value()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
    snapshots: true,
  },
  {
    name: "own-panic-names-the-builders-home",
    breaks: "a crate Cargo fetched names its own panics' files under the builder's `$CARGO_HOME`, its home shipped in an app's JS, where its sites are under `/cargo/` (#2435)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "        let file = rust_js_program::library::registry_sites::cargo_named(std::path::Path::new(&file)).unwrap_or(file);\n",
    replace: "",
    tests: ["test/crate-snapshots.test.ts", "-t", "itertools"],
  },
];

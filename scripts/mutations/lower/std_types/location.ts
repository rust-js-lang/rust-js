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
    name: "std-tracked-method-untaken",
    breaks: "a trait method std's impls of which raise their panic at their caller, `Div::div`, `Ord::clamp`, doesn't take the location, so generic code's `a / b` panics `<unknown>`, and a library's generic `clamp` given its app's dictionary loses its drop (#2324)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: " || crate::recognition::std_raises_at_caller(tcx, id)\n",
    replace: "\n",
    tests: ["test/binaries.test.ts", "-t", "through a dictionary, names generic"],
  },
  {
    name: "tracked-declaration-untaken",
    breaks: "a trait's tracked method without a default takes the location only where the crate's own impls read it, so a library's impl that reads it, called through an app's generic code, is given none (#2350)",
    file: "crates/rust-js-lower/src/std_types/location.rs",
    find: "        || tracked_without_default(tcx, id)\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: track_caller$"],
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
];

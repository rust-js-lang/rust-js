// Mutations of crates/rust-js-lower/src/analysis/only_called.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "only-called-js-caller-unseen",
    breaks:
      "a `pub fn` of a private module, its module's JS export, is taken for one only the crate calls, its parameters what the crate gives, so a JS caller's `u8` sum goes unwrapped (#2025)",
    file: "crates/rust-js-lower/src/analysis/only_called.rs",
    find: "            && !crate::library::seen_outside(tcx, library, def_id)\n",
    replace: "            && !crate::library::escapes(tcx, library, def_id)\n",
    tests: ["test/js-callers-of-modules.test.ts"],
  },
  {
    name: "named-as-value-only-called",
    breaks:
      "a function named as a value, `map(double)`, is taken as only its calls give it values, so `n * 2` doesn't wrap",
    file: "crates/rust-js-lower/src/analysis/only_called.rs",
    find: "            if naming == Naming::Value {\n",
    replace: "            if false {\n",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "exported-only-called",
    breaks:
      "a function a JS caller sees is taken as only the crate's calls give it values, so `half(0)` doesn't wrap",
    file: "crates/rust-js-lower/src/analysis/only_called.rs",
    find: "            && !crate::library::seen_outside(tcx, library, def_id)\n",
    replace: "            && true\n",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "default-only-called",
    breaks:
      "a module's default, `js::export_default!(page)` of a private `fn page`, is taken for a function only the crate's calls reach, so its module's `page(3)` is all it's given, and its `n * 5` isn't wrapped where JS gives it 4000000000 (ADR 0577, amended)",
    file: "crates/rust-js-lower/src/analysis/only_called.rs",
    find: "            && !defaults.contains(&def_id)\n",
    replace: "",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's default is called with whatever JS gives it",
    ],
  },
  {
    name: "forwarded-target-only-called",
    breaks:
      "a function a namespace's method is (#566), whose one call in the crate's MIR is the method's body's, is taken as only calls reach it, so it's a throwing function, `return JSON.parse(text)`, where the method's callers, direct, as a value and JS, read its `Result` (#1675)",
    file: "crates/rust-js-lower/src/analysis/only_called.rs",
    find: "        if !calls.contains_key(&method) {\n",
    replace: "        if false {\n",
    tests: ["test/throwing-calls.test.ts", "-t", "a function a namespace's method is"],
  },
];

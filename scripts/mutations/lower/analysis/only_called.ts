// Mutations of src/lower/analysis/only_called.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "named-as-value-only-called",
    breaks:
      "a function named as a value, `map(double)`, is taken as only its calls give it values, so `n * 2` doesn't wrap",
    file: "src/lower/analysis/only_called.rs",
    find: "                Naming::Value => {\n                    values.insert(callee);\n                }\n",
    replace: "                Naming::Value => {}\n",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "exported-only-called",
    breaks:
      "a function a JS caller sees is taken as only the crate's calls give it values, so `half(0)` doesn't wrap",
    file: "src/lower/analysis/only_called.rs",
    find: "        def_id.as_local().is_some_and(|local| !visibilities.is_exported(local))",
    replace:
      "        def_id.as_local().is_some_and(|_| visibilities.is_exported(def_id.expect_local()) || true)",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "default-only-called",
    breaks:
      "a module's default, `js::export_default!(page)` of a private `fn page`, is taken for a function only the crate's calls reach, so its module's `page(3)` is all it's given, and its `n * 5` isn't wrapped where JS gives it 4000000000 (ADR 0577, amended)",
    file: "src/lower/analysis/only_called.rs",
    find: "            && !defaults.contains(&def_id)\n",
    replace: "",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's default is called with whatever JS gives it",
    ],
  },
];

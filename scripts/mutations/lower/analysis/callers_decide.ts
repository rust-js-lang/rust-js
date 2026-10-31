// Mutations of src/lower/analysis/callers_decide.rs (ADR 1014).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-zip-fold-allowed",
    breaks:
      "generic code given a zip of a map it collects steps its first side once more where std's fold stops at the shorter, running the map's closure Rust leaves unrun",
    file: "src/lower/analysis/callers_decide.rs",
    find: "                                _ => by_index.is_some(),\n",
    replace: "                                _ => false,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a zip of a map collected in generic code",
    ],
  },
  {
    name: "generic-zip-step-allowed",
    breaks:
      "generic code given a zip of a map of sides std reads by index, stepped by `next()`, steps its first side once more where std's `next()` stops at the shorter",
    file: "src/lower/analysis/callers_decide.rs",
    find: "                                Asked::ZipStep(_) => by_index == Some(true),\n",
    replace: "                                Asked::ZipStep(_) => false,\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a zip of a map stepped in generic code",
    ],
  },
  {
    name: "generic-trusted-eq-allowed",
    breaks:
      "generic code given two `TrustedLen` maps it compares by `eq` compares them item by item, running their closures where Rust compares their lengths first and runs nothing (#386)",
    file: "src/lower/analysis/callers_decide.rs",
    find: "                            if !trusted || !(runs_code(a) || runs_code(b)) {\n",
    replace: "                            if true {\n",
    tests: ["test/diagnostics.test.ts", "-t", "eq in generic code of two maps"],
  },
];

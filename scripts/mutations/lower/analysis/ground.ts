// Mutations of src/lower/analysis/ground.rs (ADR 0093): what the crate's
// analysis stands on.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "harness-bodies-kept",
    breaks: "rustc's test harness's `main` and markers are bodies the crate lowers, beside the JS runner that takes their place (ADR 0026)",
    file: "src/lower/analysis/ground.rs",
    find: "        all_bodies.iter().filter(|body| !is_harness(body.def_id)).collect()\n",
    replace: "        all_bodies.iter().collect()\n",
    tests: ["test/test-files.test.ts", "-t", "a module's one module of tests is its test file"],
  },
];

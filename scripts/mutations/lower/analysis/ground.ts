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
  {
    name: "growing-extend-capacity-unkept",
    breaks: "a crate whose `extend` asks the crate's own hint as a `Vec` fills keeps no capacity, so `with_capacity(n)` is an empty array of none (a wrong answer, #849)",
    file: "src/lower/analysis/ground.rs",
    find: "    let asks_capacity = vec_uses::asks_capacity(tcx, &all_bodies) || recognition.extends_as_it_fills(&all_bodies);\n",
    replace: "    let asks_capacity = vec_uses::asks_capacity(tcx, &all_bodies);\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_asks_hint_capacity_unasked.rs$"],
  },
];

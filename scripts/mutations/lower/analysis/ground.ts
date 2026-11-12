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
    find: "        || recognition.extends_as_it_fills(&all_bodies)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extend_asks_hint_capacity_unasked.rs$"],
  },
  {
    name: "reads-show-capacity-unasked",
    breaks: "a crate that never asks a capacity keeps none though its reader sees reads of it, so a `Vec::with_capacity` read by it is read as one of none (#1297)",
    file: "src/lower/analysis/ground.rs",
    find: "        || vec_uses::reads_show_capacity(tcx, &all_bodies);\n",
    replace: ";\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^reads_of_a_crates_reader_into_room.rs$"
    ]
  },
];

// Mutations of src/lower/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "back-stepped-none",
    breaks: "no map the crate steps from its back is seen, and one held before it's stepped is refused",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                stepped.insert(tcx.erase_and_anonymize_regions(adapter));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_stepped_from_its_back.rs$"],
  },
];

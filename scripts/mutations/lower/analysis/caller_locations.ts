// Mutations of src/lower/analysis/caller_locations.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "caller-read-unseen",
    breaks: "a tracked function that reads `Location::caller()` takes no location, and reads none",
    file: "src/lower/analysis/caller_locations.rs",
    find: "                taking.insert(owner);\n",
    replace: "                let _ = owner;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "caller-passed-on-unseen",
    breaks: "a tracked function that calls one taking its location takes none, and gives its own place instead",
    file: "src/lower/analysis/caller_locations.rs",
    find: "            if (taking.contains(&callee) || foreign.takes_location(callee)) && taking.insert(caller) {\n",
    replace: "            if false && taking.insert(caller) {\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
];

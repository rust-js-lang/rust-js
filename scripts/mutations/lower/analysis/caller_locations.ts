// Mutations of src/lower/analysis/caller_locations.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "caller-read-unseen",
    breaks:
      "a tracked function that reads `Location::caller()` takes no location, and reads none",
    file: "src/lower/analysis/caller_locations.rs",
    find: "                taking.insert(owner);\n",
    replace: "                let _ = owner;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "caller-passed-on-unseen",
    breaks:
      "a tracked function that calls one taking its location takes none, and gives its own place instead",
    file: "src/lower/analysis/caller_locations.rs",
    find: "            if (taking.contains(&callee) || foreign.takes_location(callee)) && taking.insert(caller) {\n",
    replace: "            if false && taking.insert(caller) {\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "dictionary-location-untaken",
    breaks:
      "a trait method one of whose impls reads its caller's location is called through a dictionary without it",
    file: "src/lower/analysis/caller_locations.rs",
    find: "            if takes {\n                changed |= taking.insert(declared);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
  {
    name: "dictionary-call-not-passed-on",
    breaks:
      "a tracked function calling a trait method through a dictionary passes on no location",
    file: "src/lower/analysis/caller_locations.rs",
    find: "                _ => calls.push((owner, callee)),\n",
    replace: "                _ => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
];

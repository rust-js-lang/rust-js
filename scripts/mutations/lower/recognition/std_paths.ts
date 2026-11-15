// Mutations of src/lower/recognition/std_paths.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const file = "src/lower/recognition/std_paths.rs";

export const mutations: Mutation[] = [
  {
    name: "std-paths-std-alone",
    breaks: "a row is found through `std`'s modules alone, so a `#![no_std]` crate's `panic::Location`, core's, isn't std's",
    file,
    find: "        let found = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]\n",
    replace: "        let found = [StdCrate::Std]\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "std-paths-none",
    breaks: "no item is std's of the table, so `env::VarError` is a crate's enum, not std's `\"NotPresent\"`",
    file,
    find: "    std_paths(tcx).by_id.get(&id).copied()\n",
    replace: "    std_paths(tcx).by_id.get(&id).copied().filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
];

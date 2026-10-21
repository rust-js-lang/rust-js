// Mutations of src/lower/analysis/held_adapters.rs (ADR 0542).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-adapters-none",
    breaks: "no kind of adapter the crate's types hold is seen, and a held `Enumerate`'s size hint, back and `Debug` are refused",
    file: "src/lower/analysis/held_adapters.rs",
    find: "                    held.insert(adapter.did());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
];

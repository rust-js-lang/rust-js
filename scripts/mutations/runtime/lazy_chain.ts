// Mutations of src/runtime/lazy_chain.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-second-unclosed",
    breaks: "`chain`'s second side isn't closed with it, so none of it is dropped",
    file: "src/runtime/lazy_chain.js",
    find: "    b.return?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
];

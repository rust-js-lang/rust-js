// Mutations of src/runtime/lazy_chain.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-second-unclosed",
    breaks: "`chain`'s second side isn't closed with it, so none of it is dropped",
    file: "src/runtime/lazy_chain.js",
    find: "      second.return?.();\n    },\n",
    replace: "    },\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "chain-fold-steps",
    breaks: "`chain`'s fold steps each side by `next`, not by its own `fold`, `a`'s then `b`'s",
    file: "src/runtime/lazy_chain.js",
    find: "  chained.fold = (init, f) => {\n",
    replace: "  chained.unused = (init, f) => {\n",
    tests: ["test/corpus.test.ts",  "-t",  "^own_fold_and_nth.rs$"],
  },
];

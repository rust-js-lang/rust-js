// Mutations of src/runtime/made.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "made-in-turn-drops-none",
    breaks: "a derived `Clone`'s or `Default`'s parts made before one that panics are left undropped (#897, #902)",
    file: "src/runtime/made.js",
    find: "    () => drops.forEach((drop) => drop()),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(clone|default)_unwinds_what_it_made.rs$"],
  },
  {
    name: "made-in-turn-first-first",
    breaks: "a derived `Clone`'s parts made before one that panics are dropped first first, not the latest first as Rust drops an aggregate's operands (#897)",
    file: "src/runtime/made.js",
    find: "    drops.unshift(() => drop?.(part));\n",
    replace: "    drops.push(() => drop?.(part));\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
  {
    name: "made-each-drops-none",
    breaks: "a clone of a `Vec` whose item's clone panics leaves the items it made undropped (#897)",
    file: "src/runtime/made.js",
    find: "    () => $dropItems(made, drop),\n",
    replace: "    () => undefined,\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
  {
    name: "made-each-latest-first",
    breaks: "a clone of a `Vec` whose item's clone panics drops the items it made the latest first, not in order as std's guard does (#897)",
    file: "src/runtime/made.js",
    find: "    () => $dropItems(made, drop),\n",
    replace: "    () => $dropItems(made.reverse(), drop),\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_unwinds_what_it_made.rs$"],
  },
];

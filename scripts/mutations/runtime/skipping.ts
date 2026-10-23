// Mutations of src/runtime/skipping.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "skip-dropping-keeps-skipped",
    breaks: "`skip` of items with a destructor drops none it skips",
    file: "src/runtime/skipping.js",
    find: "        drop?.(step.value);\n      }\n      return skipping.iter.next();\n",
    replace: "      }\n      return skipping.iter.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "skip-dropping-unclosed",
    breaks: "closing a `skip` of items with a destructor doesn't close what it steps",
    file: "src/runtime/skipping.js",
    find: "    () => $closed(skipping.iter),\n",
    replace: "    () => {},\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "skipping-length-its-source",
    breaks: "an explicit `skip`'s length is its source's, counting what it skips",
    file: "src/runtime/skipping.js",
    find: "  skipping.len = () => Math.max(skipping.iter.len() - skipping.n, 0);\n",
    replace: "  skipping.len = () => skipping.iter.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_counted_adapters.rs$"],
  },
];

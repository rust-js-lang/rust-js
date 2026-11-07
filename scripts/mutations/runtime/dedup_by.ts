// Mutations of src/runtime/dedup_by.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dedup-by-drops-none",
    breaks: "`dedup_by` of items with a destructor drops none it takes out",
    file: "src/runtime/dedup_by.js",
    find: "      if (duplicate) drop?.(item);\n",
    replace: "      if (duplicate);\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "dedup-cells-unwritten",
    breaks: "what the closure changes of the kept item isn't kept",
    file: "src/runtime/dedup_by.js",
    find: "      v[n - 1] = b.value;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "dedup-cells-unwritten-as-it-unwinds",
    breaks: "what a `dedup_by` closure wrote through its `&mut` before it panicked isn't kept, so what it replaced, dropped already, is dropped again (#906)",
    file: "src/runtime/dedup_by.js",
    find: "    try {\n      return same(a, b);\n    } finally {\n      v[i] = a.value;\n      v[n - 1] = b.value;\n    }\n",
    replace: "    const same$ = same(a, b);\n    v[i] = a.value;\n    v[n - 1] = b.value;\n    return same$;\n",
    tests: ["test/corpus.test.ts", "-t", "^dedup_by_drops_as_it_finds.rs$"],
  },
];

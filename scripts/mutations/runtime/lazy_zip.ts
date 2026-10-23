// Mutations of src/runtime/lazy_zip.js (ADR 0465).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "zip-pulled-undropped",
    breaks: "an item `zip` steps where the other side's run out isn't dropped",
    file: "src/runtime/lazy_zip.js",
    find: "      dropA?.(item.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "zip-other-unclosed",
    breaks: "`zip`'s other side isn't closed with it, so what it hasn't given isn't dropped",
    file: "src/runtime/lazy_zip.js",
    find: "    right.return?.();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "zip-back-untrimmed",
    breaks: "`rev()` of a zip pairs the longer one's last items, not those the shorter has",
    file: "src/runtime/lazy_zip.js",
    find: "  for (let n = a.len() - b.len(); n > 0; n--) a.back();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_from_back.rs$"],
  },
  {
    name: "zip-closed-at-end",
    breaks: "a `zip` closes both sides at its first end, so a side that gives more after a `None` isn't stepped on",
    file: "src/runtime/lazy_zip.js",
    find: "    if (item.done) return item;\n",
    replace: "    if (item.done) {\n      close();\n      return item;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^restarting_iterators.rs$"],
  },
];

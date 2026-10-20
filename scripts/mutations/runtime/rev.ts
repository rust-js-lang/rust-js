// Mutations of src/runtime/rev.js (ADR 0516).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rev-unclosed",
    breaks: "a `Rev` dropped drops none of what it holds",
    file: "src/runtime/rev.js",
    find: "  rev.return = () => it.return?.() ?? { done: true, value: undefined };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "rev-front-first",
    breaks: "`$rev` steps from the front",
    file: "src/runtime/rev.js",
    find: "  rev.next = () => it.back();\n",
    replace: "  rev.next = () => it.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^(rev_drops|generic_double_ended).rs$"],
  },
  {
    name: "rev-sourceless",
    breaks: "a `$rev` keeps nothing of what it steps for a clone to clone",
    file: "src/runtime/rev.js",
    find: "  rev.source = it;\n",
    replace: "",
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
];

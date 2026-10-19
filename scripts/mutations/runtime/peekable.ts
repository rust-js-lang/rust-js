// Mutations of src/runtime/peekable.js (ADR 0164).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "peekable-peeked-lost",
    breaks: "a lazy peekable's `next()` steps its source past what `peek` took, losing it",
    file: "src/runtime/peekable.js",
    find: "    const step = this.peeked ?? this.it.next();\n",
    replace: "    const step = this.it.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
  {
    name: "peekable-peeked-kept",
    breaks: "a lazy peekable gives what `peek` took again after `next()` gave it",
    file: "src/runtime/peekable.js",
    find: "    const step = this.peeked ?? this.it.next();\n    this.peeked = undefined;\n",
    replace: "    const step = this.peeked ?? this.it.next();\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
];

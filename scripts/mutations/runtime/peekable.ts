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
  {
    name: "peekable-peeked-dropped-first",
    breaks: "a `Peekable` dropped drops what it peeked at before its source's rest",
    file: "src/runtime/peekable.js",
    find: "  return() {\n    this.it.return?.();\n    const peeked = this.peeked;\n    this.peeked = undefined;\n    if (peeked !== undefined && !peeked.done) this.drop?.(peeked.value);\n",
    replace: "  return() {\n    const peeked = this.peeked;\n    this.peeked = undefined;\n    if (peeked !== undefined && !peeked.done) this.drop?.(peeked.value);\n    this.it.return?.();\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_drops.rs$"],
  },
  {
    name: "peekable-peeked-undropped",
    breaks: "a `Peekable` dropped drops none of what it peeked at",
    file: "src/runtime/peekable.js",
    find: "    if (peeked !== undefined && !peeked.done) this.drop?.(peeked.value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^peekable_drops.rs$"],
  },
  {
    name: "peekable-len-unpeeked",
    breaks: "a `Peekable`'s `len()` counts none of what it peeked at",
    file: "src/runtime/peekable.js",
    find: "    return (this.peeked === undefined || this.peeked.done ? 0 : 1) + this.it.len();\n",
    replace: "    return this.it.len();\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_drops.rs$"],
  },
  {
    name: "peekable-back-unpeeked",
    breaks: "a `Peekable`'s `next_back()` never gives what it peeked at",
    file: "src/runtime/peekable.js",
    find: "    if (!step.done) return step;\n    const peeked = this.peeked;\n    this.peeked = undefined;\n    return peeked;\n",
    replace: "    return step;\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_drops.rs$"],
  },
];

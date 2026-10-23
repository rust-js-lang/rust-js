// Mutations of src/runtime/closing.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "closing-drops-none",
    breaks: "a `take` of an iterator that owns items with a destructor drops none it hasn't given as it ends",
    file: "src/runtime/closing.js",
    find: "      drop?.(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
  },
  {
    name: "closing-twice",
    breaks: "an iterator closed twice, by `chain`'s `yield*` and its end, is dropped twice",
    file: "src/runtime/closing.js",
    find: "    if (!closed) {\n",
    replace: "    if (true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "closing-not-dropped-when-done",
    breaks: "an adapter's source of the crate's, stepped to its end by a consumer, is never dropped, as no JS consumer calls `return()` at its end",
    file: "src/runtime/closing.js",
    find: "    if (step.done) close();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_source_drops.rs$"],
  },
  {
    name: "closing-back-hidden",
    breaks: "what `$closing` closes is stepped from its back by nothing, so `rev()` of an adapter of a source with a destructor fails",
    file: "src/runtime/closing.js",
    find: "  closing.back = () => it.back();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rev_drops.rs$"],
  },
  {
    name: "closing-own-fold-dropped-again",
    breaks: "what the crate's own fold took and dropped is dropped again as it's closed",
    file: "src/runtime/closing.js",
    find: "    if (it.ownFold) closed = true;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "closing-fold-steps",
    breaks: "a closed iterator's fold steps it by `next`, not its source's own `fold`",
    file: "src/runtime/closing.js",
    find: "  closing.fold = (init, f) => {\n",
    replace: "  closing.unused = (init, f) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
];

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
    name: "closing-len-unforwarded",
    breaks: "an iterator closed to drop what it holds has no `len()` of its own",
    file: "src/runtime/closing.js",
    find: "  closing.len = () => it.len();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^peekable_drops.rs$"],
  },
  {
    name: "closing-hint-hidden",
    breaks: "a `$closing` hides what it closes' size hint",
    file: "src/runtime/closing.js",
    find: "  closing.hint = () => $sizeHint(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fuse_size_hint.rs$"],
  },
  {
    name: "closing-hides-its-iter",
    breaks: "what `$closing` closes isn't reachable, so a use site can't give it its back",
    file: "src/runtime/closing.js",
    find: "  closing.iter = it;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_fuse_back_drops.rs$"],
  },
];

// Mutations of src/runtime/repeating.js (ADR 0422).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeating-n-unhinted",
    breaks: "`repeat_n(x, n)` hints `(0, None)`, so std's struct of a `map` of it, collected, asks no room and runs its closure until memory runs out, where std panics `capacity overflow` first (#246)",
    file: "src/runtime/repeating.js",
    find: "  it.hint = () => [left, left];\n",
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "collected_counts_ask_room"],
  },
  {
    name: "repeating-closed-undropped",
    breaks: "a `repeat`'s value isn't dropped when its iterator is closed",
    file: "src/runtime/repeating.js",
    find: "      if (!closed) {\n        closed = true;\n        drop?.(value);\n",
    replace: "      if (!closed) {\n        closed = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "repeating-closed-twice",
    breaks: "a `repeat`'s value is dropped each time its iterator is closed, so a `take(2)` of it, closed at its limit and again at its collect's end, drops it twice (#708)",
    file: "src/runtime/repeating.js",
    find: "      if (!closed) {\n        closed = true;\n        drop?.(value);\n      }\n",
    replace: "      drop?.(value);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "repeating-js-iterator",
    breaks: "a `repeat` is JS's iterator, whose `take` is done with it once a clone throws, a panic, so the value it holds is never dropped as the panic unwinds (#708)",
    file: "src/runtime/repeating.js",
    find: "  const it = new $Steps(\n    () => ({ done: false, value: clone(value) }),\n",
    replace: "  const it = ((next, close) => Iterator.from({ next, return: () => (close(), { done: true }) }))(\n    () => ({ done: false, value: clone(value) }),\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "repeat-n-none-kept",
    breaks: "`repeat_n` of a count of none drops none of its value",
    file: "src/runtime/repeating.js",
    find: "  if (left === 0) drop?.(value);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
  {
    name: "repeat-n-last-cloned",
    breaks: "`repeat_n`'s last item is a clone, its value never given or dropped",
    file: "src/runtime/repeating.js",
    find: "      return { done: false, value: left === 0 ? value : clone(value) };\n",
    replace: "      return { done: false, value: clone(value) };\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
  {
    name: "repeat-n-closed-kept",
    breaks: "`repeat_n` dropped before its last drops none of its value",
    file: "src/runtime/repeating.js",
    find: "        left = 0;\n        drop?.(value);\n",
    replace: "        left = 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_and_peek_mut.rs$"],
  },
];

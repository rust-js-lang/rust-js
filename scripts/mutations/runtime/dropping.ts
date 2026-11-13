// Mutations of src/runtime/dropping.js (ADR 0484).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dropping-carries-nothing",
    breaks: "`$dropping` gives back the closure carrying no drop",
    file: "src/runtime/dropping.js",
    find: "  own[$disposeKey] = drop;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "dispose-plain-function",
    breaks: "a `dyn Fn` of a function that carries no drop throws as it's dropped",
    file: "src/runtime/dropping.js",
    find: "  const drop = f[$disposeKey];\n  if (drop === undefined) return;\n",
    replace: "  const drop = f[$disposeKey];\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "dropping-once-not-on-unwind",
    breaks: "a `Fn` closure made a `dyn FnOnce` whose call panics never drops what it took as it unwinds (#409)",
    file: "src/runtime/dropping.js",
    find: "  return $dropping((...args) => $endsAfter(drop, () => f(...args)), drop);\n",
    replace: "  return $dropping((...args) => {\n    const result = f(...args);\n    drop();\n    return result;\n  }, drop);\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_boxed_once_drops.rs$"],
  },
  {
    name: "borrowed-is-the-function",
    breaks: "`$borrowed(f)` gives back `f` itself, and a `dyn Fn` of a borrowed function runs the drop `f` carries as it's dropped",
    file: "src/runtime/dropping.js",
    find: "  return (...args) => f(...args);\n",
    replace: "  return f;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_of_a_borrowed_function.rs$"],
  },
  {
    name: "dropping-again-in-place",
    breaks:
      "a function that carries a drop already, a `Box<dyn Fn>` generic code boxes again, is given the new drop in place of its own, which calls `$dispose` of itself: its drop overflows the stack (#1221)",
    file: "src/runtime/dropping.js",
    find: "  const own = f[$disposeKey] === undefined ? f : (...args) => f(...args);\n",
    replace: "  const own = f;\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_function_boxed_again.rs$"],
  },
  {
    name: "iterator-dyn-drop-inherited",
    breaks: "`$disposeIterator` reads `Symbol.dispose` through the prototype, so a generator given as a `dyn Iterator` that carries no drop is closed by `Iterator.prototype`'s (#1420)",
    file: "src/runtime/dropping.js",
    find: "  if (!Object.hasOwn(it, $disposeKey)) return;\n",
    replace: "  if (it[$disposeKey] === undefined) return;\n",
    tests: ["test/runtime-package.test.ts", "-t", "a dyn Iterator's drop is its own"],
  },
  {
    name: "iterator-dyn-reboxed-in-place",
    breaks: "a `dyn Iterator` that carries a drop boxed again carries the new one in its place, whose drop disposes itself, again and again (#1420)",
    file: "src/runtime/dropping.js",
    find: "  const lent = Object.hasOwn(it, $disposeKey) ? $lent(it) : it;\n",
    replace: "  const lent = it;\n",
    tests: ["test/runtime-package.test.ts", "-t", "a dyn Iterator's drop is its own"],
  },
  {
    name: "iterator-dyn-unclosed-by-its-consumer",
    breaks: "a `Box<dyn Iterator>` a consumer takes by value, `count()` or `collect()`, never drops what it owns, as the consumer closes it and nothing runs the drop it carries (#1420)",
    file: "src/runtime/dropping.js",
    find: "    once();\n    return step;\n",
    replace: "    return step;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_iterator_drops.rs$"],
  },
  {
    name: "dropping-iterator-js-helpers",
    breaks: "a `dyn Iterator` carrying a drop keeps JS's own iterator helpers, which close it as their function throws, dropping what the box holds before the fold's total (#1690)",
    file: "src/runtime/dropping.js",
    find: "  const own = lent instanceof $Steps ? lent : $stepsAround(lent);\n",
    replace: "  const own = lent;\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_summed_drops_its_rest_after_the_total.rs$"],
  },
];

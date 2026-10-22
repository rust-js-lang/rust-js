// Mutations of src/runtime/peek.js (ADR 0071).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "peek-takes-the-item",
    breaks: "`peek()` takes the item, which `next()` then never gives",
    file: "src/runtime/peek.js",
    find: "  if (peekable.peeked === undefined) peekable.peeked = $some($next(peekable));\n  return $someValue(peekable.peeked);\n",
    replace: "  return $next(peekable);\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
  {
    name: "peek-some-takes-the-item",
    breaks: "`peek()` of `Option`s takes the item, which `next()` then never gives",
    file: "src/runtime/peek.js",
    find: "  if (peekable.peeked === undefined) peekable.peeked = $some($nextSome(peekable));\n  return $someValue(peekable.peeked);\n",
    replace: "  return $nextSome(peekable);\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_lazy.rs$"],
  },
  {
    name: "peekable-hint-without-peeked",
    breaks: "a `Peekable`'s size hint and `len()` leave out what it peeked at",
    file: "src/runtime/peek.js",
    find: "  const more = peekable.peeked === undefined ? 0 : 1;\n",
    replace: "  const more = 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^(peekable_len|held_peekable).rs$"],
  },
  {
    name: "peekable-hint-dictionary-dropped",
    breaks: "`$peekableHint` doesn't pass on the dictionary it's given",
    file: "src/runtime/peek.js",
    find: "  const [low, high] = $sizeHint(peekable.iter, iterator);\n",
    replace: "  const [low, high] = $sizeHint(peekable.iter);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_size_hints.rs$"],
  },
];

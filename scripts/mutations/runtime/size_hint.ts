// Mutations of src/runtime/size_hint.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-size-hint-stepped-whole",
    breaks: "`size_hint()` of a generic iterator partly stepped counts the items it's past",
    file: "src/runtime/size_hint.js",
    find: "Array.isArray(it.items) ? it.end - it.at : undefined;",
    replace: "Array.isArray(it.items) ? it.items.length : undefined;",
    tests: ["test/corpus.test.ts", "-t", "size_hint"],
  },
  {
    name: "size-hint-past-end",
    breaks: "a generic `size_hint()` counts what `next_back()` took",
    file: "src/runtime/size_hint.js",
    find: "it.end - it.at",
    replace: "it.items.length - it.at",
    tests: ["test/corpus.test.ts","-t","iter_next_back"],
  },
  {
    name: "size-hint-not-adapters-own",
    breaks: "an explicit adapter's size hint isn't its own",
    file: "src/runtime/size_hint.js",
    find: "  if (it.hint !== undefined) return it.hint();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
  {
    name: "size-hint-not-what-it-steps",
    breaks: "a `$iterator`'s size hint isn't that of the JS iterator it steps",
    file: "src/runtime/size_hint.js",
    find: "  if (typeof it.of?.next === \"function\") return $sizeHint(it.of);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^held_enumerate.rs$"],
  },
  {
    name: "size-hint-dictionary-unasked",
    breaks: "`$sizeHint` of a value given as it is doesn't ask its dictionary's own `size_hint`",
    file: "src/runtime/size_hint.js",
    find: "  if (iterator?.sizeHint !== undefined) return iterator.sizeHint(it);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapter_size_hints.rs$"],
  },
  {
    name: "size-hint-of-range-unknown",
    breaks: "a range given as an `Iterator`, `{ next: $rangeNext }`, hints `(0, None)`, so std's `Peekable` of one does, and its `len()` panics",
    file: "src/runtime/size_hint.js",
    find: "  if (iterator?.next === $rangeNext && typeof it.start === \"number\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^peekable_range_len.rs$"],
  },
  {
    name: "inclusive-range-hint-unknown",
    breaks: "an `a..=b` given as it is to std's `Skip` says nothing of how many it has left, so its `len` asserts and panics (ADR 0918)",
    file: "src/runtime/size_hint.js",
    find: "  if (iterator?.next === $rangeInclusiveNext && typeof it.start === \"number\") {\n",
    replace: "  if (false) {\n",
    tests: ["test/iterator-forms.test.ts", "-t", "^range_hints_given with std's iterator structs$"],
  },
];

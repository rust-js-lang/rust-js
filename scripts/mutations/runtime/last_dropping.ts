// Mutations of src/runtime/last_dropping.js (ADR 0098).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "last-steps-not-folds",
    breaks: "`last` steps an adapter with a `fold` of its own by `next`, not by its `fold` as std's default does, so a `take_while` of a reversed `Vec`'s `skip` drops in the wrong order (#299)",
    file: "src/runtime/last_dropping.js",
    find: "  if (Object.hasOwn(it, \"fold\")) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^step_by_compiled_source_nth.rs$"],
  },
  {
    name: "last-dropping-drops-none",
    breaks: "`last()` of items with a destructor drops none it passes",
    file: "src/runtime/last_dropping.js",
    find: "  const keep = (item) => {\n    if (seen) drop?.(last);\n",
    replace: "  const keep = (item) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
  },
  {
    name: "last-dropping-own-unused",
    breaks: "generic code's `last` of items with a destructor steps by `next`, not the crate's own `last`",
    file: "src/runtime/last_dropping.js",
    find: "  if (it.ownLast) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^own_nth_last_dropping.rs$"],
  },
  {
    name: "chain-last-each-stepped",
    breaks: "`a.chain(b).last()` steps every item in turn, dropping each as the next comes, not std's each side's own `last` (#232)",
    file: "src/runtime/last_dropping.js",
    find: "  if (it.sides !== undefined) {",
    replace: "  if (false) {",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "chain-last-first-kept",
    breaks: "`a.chain(b).last()` never drops `a`'s last where `b` has one, as `b_last.or(a_last)` does (#232)",
    file: "src/runtime/last_dropping.js",
    find: "    if (aLast !== undefined) drop?.($someValue(aLast));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "skip-last-each-stepped",
    breaks: "`chain(..).skip(n).last()` steps every item, not its source's own `last` after what it skips (#232)",
    file: "src/runtime/last_dropping.js",
    find: "  if (it.skipped !== undefined) {",
    replace: "  if (false) {",
    tests: ["test/corpus.test.ts", "-t", "^chain_sides_own_steps.rs$"],
  },
  {
    name: "last-throw-keeps-last",
    breaks: "a `last` whose stage panics leaves the one it kept undropped (#423)",
    file: "src/runtime/last_dropping.js",
    find: "      if (seen) drop?.(last);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^panic_in_own_next_drops.rs$"],
  },
  {
    name: "last-fold-unwind-lost",
    breaks: "a `last` gives its fold no `unwind`, so a stage's panic drops what's left before the one it kept (#423)",
    file: "src/runtime/last_dropping.js",
    find: "      it.fold(undefined, (_, item) => keep(item), drop && unwind);",
    replace: "      it.fold(undefined, (_, item) => keep(item));",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
];

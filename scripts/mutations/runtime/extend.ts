// Mutations of src/runtime/extend.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extend-unclosed-on-throw",
    breaks: "an `extend` whose iterator's closure panics leaves what's left of it undropped (#422)",
    file: "src/runtime/extend.js",
    find: "    () => it.return?.(),\n  );\n}",
    replace: "    () => undefined,\n  );\n}",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_iteration_drops.rs$"],
  },
  {
    name: "extend-by-next",
    breaks: "an `extend` of a `TrustedLen` iterator steps it by `next`, so a `map`'s function drops after what it left, where `extend_trusted` folds it (#710)",
    file: "src/runtime/extend.js",
    find: "  if ((byFold || items.byItsFold) && typeof items.fold === \"function\") {\n",
    replace: "  if (items.byItsFold && typeof items.fold === \"function\") {\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
  {
    name: "extend-of-folded-by-next",
    breaks: "an `extend` of what `$folded` made, a type parameter's or the crate's own, steps it by `next`, which lists it whole first, so what a step panics past is never put in, a `LinkedList`'s in generic code (#1161)",
    file: "src/runtime/extend.js",
    find: "  if ((byFold || items.byItsFold) && typeof items.fold === \"function\") {\n",
    replace: "  if (byFold && typeof items.fold === \"function\") {\n",
    tests: ["test/corpus.test.ts", "-t", "^consumers_keep_what_they_built_on_a_panic.rs$"],
  },
];

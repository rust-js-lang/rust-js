// Mutations of src/runtime/splice.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "splice-replaces-nothing",
    breaks: "`splice` removes the range but puts nothing in its place",
    file: "src/runtime/splice.js",
    find: "      items.push(step.value);\n",
    replace: "      void step.value;\n",
    tests: ["test/corpus.test.ts", "-t", "vec_edits"],
  },
  {
    name: "splice-replacement-spread",
    breaks: "what a splice collects of its replacement past its hint is spread into `push`'s arguments, which Babel's loose spread takes as one item, and more of which than a JS call takes is a `RangeError` (ADR 0919)",
    file: "src/runtime/splice.js",
    find: "    $extend(items, collected);\n",
    replace: "    items.push(...collected);\n",
    tests: ["test/corpus.test.ts", "-t", "^runtime_long_writes.rs$"],
  },
  {
    name: "splice-replacement-unclosed",
    breaks: "a splice whose replacement's step panics leaves what's left of the replacement unclosed, so it's never dropped, where `Splice`'s fields drop as it unwinds (#1141, #1149)",
    file: "src/runtime/splice.js",
    find: "      restored,\n      closed,\n    );\n",
    replace: "      restored,\n    );\n",
    tests: ["test/corpus.test.ts", "-t", "^(consumer_closes_what_it_steps_on_a_panic|splice_drops_and_fills_as_std).rs$"],
  },
  {
    name: "splice-removed-undropped",
    breaks: "a `Splice` dropped before it's given all it took out never drops the rest, which std's drop drops before it fills the gap (#1149)",
    file: "src/runtime/splice.js",
    find: "        if (dropItem !== undefined) $dropItems(rest, dropItem);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "splice-gap-collected-apart",
    breaks: "a splice's replacement is collected apart from the start, not put in the gap as it's stepped, so what it put before a step panics is dropped where std's stays in the `Vec` (#1149)",
    file: "src/runtime/splice.js",
    find: "    if (!fill(gap)) return;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "splice-tail-lost-on-a-panic",
    breaks: "a splice whose replacement's step panics never puts the `Vec`'s tail back, which std's `Drain::drop` does as it unwinds (#1149)",
    file: "src/runtime/splice.js",
    find: "      restored,\n      closed,\n    );\n",
    replace: "      closed,\n    );\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_drops_and_fills_as_std.rs$"],
  },
  {
    name: "splice-now-tail-first",
    breaks: "a `splice` put in place where it's made puts its tail back before its replacement (#1189)",
    file: "src/runtime/splice.js",
    find: "  $extend(items, replacement);\n  $extend(items, tail);\n  return $iter(removed);\n",
    replace: "  $extend(items, tail);\n  $extend(items, replacement);\n  return $iter(removed);\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_of_array_items.rs$"],
  },
  {
    name: "splice-now-takes-nothing",
    breaks: "a `splice` put in place where it's made gives nothing of what it took out (#1189)",
    file: "src/runtime/splice.js",
    find: "  return $iter(removed);\n",
    replace: "  return $iter([]);\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_of_array_items.rs$"],
  },
];

// Mutations of src/runtime/flat_map.js (ADR 0510): a source and inner
// iterators that both hold what has a destructor, closed source first.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flat-map-inner-closed-first",
    breaks: "`$flatMap` closes the inner iterator before its source, as JS's `flatMap` does, not as Rust drops a `FlatMap`",
    file: "src/runtime/flat_map.js",
    find: "      try {\n        if (!ended) {\n          ended = true;\n          it.return?.();\n",
    replace: "      inner?.return?.();\n      try {\n        if (!ended) {\n          ended = true;\n          it.return?.();\n",
    tests: ["test/corpus.test.ts", "-t", "^flat_map_drops.rs$"],
  },
  {
    name: "flat-map-no-inners",
    breaks: "`$flatMap` gives `$lastDropping` no parts, so a flatten's `last` drops each item as the next comes, not each inner iterator's own `last` (#645)",
    file: "src/runtime/flat_map.js",
    find: "  flat.inners = () => {\n",
    replace: "  flat.unasked = () => {\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_last_takes_each_inner_last.rs$"],
  },
  {
    name: "flat-map-source-unfused",
    breaks: "`$flatMap` steps its source again once it's given `None`, where std's `Flatten` holds it fused",
    file: "src/runtime/flat_map.js",
    find: "        if (ended) return { done: true, value: undefined };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flatten_last_takes_each_inner_last.rs$"],
  },
  {
    name: "flat-map-last-source-unfused",
    breaks: "a flatten's `last` folds its source again once it's given `None`, where std's `Flatten` holds it fused",
    file: "src/runtime/flat_map.js",
    find: "    const source = ended ? undefined : it;\n",
    replace: "    const source = it;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_last_takes_each_inner_last.rs$"],
  },
  {
    name: "flat-map-fold-by-next",
    breaks: "`$flatMap`'s fold steps its source by `next`, not by its own `fold`, so a flatten's `count` never calls the crate's own `fold`, as std's `FlattenCompat::fold` does",
    file: "src/runtime/flat_map.js",
    find: "  $forward(flat, it, { fold, tryFold });\n",
    replace: "  $forward(flat, it, { tryFold });\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "flat-map-source-kept-at-its-end",
    breaks: "`$flatMap`'s source, ended, isn't dropped until the flatten is, where std's `Fuse` drops it at its first `None`: a peeked `Peekable`'s source is dropped after what's collected",
    file: "src/runtime/flat_map.js",
    find: "          ended = true;\n          it.return?.();\n          return outer;\n",
    replace: "          ended = true;\n          return outer;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "flat-map-inner-kept-as-source-drop-throws",
    breaks: "`$flatMap`'s inner iterator isn't dropped where its source's drop panics, where Rust's drop glue drops each field as it unwinds",
    file: "src/runtime/flat_map.js",
    find: "      } finally {\n        const front = inner;\n        inner = undefined;\n        front?.return?.();\n      }\n",
    replace: "      } finally {\n        inner = undefined;\n      }\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_closed_part_way_drops_source_first.rs$"],
  },
  {
    name: "flat-map-own-unpassed",
    breaks: "a `flat_map`'s own function isn't given to its source's fold, so it drops after the source's rest (#710)",
    file: "src/runtime/flat_map.js",
    find: "      const passed = ends === false ? false : own;\n",
    replace: "      const passed = undefined;\n",
    tests: ["test/corpus.test.ts", "-t", "^fold_drops_moved_function_first.rs$"],
  },
];

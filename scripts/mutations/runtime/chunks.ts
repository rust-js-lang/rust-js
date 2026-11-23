// Mutations of crates/rust-js-program/src/runtime/chunks.js (ADR 0335).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chunks-past-end",
    breaks: "the last chunk runs past the end, which a view refuses",
    file: "crates/rust-js-program/src/runtime/chunks.js",
    find: "cut(v, i * size, Math.min(i * size + size, v.length))",
    replace: "cut(v, i * size, i * size + size)",
    tests: ["test/corpus.test.ts", "-t", "^slice_edit_drops\\.rs$"],
  },
  {
    name: "chunks-source-unkept",
    breaks: "a slice's chunks keep no record of what they were cut of, so `{:?}` of what's left of one can't say its `v` or its `chunk_size` (#2252)",
    file: "crates/rust-js-program/src/runtime/chunks.js",
    find: "  parts[$partsOf] = { source: v, size, windows: false };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_chunks_and_windows.rs$"],
  },
  {
    name: "chunks-left-of-windows",
    breaks: "what's left of a slice's chunks is read as windows', from the first left to the last's end and its window less one, `v: [3, 4, 5]` where std's has `[3, 4]` (#2252)",
    file: "crates/rust-js-program/src/runtime/chunks.js",
    find: "  if (windows) return $part(source, at, Math.min(end + size - 1, source.length));\n",
    replace: "  return $part(source, at, Math.min(end + size - 1, source.length));\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_chunks_and_windows.rs$"],
  },
  {
    name: "parts-stepped-unrebased",
    breaks: "what's left of a slice's chunks, a clone's, keeps its whole source, so its `v` shows the chunks it stepped past too (#2252)",
    file: "crates/rust-js-program/src/runtime/chunks.js",
    find: "  parts[$partsOf] = { source: $part(source, from, source.length), size, windows };\n",
    replace: "  parts[$partsOf] = { source, size, windows };\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_std_adapters.rs$"],
  },
  {
    name: "item-left-never-boxed",
    breaks: "the item left of an iterator of one item at most is never boxed, so `once(())`'s `Some(())` reads as `None` (ADR 0051, #2252)",
    file: "crates/rust-js-program/src/runtime/chunks.js",
    find: "  return boxed ? $some(it.items[it.at]) : it.items[it.at];\n",
    replace: "  return it.items[it.at];\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_once_and_successors.rs$"],
  },
];

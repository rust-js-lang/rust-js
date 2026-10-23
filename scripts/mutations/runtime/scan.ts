// Mutations of src/runtime/scan.js (ADR 0517).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scan-past-none",
    breaks: "`scan` goes on past its first `None`",
    file: "src/runtime/scan.js",
    find: "      if (next !== undefined) return { done: false, value: next };\n      source.return?.();\n",
    replace: "      if (next !== undefined) return { done: false, value: next };\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_lazy_drops.rs$"],
  },
  {
    name: "scan-handle-always",
    breaks: "`$lazyScan` boxes its state even where it's told the state's `&mut` is the object, so a struct's state is read as its box",
    file: "src/runtime/scan.js",
    find: "  const state = handle ? { value: init } : init;",
    replace: "  const state = { value: init };",
    tests: ["test/corpus.test.ts", "-t", "^scan_object_state.rs$"],
  },
  {
    name: "scan-fused",
    breaks: "`scan` stays ended after a `None`, where Rust's `Scan` steps its source on",
    file: "src/runtime/scan.js",
    find: "    if (next !== undefined) return { done: false, value: next };\n",
    replace: "    if (next !== undefined) return { done: false, value: next };\n    it.next = () => ({ done: true, value: undefined });\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_map_while_unfused.rs$"],
  },
];

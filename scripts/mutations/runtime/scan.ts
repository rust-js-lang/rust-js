// Mutations of src/runtime/scan.js (ADR 0517).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "scan-past-none",
    breaks: "`scan` goes on past its first `None`",
    file: "src/runtime/scan.js",
    find: "      if (next !== undefined) return { done: false, value: next };\n      if (!scanning.held) source.return?.();\n",
    replace: "      if (next !== undefined) return { done: false, value: next };\n",
    tests: ["test/corpus.test.ts", "-t", "^(scan_lazy_drops|held_iterator_dropped_by_its_owner).rs$"],
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
  {
    name: "scan-try-fold-by-next",
    breaks: "scan passes tryFold to its source",
    file: "src/runtime/scan.js",
    find: "  scanning.tryFold = (init, g, broke) => {",
    replace: "  scanning.tryFoldUnused = (init, g, broke) => {",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "scan-fold-by-next",
    breaks: "a `scan`'s fold steps it by `next`, not std's `try_fold`, so a `skip` inside asks the crate's own `nth(n)` (#216)",
    file: "src/runtime/scan.js",
    find: "    const folded = scanning.tryFold(init, g, () => false);\n",
    replace: "    const folded = $Steps.prototype.fold.call(scanning, init, g);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
];

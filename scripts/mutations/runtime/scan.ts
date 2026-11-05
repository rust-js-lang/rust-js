// Mutations of src/runtime/scan.js (ADR 0517).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
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
    find: "  $forward(scanning, source, { tryFold, fold });\n",
    replace: "  $forward(scanning, source, { fold });\n",
    tests: ["test/corpus.test.ts", "-t", "skip_try_fold"],
  },
  {
    name: "scan-fold-by-next",
    breaks: "a `scan`'s fold steps it by `next`, not std's `try_fold`, so a `skip` inside asks the crate's own `nth(n)` (#216)",
    file: "src/runtime/scan.js",
    find: "    const folded = $consumed(scanning, () => scanning.tryFold(init, g, () => false, unwind), ends);\n",
    replace: "    const folded = $Steps.prototype.fold.call(scanning, init, g, unwind);\n",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "scan-fold-unguarded",
    breaks: "a `scan` whose closure panics inside a fold leaves the accumulator undropped (#423)",
    file: "src/runtime/scan.js",
    find: "        const next = unwind === undefined ? f(state, x) :",
    replace: "        const next = true ? f(state, x) :",
    tests: ["test/corpus.test.ts", "-t", "^panic_mid_fold_drops.rs$"],
  },
  {
    name: "lazy-scan-drops-before-source",
    breaks: "a lazy `scan()` drops its function and state before what it steps, where `Scan { iter, f, state }` drops its source first (#760)",
    file: "src/runtime/scan.js",
    find: "      $closedAfter(\n        () => source.return?.(),\n        () => {\n          if (dropped) return;\n          dropped = true;\n          drop(handle ? state.value : state);\n        },\n      ),\n",
    replace: "      $closedAfter(\n        () => {\n          if (dropped) return;\n          dropped = true;\n          drop(handle ? state.value : state);\n        },\n        () => source.return?.(),\n      ),\n",
    tests: ["test/corpus.test.ts", "-t", "^scan_drops_its_function_and_state.rs$"],
  },
  {
    name: "lazy-scan-drops-twice",
    breaks: "a lazy `scan()` closed twice drops its function and state twice (#760)",
    file: "src/runtime/scan.js",
    find: "          if (dropped) return;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^scan_drops_its_function_and_state.rs$"],
  },
  {
    name: "scan-hint-dropped",
    breaks: "a `scan()`'s hint is `(0, None)`, not its source's upper bound (#733)",
    file: "src/runtime/scan.js",
    find: "  scanning.sizeHint = () => $upperOnly(source);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^size_hint_of_adapters.rs$"],
  },
];

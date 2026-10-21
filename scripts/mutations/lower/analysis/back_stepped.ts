// Mutations of src/lower/analysis/back_stepped.rs (ADR 0545).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "back-stepped-none",
    breaks: "no map the crate steps from its back is seen, and one held before it's stepped is refused",
    file: "src/lower/analysis/back_stepped.rs",
    find: "            stepped.extend(back_stepped_adapter(tcx, recognition, adapter));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_stepped_from_its_back.rs$"],
  },
  {
    name: "held-adapter-not-back-stepped",
    breaks: "a filter or a map a type holds, asked to be double-ended by the call that asks it of that type, isn't made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                            stepped.extend(back_stepped_adapter(tcx, recognition, held));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^filters_held_double_ended.rs$"],
  },
  {
    name: "held-adapter-opaque",
    breaks: "what an `impl DoubleEndedIterator` is isn't seen, so a filter behind one isn't made knowing its back",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                let asked = recognition.reveal(asked.self_ty().skip_binder().peel_refs());\n",
    replace: "                let asked = asked.self_ty().skip_binder().peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^filters_held_double_ended.rs$"],
  },
  {
    name: "held-adapter-through-adapters",
    breaks: "what a std adapter holds, `filter(p).map(f).rev()`'s filter, is made knowing its back where it's made, not by its `rev()`",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        walk.skip_current_subtree();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
  {
    name: "held-adapter-asked-itself",
    breaks: "the adapter a `rev()` asks to be double-ended is made knowing its back where it's made, not by its `rev()`",
    file: "src/lower/analysis/back_stepped.rs",
    find: "                        if held != asked {\n",
    replace: "                        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_lazy_range.rs$"],
    snapshots: true,
  },
];

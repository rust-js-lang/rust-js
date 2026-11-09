// Mutations of src/lower/recognition/zip_folds.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "zip-folding-apart-in-place",
    breaks:
      "a zip whose sides are `TrustedLen` but not both `TrustedRandomAccess`, made in place, is the recognizers' `$lazyZip`, whose `collect()` runs the longer side's closure once more than std's `SpecFold` (#380)",
    file: "src/lower/recognition/zip_folds.rs",
    find: "    (runs_code && (by_index || holds(a, trusted_len) && holds(b, trusted_len))).then_some(by_index)\n",
    replace: "    (runs_code && by_index).then_some(by_index)\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_fold_steps_the_shorter.rs$"],
  },
  {
    name: "map-of-a-zip-folded-apart-made-in-place",
    breaks:
      "a `map()` of a zip std folds as a whole is made in place, a JS `map` the collect steps by `next`, where std's `extend_trusted` folds it by the zip's fold: a reversed `Vec`'s rest is dropped back to front (#654)",
    file: "src/lower/recognition/zip_folds.rs",
    find: "        || (super::mapped_source(tcx, ty))\n",
    replace: "        || false && (super::mapped_source(tcx, ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_mapped_collect_folds.rs$"],
  },
  {
    name: "enumerate-of-a-zip-folded-apart-made-in-place",
    breaks:
      "an `enumerate()` of a zip std folds as a whole is made in place, stepped by `next` as it's collected, where std folds it by the zip's fold (#654)",
    file: "src/lower/recognition/zip_folds.rs",
    find: "            .or_else(|| super::enumerated_source(tcx, ty))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^zip_mapped_collect_folds.rs$"],
  },
  {
    name: "cloned-clones-nothing",
    breaks: "a `cloned()` is taken to clone nothing, so the panic of the crate's clone leaves what it collected undropped (#708)",
    file: "src/lower/recognition/zip_folds.rs",
    find: '        "std::iter::Cloned" => {\n',
    replace: '        "std::iter::Cloned" if false => {\n',
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "cycle-clones-nothing",
    breaks: "a `cycle()` is taken to clone nothing, so the panic of its clone of the crate's iterator leaves what it holds undropped (#708)",
    file: "src/lower/recognition/zip_folds.rs",
    find: '        "std::iter::Cycle" => first,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "repeat-clones-nothing",
    breaks: "a `repeat()` is taken to clone nothing, so the panic of the crate's clone leaves what its collect collected undropped (#708)",
    file: "src/lower/recognition/zip_folds.rs",
    find: '        "std::iter::Repeat" | "std::iter::RepeatN" => first,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_work_that_panics.rs$"],
  },
  {
    name: "zip-clone-runs-no-code",
    breaks: "a zip whose first side's step clones what may be the crate's own, a `repeat()`'s value, is taken to run no code, so it's made in place, folded as its `next()` steps it, cloning once more than std's `SpecFold` (#1162)",
    file: "src/lower/recognition/zip_folds.rs",
    find: "            .is_some_and(|t| t.is_closure() || t.is_fn() || clones_own(t))\n",
    replace: "            .is_some_and(|t| t.is_closure() || t.is_fn())\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_a_repeat_folds_to_the_shorter.rs$"],
  },
  {
    name: "derived-clone-runs-code",
    breaks: "a derived `Clone` of plain fields is taken to run the crate's code, so a zip of a `cloned()` of such values folds apart, std's struct where the recognizers' JS was the same (#1162)",
    file: "src/lower/recognition/zip_folds.rs",
    find: "                Some(imp) if tcx.is_automatically_derived(imp) => adt\n",
    replace: "                Some(imp) if false && tcx.is_automatically_derived(imp) => adt\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_generic.rs$"],
    snapshots: true,
  },
];

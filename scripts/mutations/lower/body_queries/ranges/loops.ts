// Mutations of src/lower/body_queries/ranges/loops.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inclusive-for-unranged",
    breaks:
      "`for k in 1..=n` gives `k` no range, so `k - 1` keeps its wrap (#337)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                Some((start, end, 0))\n",
    replace: "                None\n",
    tests: ["test/intervals-bridged.test.ts"],
  },
  {
    name: "inclusive-for-past-its-end",
    breaks:
      "`for k in 1..=n` is taken to reach `n + 1`, so `n - k` keeps its wrap",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                Some((start, end, 0))\n",
    replace: "                Some((start, end, -1))\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "another's value"],
  },
  {
    name: "value-range-end-unbounded",
    breaks:
      "`for k in 0..n` doesn't put `k` below `n`, so `n - k - 1` keeps its wrap",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                let end = end.map(|r| self.bounded_by_itself(r, end_at));\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "another's value"],
  },
  {
    name: "widened-counter-within-forgotten",
    breaks:
      "a widened counter found within what it was taken as is anything, so a counter below a string's length keeps its wrap, `(i + 1) >>> 0 < formatString.length`",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                        if was.join(&r) == *was {\n",
    replace: "                        if *was == r {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "below a JS string"],
  },
  {
    name: "growing-counter-forgotten",
    breaks:
      "a counter still growing is anything at once, so `i + 1 < formatString.length` keeps its wrap",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                        (was.hi < hi && r.hi <= hi && r.lo >= was.lo).then(|| {\n",
    replace:
      "                        (false && was.hi < hi && r.hi <= hi && r.lo >= was.lo).then(|| {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "below a JS string"],
  },
  {
    name: "ranges-for-changed-kept",
    breaks:
      "a range loop's end holds of a slice its body pops, so `items[i]` past its length reads `undefined`",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "        (!changed).then_some(slice)",
    replace: "        Some(slice)",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "ranges-loop-entry-only",
    breaks:
      "a loop's start is only what's known as it's entered, so `steps += 1` in a `loop` doesn't wrap",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "            let mut next = either(entry.clone(), either(here, continued));",
    replace:
      "            let mut next = {\n                let _ = (here, continued);\n                entry.clone()\n            };",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "ranges-continue-dropped",
    breaks:
      "what's known at a `continue` doesn't go round again, so `k + 1` of a `k` set to `n` before it doesn't wrap",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "            let continued = scope.and_then(|at| self.targets[at].continues.take());",
    replace:
      "            let continued = scope.and_then(|at| self.targets[at].continues.take()).and(None);",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "enumerate-index-unranged",
    breaks:
      "`for (i, t) in terraces.iter().enumerate()` gives `i` no bound, `$index(levels, i)` (#363)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                    && is_iterator_enumerate(self.tcx, def_id)\n",
    replace:
      "                    && !is_iterator_enumerate(self.tcx, def_id)\n",
    tests: ["test/lowering-collections.test.ts", "-t", "enumerate's index"],
  },
  {
    name: "enumerate-of-any-slice-fn",
    breaks:
      "an `enumerate()` of any function of a slice, one yielding more than its items, bounds its index by the slice, `xs[i]` read as `undefined` (#363)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                    && is_slice_items(self.tcx, items_def)\n",
    replace: "",
    tests: ["test/intervals-bridged.test.ts"],
  },
  {
    name: "enumerate-index-field-missed",
    breaks:
      "the index of an `enumerate()`'s `(i, x)` isn't bound, its item is looked for (#363)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                        .find(|field| field.field.index() == 0)\n",
    replace:
      "                        .find(|field| field.field.index() == 1)\n",
    tests: ["test/lowering-collections.test.ts", "-t", "enumerate's index"],
  },
  {
    name: "stepped-range-index-unknown",
    breaks: "a range's `step_by(n)`'s index isn't known below the slice's length its end is, `$index(v, i)`, not `v[i]` (rd260)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "                [range, n] if is_iterator_step_by(self.tcx, def_id) => {\n",
    replace: "                [range, n] if false && is_iterator_step_by(self.tcx, def_id) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^stepped_range_loops.rs$"],
    snapshots: true,
  },
];

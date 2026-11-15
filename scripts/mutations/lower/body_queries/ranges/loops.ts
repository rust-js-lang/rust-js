// Mutations of src/lower/body_queries/ranges/loops.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
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
  {
    name: "loop-cells-unchanged-by-calls",
    breaks: "a loop's bound by a `RefCell`'s length is kept each time round past a call that may change the cell, so `v.borrow_mut().drain(..j + 1)` of `j` in `k..end` drains past the end unchecked after the first time shortens it (#1885)",
    file: "src/lower/body_queries/ranges/loops.rs",
    find: "        let cell_changed = slice.0.contains(&CELL)\n",
    replace: "        let cell_changed = false && slice.0.contains(&CELL)\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_drained_in_a_loop\\.rs$"],
  },
];

// Mutations of src/lower/body_queries/ranges.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "let-else-present-index-unknown",
    breaks:
      "past `let Some(&n) = counts.get(i) else { break };` the index isn't known below a length, so `i += 1` wraps, `i = (i + 1) >>> 0`, where a `while let`'s is `i++` (ADR 0881)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    if let Some(init) = initializer\n                        && present::some_pattern(self.tcx, pattern)\n",
    replace:
      "                    if let Some(init) = initializer\n                        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^present_index_counts.rs$"],
    snapshots: true,
  },
  {
    name: "insert-index-below-length",
    breaks:
      "an `insert` at the length, `targets.insert(n, 9)` of `n = targets.len()`, is checked, `$insertAt`, as if it had to be below it (rd103)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                && (index.below.any(|(s, b)| k.same_length(s, &slice) && b.up_to())\n",
    replace: "                && (index.below.any(|(s, b)| k.same_length(s, &slice) && b.within())\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_remove_insert_in_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "insert-index-past-least",
    breaks:
      "an `insert` past what the test shows of the length is taken in bounds, `splice` where `$insertAt` panics as Rust's does",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    || k.lengths.get(&slice).is_some_and(|&least| index.hi <= least))\n",
    replace:
      "                    || k.lengths.get(&slice).is_some_and(|&least| index.hi <= least + 9))\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_remove_insert_in_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "value-difference-unbounded",
    breaks:
      "`n - k` of a `k < n` isn't known at least 1, so `n - k - 1` keeps its wrap",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    .map_or(i128::MIN, |x| -x.b);\n",
    replace: "                    .map_or(i128::MIN, |_| i128::MIN);\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "another's value"],
  },
  {
    name: "value-difference-past-bound",
    breaks:
      "`n - k` of a `k ≤ n + 1` is taken as at least 0, so `n - k` of `k = n + 1` is -1, not `u32::MAX`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    .map_or(i128::MIN, |x| -x.b);\n",
    replace: "                    .map_or(i128::MIN, |_| 0);\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "another's value"],
  },
  {
    name: "value-wrapped-sum-forgets",
    breaks:
      "`line + 1`, which may wrap, isn't known at most `line + 1`, so `line - k + 1` of a `k < size` keeps its wraps",
    file: "src/lower/body_queries/ranges.rs",
    find: "            (None, Some(made)) if op == BinOp::Add && (l.lo >= 0 || r.lo >= 0) => Range {\n",
    replace: "            (None, Some(made)) if false => Range {\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "another's value"],
  },
  {
    name: "value-assigned-keeps-own-bound",
    breaks:
      "`n -= 1` keeps `n ≤ n - 1`, its bound by the `n` it was, so `k < n` puts `k` two below `n` and `n - k - 1 - 1` doesn't wrap where it should",
    file: "src/lower/body_queries/ranges.rs",
    find: "                value.below.retain(|((_, v), _)| *v != var);\n",
    replace: "",
    tests: ["test/lowering-numbers.test.ts", "-t", "another's value"],
  },
  {
    name: "call-keeps-cells",
    breaks:
      "a call between a loop's test and `queue.borrow()[i]`, which may empty the queue, leaves its length known, and the index read unchecked",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if !views && let Some(k) = known.as_mut() {\n",
    replace: "        if false && !views && let Some(k) = known.as_mut() {\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
  },
  {
    name: "assign-drop-keeps-cells",
    breaks:
      "`slots.borrow_mut()[0] = other`, whose old value's drop empties the queue, leaves the queue's length known, and `queue.borrow()[0]` read unchecked",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    && cells::drop_may_change_cells(self.tcx, self.typing_env, thir[lhs].ty)\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
  },
  {
    name: "held-test-of-a-cell",
    breaks:
      "`let there = !queue.borrow().is_empty();` is held past a call that empties the queue, and `queue.borrow()[0]` read unchecked",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if reads_a_cell(self.tcx, thir, test) {\n",
    replace: "        if false && reads_a_cell(self.tcx, thir, test) {\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
  },
  {
    name: "drain-range-unjudged",
    breaks:
      "`queue.drain(0..i)` below the loop's `i < queue.len()` is `$drain(queue, 0, i)`, not React's `queue.splice(0, i)`",
    file: "src/lower/body_queries/ranges.rs",
    find: "            Some((SliceLength::Range | SliceLength::Drain, items)) => {\n",
    replace: "            Some((SliceLength::Range, items)) => {\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_vec_bounds"],
    snapshots: true,
  },
  {
    name: "labeled-block-not-a-target",
    breaks:
      "a labeled block's `break 'desc` is a jump the walk can't follow, so nothing its body's ranges show holds, and `indent + 1` of `indent < 3` is `(indent + 1) >>> 0`",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if !block.targeted_by_break {\n            return walk(self, known);\n",
    replace: "        if true {\n            return walk(self, known);\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a labeled block and a named constant",
    ],
  },
  {
    name: "labeled-block-breaks-unjoined",
    breaks:
      "what a `break` knows doesn't go past the scope it leaves, a labeled block's, so after one of `indent >= 3`, `indent + 1` is taken as below 3's, unwrapped, and 4294967295's is 4294967296",
    file: "src/lower/body_queries/ranges.rs",
    find: "            Some(broken) => {\n                *known = either(known.take(), Some(broken));\n                top\n            }\n",
    replace: "            Some(_) => top,\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a labeled block and a named constant",
    ],
  },
  {
    name: "named-constant-unknown",
    breaks:
      "a `const` of an integer is any value its type holds, so a counter tested `> LIMIT` adds one with a wrap, `(seen + 1) >>> 0`, where one tested `> 100` is `seen++`",
    file: "src/lower/body_queries/ranges.rs",
    find: "            ExprKind::NamedConst { def_id, args, .. } if ty_range.is_some() => constant_integer(self.tcx, def_id, args)\n",
    replace:
      "            ExprKind::NamedConst { def_id, args, .. } if ty_range.is_some() => None::<i128>.filter(|_| constant_integer(self.tcx, def_id, args).is_some())\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a labeled block and a named constant",
    ],
  },
  {
    name: "named-constants-judged",
    breaks:
      "an operator of constants alone, one named, is judged exact, so `usize::BITS - 1` is written `32 - 1`, where its wrap folds it to `31`",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) && !constant {\n",
    replace:
      "        if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a labeled block and a named constant",
    ],
  },
  {
    name: "cast-kept-out-of-range",
    breaks:
      "an `as` of an integer that may be outside its target's range is taken as its operand, so `-1 as usize` reads `items[-1]`, `undefined`, where Rust panics",
    file: "src/lower/body_queries/ranges.rs",
    find: "                let kept = value.filter(|v| ty_range.is_some_and(|(lo, hi)| lo <= v.lo && v.hi <= hi));\n",
    replace: "                let kept = value;\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a signed index tested not negative",
    ],
  },
  {
    name: "sized-length-of-zero-sized",
    breaks:
      "a slice of `()`'s length is taken as at most `isize`'s most, so `len() as isize - 1` of one longer is written without its wraps",
    file: "src/lower/body_queries/ranges.rs",
    find: "            .is_ok_and(|layout| !layout.is_zst())\n",
    replace: "            .is_ok()\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a signed index tested not negative",
    ],
  },
  {
    name: "sized-length-any-u32",
    breaks:
      "a slice's length of sized items may be `u32::MAX`, so `len() as isize - 1` keeps `((length | 0) - 1) | 0`",
    file: "src/lower/body_queries/ranges.rs",
    find: "            true => SIZED_SLICE_LENGTH_MOST,\n",
    replace: "            true => u32::MAX as i128,\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "a signed index tested not negative",
    ],
  },
  {
    name: "string-length-any-u32",
    breaks:
      "a JS string's `length` may be `u32::MAX`, so a counter two steps below it wraps, `(i + 1) >>> 0 < formatString.length`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                .map(|_| Range::of(0, STRING_LENGTH_MOST)),\n",
    replace: "                .map(|_| Range::of(0, u32::MAX as i128)),\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "below a JS string"],
  },
  {
    name: "string-length-past-int32",
    breaks:
      "a JS string's `length` may be `2^31`, past every engine's, so the sum of two may wrap, `(str.length + substr.length) >>> 0 < 40`",
    file: "src/lower/body_queries/ranges.rs",
    find: "const STRING_LENGTH_MOST: i128 = (1 << 31) - 1;\n",
    replace: "const STRING_LENGTH_MOST: i128 = 1 << 31;\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "lengths add without"],
  },
  {
    name: "string-length-below-engines",
    breaks:
      "a JS string's `length` is taken as at most `2^30 - 1`, below JavaScriptCore's, so three lengths add without the wrap they need",
    file: "src/lower/body_queries/ranges.rs",
    find: "const STRING_LENGTH_MOST: i128 = (1 << 31) - 1;\n",
    replace: "const STRING_LENGTH_MOST: i128 = (1 << 30) - 1;\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "lengths add without"],
  },
  {
    name: "ranges-return-goes-on",
    breaks:
      "nothing of type `!` ends the path, a `return` too, so what's known where an `if` returns goes on after it, and `items[1]` after `len < 2` returns is checked",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if self.thir[e].ty.is_never() {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "an index a condition shows",
    ],
  },
  {
    name: "ranges-if-else-only",
    breaks:
      "what follows any `if` is what its `else` shows, so `items[0]` after `if items.is_empty() { n = 1; }` reads `undefined`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                *known = either(yes, no);\n                value.or_else(top)",
    replace: "                *known = no;\n                value.or_else(top)",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "an index a condition shows",
    ],
  },
  {
    name: "ranges-resize-kept",
    breaks:
      "a `&mut Vec` given to a call keeps its length, so `items[0]` after `items.clear()` reads `undefined`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    false => k.changed(var),",
    replace: "                    false => _ = k.vars.remove(&var),",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "an index a condition shows",
    ],
  },
  {
    name: "ranges-vec-index-unknown",
    breaks:
      "a `Vec`'s `v[i]` is never known in bounds, so `items[1]` of a `&Vec` is checked",
    file: "src/lower/body_queries/ranges.rs",
    find: "            Some((SliceLength::Index | SliceLength::Swap | SliceLength::SwapRemove | SliceLength::Remove, items)) => {",
    replace:
      "            Some((SliceLength::Swap | SliceLength::SwapRemove | SliceLength::Remove, items)) => {",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "an index a condition shows",
    ],
  },
  {
    name: "ranges-break-dropped",
    breaks:
      "what's known at a `break` isn't known past its loop, so `n + 100` after a loop left by one wraps",
    file: "src/lower/body_queries/ranges.rs",
    find: "                *known = either(known.take(), Some(broken));",
    replace:
      "                *known = {\n                    let _ = broken;\n                    known.take()\n                };",
    tests: ["test/lowering-loops.test.ts", "-t", "while. loop from MIR tests"],
  },
  {
    name: "ranges-wrap-unbounded",
    breaks:
      "what can pass its type's largest doesn't wrap, so `n + 1` of a `u32` is `n + 1`",
    file: "src/lower/body_queries/ranges.rs",
    find: "        let within = made.clone().filter(|m| m.lo >= lo && m.hi <= hi);",
    replace: "        let within = made.clone().filter(|m| m.lo >= lo);",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "ranges-shift-unscaled",
    breaks:
      "`length >> 1` is taken at most the length, so `heap[leftIndex]` is checked",
    file: "src/lower/body_queries/ranges.rs",
    find: "Below { s: x.s + k as u32, ..x }.lowest()",
    replace: "Below { s: x.s, ..x }.lowest()",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "ranges-swap-unjudged",
    breaks:
      "a `swap`'s indexes aren't judged, so `heap.swap(index, rightIndex)` is `$swap`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                for index in ranges.into_iter().skip(1) {",
    replace:
      "                for index in ranges.into_iter().skip(1).filter(|_| false) {",
    tests: ["test/lowering-numbers.test.ts", "-t", "integer ranges show"],
  },
  {
    name: "callee-untold",
    breaks:
      "a call tells its function nothing, so `items[i]` of an index its caller tested is checked",
    file: "src/lower/body_queries/ranges.rs",
    find: "        let Some(params) = self.callees.get(&callee) else {",
    replace:
      "        let Some(params) = self.callees.get(&callee).filter(|_| false) else {",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "callee-bound-unmapped",
    breaks:
      "an index's bound by the slice its caller gives isn't the parameter's, so `siftUp`'s `heap[index]` is checked",
    file: "src/lower/body_queries/ranges.rs",
    find: "Some((theirs(slice)?, bound))",
    replace: "Some((theirs(slice).filter(|_| false)?, bound))",
    tests: ["test/lowering-functions.test.ts", "-t", "only calls reach"],
  },
  {
    name: "utf16-count-any-usize",
    breaks:
      "`s.encode_utf16().count()` is any `usize` to the ranges, so a counter below it keeps its wrap, `(i + 1) >>> 0 < s.length`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    reads_string_length(self.tcx, def_id) || reads_utf16_length(self.tcx, def_id, generic_args)\n",
    replace: "                    reads_string_length(self.tcx, def_id)\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a string's count of UTF-16 units is its length",
    ],
  },
  {
    name: "range-unjudged",
    breaks:
      "a slice's range is never judged, so `&v[..3]` past `v.len() > 3` keeps `$slice` (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                self.judge_range(fun, items, args[1], &before);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_ranges_known"],
    snapshots: true,
  },
  {
    name: "range-end-unchecked",
    breaks:
      "a range's end is taken as at most the length whatever it is, so `&items[..3]` of two items is two, where Rust panics (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: ".is_some_and(|end| up_to(&end) && start.lo >= 0 && start.hi <= end.lo),",
    replace: ".is_some_and(|end| start.lo >= 0 && start.hi <= end.lo),",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "slice_range_tested_panic|slice_range_past_length_panic",
    ],
  },
  {
    name: "range-start-past-end",
    breaks:
      "a range whose start may pass its end is in bounds, so `&items[3..1]` is empty, where Rust panics (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: " && start.lo >= 0 && start.hi <= end.lo),",
    replace: " && start.lo >= 0),",
    tests: ["test/corpus.test.ts", "-t", "slice_range_reversed_panic"],
  },
  {
    name: "range-from-unchecked",
    breaks:
      "a range from a start to the end is in bounds whatever the start, so `&items[5..]` of three items is empty, where Rust panics (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    (Some(start), None) => up_to(&start),\n",
    replace: "                    (Some(_), None) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "slice_range_from_panic"],
  },
  {
    name: "range-of-array-unknown",
    breaks:
      "an array's length, which its type says, shows no range of it in bounds, so `&items[..2]` of `[u32; 5]` keeps `$slice` (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                            || fixed.is_some_and(|n| r.hi <= n))\n",
    replace:
      "                            || fixed.is_some_and(|n| r.hi <= n && false))\n",
    tests: ["test/corpus.test.ts", "-t", "slice_ranges_known"],
    snapshots: true,
  },
  {
    name: "test-not-held",
    breaks:
      "a `bool` made of a test holds nothing, so `&array[..LIMIT]` past `if truncated` keeps `$slice` (ADR 0806)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                        k.tests.insert(var, (test, reads));\n",
    replace: "                        let _ = (test, reads);\n",
    tests: [
      "test/port-performance-track.test.ts",
      "-t",
      "slices a truncated array",
    ],
  },
  {
    name: "once-set-leaves-empty",
    breaks:
      "a `OnceCell` known empty is still known empty after it's set, so a second `set` is assigned and `Ok`, where Rust's is `Err`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    let empty = k.empty.swap_remove(&cell);\n",
    replace: "                    let empty = k.empty.contains(&cell);\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-set-through-holder-unforgotten",
    breaks:
      "a `set` of a cell through a clone of its `Rc` a struct holds leaves it known empty, so its own `set` after is assigned and `Ok`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    if !empty {\n                        k.escaped(cell.1);\n                    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-set-judged-once",
    breaks:
      "a `set` is judged empty where it's ever empty, not every time it's reached, so one in a loop of a cell made before it is assigned each turn",
    file: "src/lower/body_queries/ranges.rs",
    find: "            *self.sets.entry(fun).or_insert(true) &= empty;\n",
    replace: "            *self.sets.entry(fun).or_insert(false) |= empty;\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-mention-quiet",
    breaks:
      "a read of a variable that reaches a cell gives nothing its cells, so one given to `fill(&given)` is still known empty after",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if !self.quiet.contains(&e)\n            && let Some(k) = known.as_mut()\n        {\n            k.escaped(var);\n",
    replace:
      "        if !self.quiet.contains(&e)\n            && let Some(k) = known.as_mut()\n        {\n            let _ = (k, var);\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-place-unmentioned",
    breaks:
      "a variable borrowed, `&given`, isn't read, so a cell given to a call by reference is still known empty after",
    file: "src/lower/body_queries/ranges.rs",
    find: "            Some(var) => self.mention(strip(self.thir, e), var, known),\n",
    replace: "            Some(_) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-holder-reaches-nothing",
    breaks:
      "a struct made of a clone of an `Rc` reaches none of its cells, so one given to a call through it is still known empty",
    file: "src/lower/body_queries/ranges.rs",
    find: "                false => k.reaches.entry(var).or_default().extend(of),\n",
    replace: "                false => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-raw-borrow-unread",
    breaks:
      "a raw pointer to a struct that reaches a cell gives it nothing, so one set through the pointer is still known empty",
    file: "src/lower/body_queries/ranges.rs",
    find: "            ExprKind::RawBorrow { arg, .. } => {\n                self.place(arg, known);\n",
    replace:
      "            ExprKind::RawBorrow { arg, .. } => {\n                let _ = arg;\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-made-unknown",
    breaks:
      "a `OnceCell` a body makes isn't known empty, so React's `payload._ioInfo = ioInfo` is `if (payload._ioInfo === undefined) ..`",
    file: "src/lower/body_queries/ranges.rs",
    find: "            k.empty.extend(made.into_iter().map(|path| (path, var)));\n",
    replace: "            let _ = made;\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
    snapshots: true,
  },
  {
    name: "same-length-unread",
    breaks:
      "a table made `vec![0.0; terraces.len()]` is indexed `$index(levels, i)`, not as `terraces` is (#362)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                && (index.below.any(|(s, b)| k.same_length(s, &slice) && b.within())\n",
    replace: "                && (index.below.any(|(s, b)| *s == slice && b.within())\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "same-length-unmade",
    breaks: "no `let` of `vec![x; n]` is known as long as anything (#362)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                            k.same.insert((path.clone(), var), base);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "table-least-unrecorded",
    breaks:
      "`vec![0.0; n.max(2) + 1]` isn't known at least 3 long, `steps[$at(steps, 1)] = 1` (#367)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                            k.lengths.insert((path, var), count.lo);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "min-max-unranged",
    breaks:
      "an integer's `max` or `min` has no range, `n.max(2) + 1` no least (#367)",
    file: "src/lower/body_queries/ranges.rs",
    find: "            && let Some(larger) = ord_max_or_min(self.tcx, def_id)\n",
    replace:
      "            && let Some(larger) = ord_max_or_min(self.tcx, def_id).filter(|_| false)\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "table-rows-unmade",
    breaks:
      "`vec![vec![1.0; width]; height]`'s rows aren't known `width` long, `$index(board[r], c)` (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                        .chain(rows.map(|(base, count)| (vec![ITEM], base, count)))\n",
    replace:
      "                        .chain(rows.filter(|_| false).map(|(base, count)| (vec![ITEM], base, count)))\n",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "table-rows-unjudged",
    breaks: "an index of a table's row isn't judged by its rows' length (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "        let slice = self.slice(items).or_else(|| self.row_of(items));\n",
    replace: "        let slice = self.slice(items);\n",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "row-assigned-kept",
    breaks:
      "`board[0] = Vec::new()` leaves the rows known `width` long, `board[r][c]` past an empty row read as `undefined` (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                self.row_reached(lhs, known);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "row-borrowed-kept",
    breaks:
      "`let row = &mut board[0]; row.pop()` leaves the rows known `width` long (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    self.row_reached(arg, known);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "row-lent-kept",
    breaks: "`board[0].pop()` leaves the rows known `width` long (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                self.row_reached(place, known);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "rows-lent-kept",
    breaks:
      "`board.iter_mut()`, whose rows may be resized, leaves them known `width` long (#370)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    k.rows_changed(var);\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "rows are as long"],
  },
  {
    name: "rows-kept-never",
    breaks:
      "a table's `iter_mut()` loop that only writes its cells forgets its rows' lengths, so `board[r - 1][c]` after it is `$index(..)` (#372)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                let viewed = self.vec_view(fun).is_some() && self.rows_kept(arg);\n",
    replace: "                let viewed = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^table_rows_kept_by_views.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-of-a-match-unbound",
    breaks:
      "a `match` of a slice's `position` binds its `Some(index)` unknown, `$removeAt(children, index)`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    self.found_index_bound(&thir[arm].pattern, found.clone(), &mut here);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-of-an-if-let-unbound",
    breaks:
      "an `if let Some(index)` of a slice's `position` binds `index` unknown, `$removeAt(children, index)`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                self.found_index_bound(pat, found, known);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-of-a-let-else-unbound",
    breaks:
      "a `let Some(index) = .. else` of a slice's `position` binds `index` unknown, `$removeAt(children, index)`",
    file: "src/lower/body_queries/ranges.rs",
    find: "                self.found_index_bound(pattern, value.clone(), known);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-held-unknown",
    breaks:
      "a variable that holds a slice's `position`, `let index = ..; if let Some(index) = index`, holds nothing of it, `$insertAt(children, index, child)`",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if let Some(found) = self.found_index(e) {\n            return Some(found);\n        }\n",
    replace: "        if let Some(found) = self.found_index(e) {\n            let _ = found;\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
    snapshots: true,
  },
  {
    name: "ranges-callees-walked-as-a-stack",
    breaks:
      "each callee told otherwise is walked before the callers still waiting, a stack's order, so five callers of `small(n)` each tell it once it's walked, its `n` is forgotten past four changes, and its `n + 1` wraps, `(n + 1) >>> 0` (#866)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                    work.push_back(at);\n",
    replace: "                    work.push_front(at);\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_told_by_many_callers.rs$"],
    snapshots: true,
  },
  {
    name: "ranges-each-tell-counted",
    breaks:
      "each change of what a callee is told counts toward what's forgotten, its callers' tells before its walk too, so `small(n)`'s five callers make its `n` forgotten and its `n + 1` wraps, `(n + 1) >>> 0` (#866)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                if at.is_none_or(|at| std::mem::take(&mut walked[at])) {\n",
    replace: "                if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_told_by_many_callers.rs$"],
    snapshots: true,
  },
  {
    name: "ranges-diverging-call-goes-on",
    breaks:
      "an expression of type `!` but a `return`, `panic!()`'s call or a diverging function's, is one the body goes on past, so `if n > 100 { panic!(..) }` narrows nothing and `n + 1` wraps, `(n + 1) >>> 0` (#867)",
    file: "src/lower/body_queries/ranges.rs",
    find: "        if self.thir[e].ty.is_never() {\n",
    replace: "        if matches!(self.thir[e].kind, ExprKind::Return { .. } | ExprKind::Become { .. }) {\n",
    tests: ["test/corpus.test.ts", "-t", "^ranges_end_at_a_diverging_call.rs$"],
    snapshots: true,
  },
  {
    name: "guard-failure-unjoined",
    breaks: "each match arm is tried from the state before the match alone, so what an earlier arm's failed guard changed, a `v.pop()`, is unseen and the next arm's index unchecked (#1659)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                        before = either(before, here.clone());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^match_guard_that_shrank_a_vec_keeps_the_next_arms_index_checked.rs$"],
  },
  {
    name: "assign-op-place-first",
    breaks: "a compound assignment's place is walked before its right side, so the operator reads the place as it was and the right side's index is judged narrowed (#1660)",
    file: "src/lower/body_queries/ranges.rs",
    find: "                let r = self.expr(rhs, known);\n                let current = self.expr(lhs, known);\n",
    replace: "                let current = self.expr(lhs, known);\n                let r = self.expr(rhs, known);\n",
    tests: ["test/corpus.test.ts", "-t", "^compound_assignment_(reads_its_place_after_its_right_side|right_side_index_checked_first).rs$"],
  },
];

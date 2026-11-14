// Mutations of src/lower/body_queries/ranges/narrowing.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "value-test-unbounded",
    breaks:
      "`k < n` doesn't put `k` below `n`'s value, so `n - k - 1` where it holds keeps its wrap",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                let (l, r) = (self.bounded_by_itself(l, lhs), self.bounded_by_itself(r, rhs));\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "another's value"],
  },
  {
    name: "sum-narrowed-wrapped",
    breaks:
      "`i + 1 < n` narrows `i` where `i + 1` may have wrapped to 0, so `i += 2` below it is written without its wrap",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                if k >= 0 && hi + k <= most {\n",
    replace: "                if k >= 0 {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "below a JS string"],
  },
  {
    name: "sum-narrowed-unshifted",
    breaks:
      "`i + 1 < n` narrows `i` below `n`, not `n - 1`, so a `u8` counter below `n - 1` is taken as reaching 255, forgotten round its loop, and its `i += 1` wraps",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                        hi: other.hi - k,\n",
    replace: "                        hi: other.hi,\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "below a JS string"],
  },
  {
    name: "ranges-is-empty-unread",
    breaks:
      "`is_empty()` false shows nothing, so `items[0]` in its `else` is `$index(items, 0)`",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                if !holds\n                    && let Some(slice) = self.slice(items)",
    replace:
      "                if false\n                    && let Some(slice) = self.slice(items)",
    tests: ["test/intervals-bridged.test.ts"],
  },
  {
    name: "ranges-not-unflipped",
    breaks:
      "`!c` shows what `c` does, so `!items.is_empty()` shows nothing and `items[0]` is checked",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                let (yes, no) = self.branches(arg, known);\n                (no, yes)\n",
    replace: "                let (yes, no) = self.branches(arg, known);\n                (yes, no)\n",
    tests: ["test/intervals-bridged.test.ts"],
  },
  {
    name: "held-length-shows-nothing",
    breaks:
      "a variable narrowed to at least 1, at most a slice's length, shows nothing of the slice, so `keys[0]` past `length === 1` keeps `$index`",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "            k.at_least(below, narrowed.lo);\n",
    replace: "            let _ = (below, narrowed.lo);\n",
    tests: [
      "test/lowering-integer-ranges.test.ts",
      "-t",
      "a length held in a variable",
    ],
  },
  {
    name: "test-held-shows-nothing",
    breaks:
      "a branch on a `bool` that holds a test shows nothing of it, so `if truncated` keeps `$slice` (ADR 0806)",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                self.refine(*test, holds, known)\n",
    replace: "                let _ = (test, holds);\n",
    tests: [
      "test/port-performance-track.test.ts",
      "-t",
      "slices a truncated array",
    ],
  },
  {
    name: "present-index-unknown",
    breaks:
      "`while let Some(x) = v.get(i) { ..; i += 1; }` keeps `i = (i + 1) >>> 0`, its index's bound unknown (ADR 0881, amended)",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "            ExprKind::Let { expr, ref pat } if holds && present::some_pattern(self.tcx, pat) => {\n",
    replace:
      "            ExprKind::Let { expr, ref pat } if false && holds && present::some_pattern(self.tcx, pat) => {\n",
    tests: [
      "test/lowering-integer-ranges.test.ts",
      "-t",
      "an index an item is there at counts on without its wrap",
    ],
  },
  {
    name: "present-index-where-it-isnt",
    breaks:
      "where `v.get(i)` is `None`, `i` is taken to be an index too, and `i + 1` there loses its wrap",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "            ExprKind::Let { expr, ref pat } if holds && present::some_pattern(self.tcx, pat) => {\n",
    replace:
      "            ExprKind::Let { expr, ref pat } if present::some_pattern(self.tcx, pat) => {\n",
    tests: [
      "test/lowering-integer-ranges.test.ts",
      "-t",
      "an index an item is there at counts on without its wrap",
    ],
  },
  {
    name: "length-arms-not-passed",
    breaks:
      "a `_` arm after arms of 0, 1 and 2 of a `match list.len()` knows nothing of its length, `$slice(list, 0, (list.length - 1) >>> 0)` (rd21)",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "            *passed += 1;\n",
    replace: "            *passed += 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^match_of_a_length\\.rs$"],
    snapshots: true,
  },
  {
    name: "filtered-not-empty-unknown",
    breaks:
      "a `Vec` an `Option`'s `filter(|q| !q.is_empty())` shows isn't empty isn't known at least one long, so `queue[0]` is `$index(queue, 0)` (rd376)",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                    k.at_least(&((Vec::new(), var), Below::LENGTH), 1);\n",
    replace: "                    k.at_least(&((Vec::new(), var), Below::LENGTH), 0);\n",
    tests: ["test/corpus.test.ts", "-t", "^filtered_not_empty.rs$"],
    snapshots: true,
  },
  {
    name: "condition-left-shown-past-its-right",
    breaks:
      "what an `&&`'s left shows is shown again past its right, which may have changed it, so `v[i]` of `i < v.len() && { v.clear(); true }` reads `undefined` where Rust panics",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "                let (yes, no) = self.branches(rhs, &mut runs);\n",
    replace: "                let (mut yes, no) = self.branches(rhs, &mut runs);\n                self.refine(lhs, and, &mut yes);\n",
    tests: ["test/corpus.test.ts", "-t", "^test_changed_by_the_rest_of_its_condition\\.rs$"],
  },
  {
    name: "map-key-test-unshown",
    breaks:
      "`m.contains_key(k)` holding shows nothing, so ReactMarkupServer's `stashedErrors[id]` under its test is `$someValue($unwrap($someGet(..)))`, not upstream's `stashedErrors.get(id)`",
    file: "src/lower/body_queries/ranges/narrowing.rs",
    find: "            _ if holds && let Some(held) = self.tested_key(e) => {\n",
    replace: "            _ if false && holds && let Some(held) = self.tested_key(e) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^map_index_of_a_key_held\\.rs$"],
    snapshots: true,
  },
];

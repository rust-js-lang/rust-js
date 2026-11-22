// Mutations of crates/rust-js-lower/src/analysis/intervals/state.rs, split from intervals.rs (ADR 1137, #1672).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-copies-unnarrowed",
    breaks: "a comparison narrows only what's compared, not its copies, so `i += 1` under `i > 100` of `let mut i = start` may wrap",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "        for l in [local, root].into_iter().chain(copies) {\n",
    replace: "        for l in [root].into_iter().chain(copies.into_iter().take(0)) {\n",
    tests,
  },
  {
    name: "intervals-join-drops-what-intervals-show",
    breaks: "a join drops a relation one side has and the other's intervals show, `s = 0` before a loop, so `a[s - 1]` of a counter down from a length keeps its check",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "                    _ => two.below_by_intervals(x, y),\n",
    replace: "                    _ => None,\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
  {
    name: "intervals-lent-away-narrowed",
    breaks: "a local lent `&mut`, then tested, is taken as what the test showed, so `n - 1` past `n != 0` is JS's own where a raw pointer the borrow became set `n` to 0, and gives `-1` (#1782)",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "        if self.lent_away.contains(&root) {\n            return;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointer_written_after_its_owner_reads.rs$"],
  },
  {
    name: "below-past-never",
    breaks: "a join keeps no guarded relation, so `candidates[best]` of a `best` 0 before a loop and an `enumerate()` index in it keeps its check",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "            below_past: self.below_past_joined(other),\n",
    replace: "            below_past: Default::default(),\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "0 before it"],
  },
  {
    name: "least-of-below-unread",
    breaks: "a slice's least length isn't one past what's below it, so `candidates[best]` in a loop over its `enumerate()` isn't known held, and keeps its check",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "        self.least.get(&slice).copied().unwrap_or(0).max(below.unwrap_or(0))\n",
    replace: "        self.least.get(&slice).copied().unwrap_or(0).max(below.unwrap_or(0).min(0))\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "0 before it"],
  },
  {
    name: "keys-kept-when-written",
    breaks: "a key held is kept when its local is written, so `m[&k]` past `k += 1` is taken in bounds, a proved-call-bounds line",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "        self.keys.retain(|&(map, key)| map != local && key != local);\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "contains_key"],
  },
  {
    name: "comparison-kept-when-its-operand-is-written",
    breaks: "a held test's comparison is kept when what it compares is written, so `items[..n]` under `fits = n <= items.len()` past `n += 10` is taken in bounds, a proved-call-bounds line, and would read past the slice where Rust panics",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "        self.comparisons\n            .retain(|_, c| c.a != Value::Local(local) && c.b != Value::Local(local));\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "a test held in"],
  },
  {
    name: "held-test-unread-through-its-copy",
    breaks: "a test held in a `bool` and switched on as its copy narrows nothing, so `items[..LIMIT]` under `if truncated` keeps its check, a call-bounds line",
    file: "crates/rust-js-lower/src/analysis/intervals/state.rs",
    find: "            .or_else(|| self.comparisons.get(&self.root(local)))\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "a test held in"],
  },
];

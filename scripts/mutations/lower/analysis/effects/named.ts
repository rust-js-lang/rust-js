// Mutations of crates/rust-js-lower/src/analysis/effects/named.rs, what std's signatures can't say of what a call writes (#2112 B).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/effects/named.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "keeps nothing the forgetting rules forget"];

export const mutations: Mutation[] = [
  {
    name: "rc-deref-writes-cells",
    breaks: "an `Rc`'s `Deref` is taken as a call that may write a cell, forgetting its length, so React's `listeners.borrow()[i]` keeps its check, a call-bounds line (#1839)",
    file,
    find: "        || rc_deref(tcx, callee, args)\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "range-step-writes-cells",
    breaks: "a `for` loop's `next()` of a range, std's, is taken to may write a cell, so `batch.borrow()[i]` of `i` in `(0..batch.borrow().len()).step_by(2)` keeps its check (ADR 1168, #1948)",
    file,
    find: "        || range_step(tcx, callee, args).is_some()\n",
    replace: "",
    tests: ["test/lowering-cell-flow.test.ts", "-t", "a held borrow counts the cells"],
  },
  {
    name: "any-call-reads-cells",
    breaks: "any call is taken to write no cell, so past a held length's `borrow_mut().clear()` in the loop `batch.borrow()[i]` reads past the end, `undefined` where Rust panics (ADR 1168, #1948)",
    file,
    find: "        || range_step(tcx, callee, args).is_some()\n",
    replace: "        || true\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_length_across_a_loops_steps.rs$"],
  },
];

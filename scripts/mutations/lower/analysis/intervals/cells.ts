// Mutations of src/lower/analysis/intervals/cells.rs, what the MIR intervals
// know of a collection a `RefCell` holds (ADR 1137, #1741).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/cells.rs";
const tests = ["test/lowering-integer-ranges.test.ts", "-t", "a collection a RefCell holds"];

export const mutations: Mutation[] = [
  {
    name: "cells-borrow-not-the-cell",
    breaks: "a `RefCell`'s `borrow()` isn't a copy of the cell, so `stack.borrow()[stack.borrow().len() - 1]` past `!is_empty()` keeps its wrap, left to the bridge",
    file,
    find: "            after.copies.insert(local, of);\n",
    replace: "            let _ = (local, of);\n",
    tests,
  },
  {
    name: "rc-deref-not-the-cell",
    breaks: "an `Rc`'s `Deref` isn't a copy of its cell, so React's `listeners.borrow()[i]` under `i < listeners.borrow().len()` keeps its check, a call-bounds line (#1839)",
    file,
    find: "                || rc_deref(self.tcx, callee, callee_args))\n",
    replace: "                || false && rc_deref(self.tcx, callee, callee_args))\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
];

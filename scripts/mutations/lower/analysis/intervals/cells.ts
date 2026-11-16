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
    name: "cells-calls-write-nothing",
    breaks: "a call of the crate's given a `RefCell`, `shorten_cell(v)`, is taken as writing nothing of it, so `v.borrow()[k]` under `k < v.borrow().len()` past it is unchecked and reads `undefined` where Rust panics; a callback or a `fn` pointer's call is no named callee, and forgets regardless",
    file,
    find: "        !reads\n",
    replace: "        let _ = reads;\n        false\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "refcell, tested wrap"],
  },
  {
    name: "cells-raw-write-quiet",
    breaks: "a write through a raw pointer is taken as reaching no cell, so past `*p = Vec::new()` of a pointer to one's `Vec` `len() - 1` is JS's own and gives `-1`",
    file,
    find: "            .any(|(base, elem)| matches!(elem, mir::PlaceElem::Deref) && base.ty(&*self.body, self.tcx).ty.is_raw_ptr())\n",
    replace: "            .any(|(base, elem)| false && matches!(elem, mir::PlaceElem::Deref) && base.ty(&*self.body, self.tcx).ty.is_raw_ptr())\n",
    tests,
  },
  {
    name: "cells-drop-writes-nothing",
    breaks: "a drop whose destructor writes a `RefCell` is taken as writing nothing of it, so past it `len() - 1` is JS's own and gives `-1`",
    file,
    find: "        !guard && drop_runs_code(self.tcx, self.typing_env, ty)\n",
    replace: "        false && drop_runs_code(self.tcx, self.typing_env, ty)\n",
    tests,
  },
  {
    name: "guard-drop-runs-code",
    breaks: "a `Ref`'s drop of a `Vec` of `Rc<dyn Fn>`s is taken to run the items' code, forgetting the cell's length, so `queue.borrow_mut()[i] = task` under `i < queue.borrow().len()` keeps its check, a call-bounds line (#1815)",
    file,
    find: "        !guard && drop_runs_code(self.tcx, self.typing_env, ty)\n",
    replace: "        drop_runs_code(self.tcx, self.typing_env, ty)\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "guard keeps"],
  },
  {
    name: "keys-kept-when-cells-change",
    breaks: "a key a `RefCell`'s map holds is kept past a call that may write the cell, so `cache.borrow()[&k]` past `clear()` is taken in bounds, a proved-call-bounds line",
    file: "src/lower/analysis/intervals/cells.rs",
    find: "        self.keys.retain(|&(map, _)| map != cell);\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "contains_key"],
  },
  {
    name: "equal-lengths-kept-when-resized",
    breaks: "two slices as long are kept so when one's length may change, so `left[i]` under `i < terraces.len()` past `left.pop()` is taken in bounds, a proved-call-bounds line",
    file: "src/lower/analysis/intervals/cells.rs",
    find: "        self.equal_lens.retain(|&(a, b)| a != cell && b != cell);\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "a table is as long"],
  },
  {
    name: "rc-deref-not-the-cell",
    breaks: "an `Rc`'s `Deref` isn't a copy of its cell, so React's `listeners.borrow()[i]` under `i < listeners.borrow().len()` keeps its check, a call-bounds line (#1839)",
    file,
    find: "                || rc_deref(self.tcx, callee, callee_args))\n",
    replace: "                || false && rc_deref(self.tcx, callee, callee_args))\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "rc-deref-writes-cells",
    breaks: "an `Rc`'s `Deref` is taken as a call that may write a cell, forgetting its length, so React's `listeners.borrow()[i]` keeps its check, a call-bounds line (#1839)",
    file,
    find: "            || rc_deref(self.tcx, callee, callee_args)\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "cells-behind-pointers-unseen",
    breaks: "a `Box`'s `RefCell` isn't a cell, so past a closure that clears it `items.borrow()[i]` under `i < items.borrow().len()` is taken in bounds, a proved-call-bounds line (#1839)",
    file,
    find: "        _ => (rc_pointee(tcx, ty).or_else(|| ty.boxed_ty())).is_some_and(|of| holds_cell(tcx, of)),\n",
    replace: "        _ => false,\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "cells-behind-one-pointer",
    breaks: "an `Rc` of an `Rc`'s `RefCell` isn't a cell, so past a closure that clears it `items.borrow()[i]` is taken in bounds, a proved-call-bounds line (#1839)",
    file,
    find: "        _ => (rc_pointee(tcx, ty).or_else(|| ty.boxed_ty())).is_some_and(|of| holds_cell(tcx, of)),\n",
    replace: "        _ => (rc_pointee(tcx, ty).or_else(|| ty.boxed_ty()))\n            .is_some_and(|of| matches!(of.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCell))),\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "range-step-writes-cells",
    breaks: "a `for` loop's `next()` of a range, std's, is taken to may write a cell, so `batch.borrow()[i]` of `i` in `(0..batch.borrow().len()).step_by(2)` keeps its check (ADR 1168, #1948)",
    file,
    find: "            || range_step(self.tcx, callee, callee_args).is_some()\n",
    replace: "",
    tests: ["test/lowering-cell-flow.test.ts", "-t", "a held borrow counts the cells"],
  },
  {
    name: "any-call-reads-cells",
    breaks: "any call is taken to write no cell, so past a held length's `borrow_mut().clear()` in the loop `batch.borrow()[i]` reads past the end, `undefined` where Rust panics (ADR 1168, #1948)",
    file,
    find: "            || range_step(self.tcx, callee, callee_args).is_some()\n",
    replace: "            || true\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_length_across_a_loops_steps.rs$"],
  },
];

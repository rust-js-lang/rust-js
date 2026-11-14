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
    breaks: "a call given a `RefCell` is taken as writing nothing of it, so past one that clears it `len() - 1` is JS's own and gives `-1`",
    file,
    find: "        !reads\n",
    replace: "        let _ = reads;\n        false\n",
    tests,
  },
  {
    name: "cells-raw-write-quiet",
    breaks: "a write through a raw pointer is taken as reaching no cell, so past `*p = Vec::new()` of a pointer to one's `Vec` `len() - 1` is JS's own and gives `-1`",
    file,
    find: "            .any(|(base, elem)| matches!(elem, mir::PlaceElem::Deref) && base.ty(self.body, self.tcx).ty.is_raw_ptr())\n",
    replace: "            .any(|(base, elem)| false && matches!(elem, mir::PlaceElem::Deref) && base.ty(self.body, self.tcx).ty.is_raw_ptr())\n",
    tests,
  },
  {
    name: "cells-drop-writes-nothing",
    breaks: "a drop whose destructor writes a `RefCell` is taken as writing nothing of it, so past it `len() - 1` is JS's own and gives `-1`",
    file,
    find: "        drop_runs_code(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty)\n",
    replace: "        false && drop_runs_code(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty)\n",
    tests,
  },
];

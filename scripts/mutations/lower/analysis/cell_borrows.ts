// Mutations of src/lower/analysis/cell_borrows.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-key-never-same",
    breaks: "a thread-local's borrow is never its own, so `LOG`'s second borrow in its own closure isn't counted and doesn't throw",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        (Cell::Key(x), Cell::Key(y)) => x == y,\n",
    replace: "        (Cell::Key(_), Cell::Key(_)) => false,\n",
    tests: ["test/lowering.test.ts", "-t", "closures call only what can't borrow"],
  },
  {
    name: "crate-call-borrows-nothing",
    breaks: "a function of the crate's is taken to borrow nothing, so `logged()` in `LOG`'s closure makes `LOG` plain, and its borrow doesn't throw",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Body(id, instance.args),\n",
    replace: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Nothing,\n",
    tests: ["test/lowering.test.ts", "-t", "closures call only what can't borrow"],
  },
  {
    name: "crate-call-borrows-nothing-inside",
    breaks: "a function of the crate's is taken to borrow nothing, so a thread-local's borrow inside its own closure isn't counted",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Body(id, instance.args),\n",
    replace: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "thread_local_borrow_inside"],
  },
  {
    name: "crate-call-borrows-anything",
    breaks: "a function of the crate's may borrow anything, as before, so `timerQueue` and `taskQueue` stay `{ value }` cells",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                    Called::Body(body, args) => found.runs.push((body, Some(args))),\n",
    replace: "                    Called::Body(..) => found.any = true,\n",
    tests: ["test/lowering.test.ts", "-t", "closures call only what can't borrow"],
  },
  {
    name: "body-asks-of-nothing",
    breaks: "a thread-local's `with_borrow_mut` closure that borrows it again keeps it plain, unchecked",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Some(Some(summary)) => summary.asks(tcx, held),\n",
    replace: "            Some(Some(_)) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "thread_local_borrow_inside"],
  },
  {
    "name": "jsx-builders-ask",
    "breaks": "making an element counts as code that may ask, so a field's `RefCell` lent to a prop is a counted cell",
    "file": "src/lower/analysis/cell_borrows.rs",
    "find": "            JsForm::Jsx(_)\n                | JsForm::Prop(_)",
    "replace": "            JsForm::Text\n                | JsForm::Prop(_)",
    "tests": [
      "test/jsx.test.ts",
      "-t",
      "borrowed for a prop is its value"
    ]
  },
];

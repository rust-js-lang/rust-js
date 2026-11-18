// Mutations of src/lower/analysis/intervals/guards.rs, a `RefCell`'s
// `borrow_mut()` guard held while nothing else reaches the cell (ADR 1137,
// #1815).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/guards.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "guard keeps"];

export const mutations: Mutation[] = [
  {
    name: "guards-none",
    breaks: "no guard is held, so `items.remove(i)` past a call `f(items[i])` keeps its check, a call-bounds line",
    file,
    find: "        guards.held.insert(guard);\n",
    replace: "",
    tests,
  },
  {
    name: "guard-made-of-any-call",
    breaks: "a `RefMut` any call gives, a function's of the crate's, is taken for a cell's guard",
    file,
    find: "                        (func.const_fn_def()).is_some_and(|(callee, _)| refcell_borrow(tcx, callee) == Some(true))\n",
    replace: "                        true\n",
    tests,
  },
];

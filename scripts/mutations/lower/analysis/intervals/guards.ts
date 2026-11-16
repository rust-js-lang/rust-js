// Mutations of src/lower/analysis/intervals/guards.rs, a `RefCell`'s
// `borrow_mut()` guard held while nothing else reaches the cell (ADR 1137,
// #1815).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/guards.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "guard keeps"];

export const mutations: Mutation[] = [
  {
    name: "guard-lent-keeps-length",
    breaks: "a call given a guard's `&mut Vec`, `truncate` or a function of the crate's, keeps its cell's length, so `items[i]` past `items.truncate(0)` is taken in bounds, a proved-call-bounds line, and would write past it",
    file,
    find: "                after.forget_lengths(state.root(lent));\n",
    replace: "",
    tests,
  },
  {
    name: "guard-reborrow-unlent",
    breaks: "a reborrow of a guard's `&mut Vec`, `&mut (*m)`, given to `truncate` keeps the cell's length, a proved-call-bounds line",
    file,
    find: "                guards.lends.extend(through);\n",
    replace: "",
    tests,
  },
  {
    name: "guard-reborrow-not-lent",
    breaks: "a held guard's `&mut` reborrowed into a local, `let all = &mut *items; all.truncate(1)`, is lent no further, so the call given `&mut (*all)` keeps the cell's length, and `(items.len() - 2) as u32` under `items.len() > 1` gives -1 where Rust wraps (#1928, #1929)",
    file,
    find: "                    through.push(holder);\n                    lent_through(body, holder, through)\n",
    replace: "                    through.push(holder);\n                    false\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "guard"],
  },
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

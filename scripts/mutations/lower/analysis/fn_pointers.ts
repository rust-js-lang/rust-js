// Mutations of src/lower/analysis/fn_pointers.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fn-pointer-made-of-nothing",
    breaks: "no coercion makes a fn pointer, so `report`'s `f`, which may be `touch`, borrows nothing, its borrow isn't counted, and `touch`'s doesn't panic",
    file: "src/lower/analysis/fn_pointers.rs",
    find: "            Some(key) if !args.has_param() => self.made.entry(key).or_default().push((f, args)),\n",
    replace: "            Some(_) if !args.has_param() => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_pointer_borrows_held.rs$"],
  },
  {
    name: "fn-pointer-closures-made-of-nothing",
    breaks: "a closure made a fn pointer isn't one it may be, so `record`'s `f`, `counted`, borrows nothing, and its borrow doesn't panic",
    file: "src/lower/analysis/fn_pointers.rs",
    find: "                    ty::Closure(closure, args) => self.found.made(tcx, adjustment.target, closure, args),\n",
    replace: "                    ty::Closure(..) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_pointer_closure_borrows_held.rs$"],
  },
  {
    name: "fn-pointer-never-from-outside",
    breaks: "no value comes from outside the crate's code, so what JS gives `apply` borrows nothing, and its borrow isn't counted",
    file: "src/lower/analysis/fn_pointers.rs",
    find: "        let held = held(recognition, ty);\n        self.all_open |= held.any;\n        self.open.extend(held.keys);\n",
    replace: "        let _ = (recognition, ty);\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "fn pointer runs what the crate made"],
  },
];

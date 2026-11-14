// Mutations of src/lower/analysis/intervals/called_on.rs, a `Vec` lent
// `&mut` only to the calls it's given to (ADR 1137, #1672 M3, step 5).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/called_on.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "only to calls"];

export const mutations: Mutation[] = [
  {
    name: "called-on-none",
    breaks: "a `Vec` lent `&mut` to any call is lent for the whole body, so `v[i]` past `v.push(4)` and `i < v.len()` keeps its check, a call-bounds line",
    file,
    find: "    borrows.retain(|_, of| !lent_otherwise.contains(of));\n",
    replace: "    borrows.clear();\n",
    tests,
  },
  {
    name: "called-on-forgets-nothing",
    breaks: "a call that may change a `Vec`'s length forgets nothing of it, so `v[i]` past `v.clear()` is taken in bounds, a mir-only-call-bounds line, and would read past it",
    file,
    find: "                after.forget_lengths(state.root(of));\n",
    replace: "                let _ = (state, of);\n",
    tests,
  },
  {
    name: "called-on-every-call-keeps",
    breaks: "every call keeps a `Vec`'s length, `clear()` too, so `v[i]` past it is taken in bounds, a mir-only-call-bounds line",
    file,
    find: "            .is_some_and(|(callee, _)| keeps_vec_length(self.tcx, callee))\n",
    replace: "            .is_some_and(|(callee, _)| true || keeps_vec_length(self.tcx, callee))\n",
    tests,
  },
  {
    name: "called-on-moved-borrow-unseen",
    breaks: "a `&mut` of a `Vec` moved to another local isn't lent, so `s.clear()` of `let s = r` of `let r = &mut v` forgets `s`'s length, not `v`'s, and `v[i]` past it is taken in bounds",
    file,
    find: "                if mentions(statement, *holder) && !reborrows(statement, *holder) {\n",
    replace: "                if false && mentions(statement, *holder) && !reborrows(statement, *holder) {\n",
    tests,
  },
  {
    name: "called-on-deref-not-a-copy",
    breaks: "a `Vec`'s `deref_mut` slice isn't of its length, so `v.swap(i, j)` past `i < v.len() && j < v.len()` keeps its check, a call-bounds line",
    file,
    find: "            after.copies.insert(local, state.root(of));\n",
    replace: "            let _ = (local, state, of);\n",
    tests,
  },
];

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
    find: "                if mentions(statement, *holder) && !reborrows(statement, *holder, &closures, &borrows) {\n",
    replace: "                if false && mentions(statement, *holder) && !reborrows(statement, *holder, &closures, &borrows) {\n",
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
  {
    name: "called-on-closures-lend-otherwise",
    breaks: "a closure that captures a `Vec`'s `&mut` lends it for the whole body, so `args[i]` under `for i in 0..args.len()` past `take()`'s last call is `$index(args, i)`: right, but not the JS a person writes (#1946)",
    file: "src/lower/analysis/intervals/called_on.rs",
    find: "                            && let Some(&of) = borrows.get(&held)\n",
    replace: "                            && let Some(&of) = borrows.get(&held).filter(|_| false)\n",
    tests: ["test/lowering-loops.test.ts", "-t", "Console's loops, spreads and calls read as written"],
  },
  {
    name: "called-on-fake-read-a-use",
    breaks: "borrowck's `FakeRead` of a `let` lends what it reads, so `let mut take = || args.pop_front();` lends `args` for the whole body and `args[i]` past it is `$index`: right, but not the JS a person writes (#1946)",
    file: "src/lower/analysis/intervals/called_on.rs",
    find: "    if matches!(statement.kind, StatementKind::FakeRead(..)) {\n        return false;\n    }\n",
    replace: "",
    tests: ["test/lowering-loops.test.ts", "-t", "Console's loops, spreads and calls read as written"],
  },
  {
    name: "called-on-closure-call-keeps-length",
    breaks: "a closure's `call_mut` is taken to keep the length of the `Vec` it borrows, so `args[i]` under `i < n` of `n = args.len()` read before `take()` pops reads `undefined` where Rust panics (#1946)",
    file: "src/lower/analysis/intervals/called_on.rs",
    find: "            .is_some_and(|(callee, _)| keeps_vec_length(self.tcx, callee))\n",
    replace: "            .is_some_and(|(callee, _)| keeps_vec_length(self.tcx, callee) || self.tcx.def_path_str(callee).ends_with(\"call_mut\"))\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_unsound_directions"],
  },
];

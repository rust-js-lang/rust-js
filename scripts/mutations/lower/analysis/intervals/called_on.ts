// Mutations of src/lower/analysis/intervals/called_on.rs, a `Vec` lent
// `&mut` only to the calls it's given to (ADR 1137, #1672 M3, step 5).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/called_on.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "only to calls"];

export const mutations: Mutation[] = [
  {
    name: "called-on-deref-not-a-copy",
    breaks: "a `Vec`'s `deref_mut` slice isn't of its length, so `v.swap(i, j)` past `i < v.len() && j < v.len()` keeps its check, a call-bounds line",
    file,
    find: "            after.copies.insert(local, state.root(of));\n",
    replace: "            let _ = (local, state, of);\n",
    tests,
  },
  {
    name: "std-given-code-runs-none",
    breaks: "std's code given a closure, `Some(1).map(|x| { clear_it(); x })`, is taken to run none of the crate's, so an escaped `Vec`'s length is kept past it, and `(o.len() - 3) as u32` prints -3 where Rust wraps (#1966)",
    file: "src/lower/analysis/intervals/called_on.rs",
    find: "            ty::Closure(..)\n            | ty::CoroutineClosure(..)\n            | ty::Coroutine(..)\n            | ty::Dynamic(..)\n            | ty::FnPtr(..)\n",
    replace: "            ty::CoroutineClosure(..)\n            | ty::Coroutine(..)\n            | ty::Dynamic(..)\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_lent_to_code_that_keeps_a_pointer.rs$"],
  },
];

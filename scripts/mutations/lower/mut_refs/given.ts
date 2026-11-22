// Mutations of crates/rust-js-lower/src/mut_refs/given.rs, what a type parameter is given
// for a `&mut` to a function or an iterator (#992, #1909).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/mut_refs/given.rs";
const through = ["test/corpus.test.ts", "-t", "^fn_given_through_a_mut_box.rs$"];
const refused = ["test/corpus.test.ts", "-t", "^fn_box_called_and_given_back_refused.rs$"];

export const mutations: Mutation[] = [
  {
    name: "fn-only-called-given-its-box",
    breaks: "a `&mut` to a boxed `dyn Fn` given for an `F: FnOnce()` is its box, `{ get value() .. }`, which `run()` throws of, where Rust calls it (#1909)",
    file,
    find: "                || self.called_through(pointee) && self.only_called(def_id, declared))\n",
    replace: "                || false && self.called_through(pointee) && self.only_called(def_id, declared))\n",
    tests: through,
  },
  {
    name: "fn-given-back-taken-as-only-called",
    breaks: "an `F` its function gives back is taken as only called, so a `&mut` to a boxed function is given as the function, and the caller's write through what comes back is lost",
    file,
    find: "        called && !mentions(sig.output()) && sig.inputs()",
    replace: "        called && sig.inputs()",
    tests: refused,
  },
  {
    name: "fn-held-mut-not-read-through",
    breaks: "a `&mut` to a boxed function held in a `&mut` that's boxed itself is given as that box, `invoke(lent)`, not `invoke(lent.value)`, and its call throws",
    file,
    find: "                self.called_value(Expr::member(value, js::HELD), *inner)\n",
    replace: "                self.called_value(value, *inner)\n",
    tests: through,
  },
  {
    name: "parameter-read-as-a-function-boxed",
    breaks: "a `&mut` to a closure or an iterator given for `impl Is<F>` of `F: FnMut()` or `I: Iterator` is generic code's box, which `f()` calls and `next()` steps: `f is not a function`, `[]` (#992)",
    file,
    find: "                && self.read_as_stepped(self.tcx.param_env(def_id), declared)\n",
    replace: "                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_for_a_parameter_read_as_another.rs$"],
  },
  {
    name: "collection-for-an-into-iterator-unboxed",
    breaks: "a `&mut Vec` given for `impl Is<I>` of `I: IntoIterator` is the array, where its dictionary's `into_iter` takes its cell, `cell.value` undefined (#992)",
    file,
    find: "            && ((self.is_callable(pointee) || self.implements_iterator(pointee))\n",
    replace: "            && ((true)\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_given_for_a_parameter_read_as_another.rs$"],
  },
];

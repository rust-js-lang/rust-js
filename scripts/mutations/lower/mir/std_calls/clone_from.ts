// Mutations of crates/rust-js-mir/src/std_calls/clone_from.rs (#919, #922, #927, #928).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clone-from-option-replaced",
    breaks: "`w.clone_from(&v)` of an `Option<Vec<D>>` of two `Some`s is `*w = v.clone()`, every clone before the old one's drop, not its `Vec`'s in place (#922)",
    file: "crates/rust-js-mir/src/std_calls/clone_from.rs",
    find: "                this.option_clone_over(ty, (place, pointer), source.clone(), span)\n",
    replace: "                Ok(None)\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_std_own.rs$"],
  },
  {
    name: "clone-from-forwarded-unrefused",
    breaks: "`clone_from` of an `Option` of a type whose own `clone_from` is the crate's is `*dest = source.clone()`, which never calls it (#922)",
    file: "crates/rust-js-mir/src/std_calls/clone_from.rs",
    find: "            if self.forwards_clone_from(ty, span)? {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_of_a_reverse_of_a_result_refused.rs$"],
  },
  {
    name: "clone-from-result-direct-unwritten",
    breaks: "a direct `clone_from` of a `Result` through a `&mut`, which is the `Result` itself, isn't written as its value's where both are `Ok` or both `Err`, so it's refused (#2028)",
    file: "crates/rust-js-mir/src/std_calls/clone_from.rs",
    find: "            if let Some(sides) = self.result_sides(ty, to.clone(), source.clone(), span)? {\n",
    replace: "            if let Some(sides) = None::<Vec<(Expr, Vec<Stmt>)>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_of_a_result_through_a_mut_ref.rs$"],
  },
  {
    name: "generic-clone-from-place-not-given-back",
    breaks: "a generic `clone_from` of a local, given its dictionary's `cloneFrom` in a box of it, never takes back what it wrote: a `None` written over by a `Some` stays `None` (#928)",
    file: "crates/rust-js-mir/src/std_calls/clone_from.rs",
    find: "                        StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),\n",
    replace: "                        StmtKind::Expr(Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_generic_std_own.rs$"],
  },
  {
    name: "generic-clone-from-box-made-again",
    breaks: "a generic `clone_from` of what its `&mut T` points at gives its dictionary a box of a copy, `{ value: dest.value }`, not the box it was given (#928)",
    file: "crates/rust-js-mir/src/std_calls/clone_from.rs",
    find: "                Value::Ref(place) => self.box_of_place(state, place),\n",
    replace: "                Value::Ref(_) => None,\n",
    tests: ["test/lowering-std-unwinds.test.ts", "-t", "generic clone_from"],
  },
];

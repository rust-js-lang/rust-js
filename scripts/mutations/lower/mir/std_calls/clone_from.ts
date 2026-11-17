// Mutations of src/lower/mir/std_calls/clone_from.rs (#919, #922, #927, #928).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clone-from-vec-replaced",
    breaks: "`w.clone_from(&v)` of a `Vec` of items with a destructor clones every item, then drops the old `Vec`, not std's `SpecCloneIntoVec` order (#919)",
    file: "src/lower/mir/std_calls/clone_from.rs",
    find: "            if let Some(into) = self.clone_into(ty, span)? {\n",
    replace: "            if let Some(into) = self.clone_into(ty, span)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(vec_clone_from_in_place|clone_from_std_own).rs$"],
  },
  {
    name: "clone-from-option-replaced",
    breaks: "`w.clone_from(&v)` of an `Option<Vec<D>>` of two `Some`s is `*w = v.clone()`, every clone before the old one's drop, not its `Vec`'s in place (#922)",
    file: "src/lower/mir/std_calls/clone_from.rs",
    find: "            let inner = self.option_of(ty).filter(|&inner| !self.boxed_payload(inner));\n",
    replace: "            let inner = self.option_of(ty).filter(|&inner| false && !self.boxed_payload(inner));\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_std_own.rs$"],
  },
  {
    name: "clone-from-forwarded-unrefused",
    breaks: "`clone_from` of an `Option` of a type whose own `clone_from` is the crate's is `*dest = source.clone()`, which never calls it (#922)",
    file: "src/lower/mir/std_calls/clone_from.rs",
    find: "            if self.forwards_clone_from(ty, span)? {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_generic_result_refused.rs$"],
  },
  {
    name: "generic-clone-from-place-not-given-back",
    breaks: "a generic `clone_from` of a local, given its dictionary's `cloneFrom` in a box of it, never takes back what it wrote: a `None` written over by a `Some` stays `None` (#928)",
    file: "src/lower/mir/std_calls/clone_from.rs",
    find: "                        StmtKind::Assign(to.clone(), Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),\n",
    replace: "                        StmtKind::Expr(Expr::member(Expr::var(name), js::HELD)).at(self.js_span(span)),\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from_generic_std_own.rs$"],
  },
  {
    name: "generic-clone-from-box-made-again",
    breaks: "a generic `clone_from` of what its `&mut T` points at gives its dictionary a box of a copy, `{ value: dest.value }`, not the box it was given (#928)",
    file: "src/lower/mir/std_calls/clone_from.rs",
    find: "                Value::Ref(place) => self.box_of_place(state, place),\n",
    replace: "                Value::Ref(_) => None,\n",
    tests: ["test/lowering-std-unwinds.test.ts", "-t", "generic clone_from"],
  },
];

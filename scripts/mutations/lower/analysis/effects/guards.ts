// Mutations of crates/rust-js-lower/src/analysis/effects/guards.rs, what a held `RefCell` guard says of its cell (#2112 B).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/effects/guards.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "keeps nothing the forgetting rules forget"];
const proofs = ["test/intervals-shadow.test.ts", "-t", "keeps what the rules prove"];

export const mutations: Mutation[] = [
  {
    name: "effects-guards-cover-nothing",
    breaks:
      "a `RefCell` guard held across code covers nothing of its cell, so `f(items[i])` under `take_first`'s `borrow_mut()` forgets the guard's length and `items.remove(i)` is checked again (the lossy direction)",
    file,
    find: "            if !covered.is_empty() {\n",
    replace: "            if false {\n",
    tests: proofs,
  },
];

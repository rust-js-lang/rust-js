// Mutations of crates/rust-js-mir/src/nulled/items.rs (ADR 0093): what a closure
// reads of the thread-local a `with` gives it, `Some` wherever it runs
// (#1933 step 4 (G)).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "present-item-read-of-any-closure",
    breaks: "a closure's read of what its `with` lends it is taken to be `Some` though nothing tested it where the closure is made, `static UNTESTED: fn(i32) -> i32 = |x| MISSING.with(|m| m.unwrap())(x)`, so it calls `undefined` where Rust panics (#1933)",
    file: "crates/rust-js-mir/src/nulled/items.rs",
    find: "            || !self.krate.nulled_params.given_present(closure)\n",
    replace: "",
    tests: ["test/lowering-nullable-present-items.test.ts", "-t", "is unproved"],
  },
];

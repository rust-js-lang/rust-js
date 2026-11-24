// Mutations of crates/rust-js-lower/src/crate_facts.rs, the crate's facts and whether a library's consumer needs each (ADR 1129, amended; #1671 change 4 M4).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "quiet-library-key-counted",
    breaks: "a library's thread-local whose borrows nothing counts is borrowed here as a counted cell's: refused as held while code rust-js can't see runs, or, where the library has a generic impl for `RefMut<'_, T>`, the consumer reads the `value` of the array the library's variable is and throws where Rust prints (#2531)",
    file: "crates/rust-js-lower/src/crate_facts.rs",
    find: "        self.quiet_local(key) || self.quiet_library_keys.contains(&key)\n",
    replace: "        self.quiet_local(key)\n",
    tests: ["test/crates-pairs-1.test.ts", "-t", "thread_local_object_of_a_library_borrowed"],
  },
  {
    name: "crate-fact-carried-by-nothing",
    breaks: "the own nullable parameters each call gives `null` or a value, which a consumer's call reads as `Calling.nulled_params`, is said to be carried across by no fact of the contract",
    file: "crates/rust-js-lower/src/crate_facts.rs",
    find: "    nulled_params => By(&[F::CallingNulledParams]),\n",
    replace: "    nulled_params => By(&[]),\n",
    unitTests: "crate_facts::tests::each_crate_fact_says_whether_a_consumer_needs_it",
  },
];

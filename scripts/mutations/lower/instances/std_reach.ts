// Mutations of crates/rust-js-lower/src/instances/std_reach.rs (ADR 0093):
// the debug build's check that what std's code reaches of the crate's impls
// that take an iterator is what `crate_impl_called_by` answers (#2376).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-reach-unchecked",
    breaks:
      "std's code reaching the crate's impl that takes an iterator by a path `crate_impl_called_by` doesn't answer, a tuple's `collect()`'s parts' `extend`, passes the debug build silently (#2376)",
    file: "crates/rust-js-lower/src/instances/std_reach.rs",
    find: "                    reached.insert((local, called.args));\n",
    replace: "",
    tests: ["test/lowering-generic-iterators.test.ts", "-t", "unanswered"],
  },
  {
    name: "std-reach-unpruned-skips",
    breaks:
      "std's code is followed only while it's given nothing of the crate's, so what reaches the crate's impl is never followed and a path no answer covers passes (#2376)",
    file: "crates/rust-js-lower/src/instances/std_reach.rs",
    find: "        if !mentions_the_crate(callee_args) {\n",
    replace: "        if mentions_the_crate(callee_args) {\n",
    tests: ["test/lowering-generic-iterators.test.ts", "-t", "unanswered"],
  },
  {
    name: "std-reach-said-of-a-refusal",
    breaks:
      "a compile a refusal ends says what it left unanswered too, a panic in place of the refusal that says what to write (#2376)",
    file: "crates/rust-js-lower/src/instances/std_reach.rs",
    find: "    (!errored && !unanswered.is_empty()).then(|| unanswered.join(\"\\n\"))\n",
    replace: "    (!unanswered.is_empty()).then(|| unanswered.join(\"\\n\"))\n",
    unitTests: "instances::std_reach::tests::an_unanswered_reach_is_said_of_a_compile_that_succeeds",
  },
];

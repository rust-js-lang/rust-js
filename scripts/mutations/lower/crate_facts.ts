// Mutations of src/lower/crate_facts.rs, the crate's facts and whether a library's consumer needs each (ADR 1129, amended; #1671 change 4 M4).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "crate-fact-carried-by-nothing",
    breaks: "the own nullable parameters each call gives `null` or a value, which a consumer's call reads as `Calling.nulled_params`, is said to be carried across by no fact of the contract",
    file: "src/lower/crate_facts.rs",
    find: "    nulled_params => By(&[F::CallingNulledParams]),\n",
    replace: "    nulled_params => By(&[]),\n",
    unitTests: "lower::crate_facts::tests::each_crate_fact_says_whether_a_consumer_needs_it",
  },
];

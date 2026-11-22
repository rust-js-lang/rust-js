// Mutations of crates/rust-js-lower/src/discriminants.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "discr-unsigned-wrapped",
    breaks: "a `u128` discriminant past `i128::MAX` wraps negative, so `Last = u128::MAX` orders before `First = 1` and casts to `-1` (#1131)",
    file: "crates/rust-js-lower/src/discriminants.rs",
    find: "            false => Discr::Unsigned(d.val),\n",
    replace: "            false => Discr::Signed(d.val as i128),\n",
    tests: ["test/corpus.test.ts", "-t", "^discriminants_past_32_bits.rs$"],
  },
];

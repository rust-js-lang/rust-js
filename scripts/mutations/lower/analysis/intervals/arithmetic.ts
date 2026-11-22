// Mutations of crates/rust-js-lower/src/analysis/intervals/arithmetic.rs, what an integer
// operation gives by the MIR intervals (ADR 1137).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-quotients-unknown",
    breaks: "a quotient by a constant isn't known, so `n / 2 + n / 4` may wrap",
    file: "crates/rust-js-lower/src/analysis/intervals/arithmetic.rs",
    find: "            BinOp::Div if b.lo > 0 || b.hi < 0 => {\n",
    replace: "            BinOp::Div if false && (b.lo > 0 || b.hi < 0) => {\n",
    tests,
  },
  {
    name: "intervals-remainder-reaches-its-divisor",
    breaks: "a remainder is taken to reach its divisor, so `slots[v % 3]` of three may be past them",
    file: "crates/rust-js-lower/src/analysis/intervals/arithmetic.rs",
    find: "                hi: if a.hi <= 0 { 0 } else { a.hi.min(b.hi - 1) },\n",
    replace: "                hi: if a.hi <= 0 { 0 } else { a.hi.min(b.hi) },\n",
    tests,
  },
  {
    name: "intervals-negative-zero-exact",
    breaks: "a product that may be JS's `-0`, `-2 * 0`, is proved exact, JS's own `*` (ADR 0064)",
    file: "crates/rust-js-lower/src/analysis/intervals/arithmetic.rs",
    find: "        zero(a) && b.lo < 0 || zero(b) && a.lo < 0\n",
    replace: "        false\n",
    tests,
  },
];

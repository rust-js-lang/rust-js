// Mutations of crates/rust-js-lower/src/analysis/nulled_params/present_items.rs (ADR 0093):
// the closures a `with` gives a thread-local a test proved `Some` where
// they're made, which never changes (#1933 step 4 (G)).
import type { Mutation } from "../../../../mutations";

const proved = ["test/lowering-nullable-present-items.test.ts", "-t", "is Some in the closure"];
const unproved = ["test/lowering-nullable-present-items.test.ts", "-t", "is unproved"];

export const mutations: Mutation[] = [
  {
    name: "present-items-undominated",
    breaks: "a test of a thread-local anywhere in a body proves it `Some` where a closure is made on its other edge, `if MISSING.with(Option::is_none) { Some(|x| MISSING.with(|m| m.unwrap())(x)) }`, so the closure calls `undefined` where Rust panics (#1933)",
    file: "crates/rust-js-lower/src/analysis/nulled_params/present_items.rs",
    find: "                && dominators.dominates(edge, block)\n",
    replace: "",
    tests: unproved,
  },
  {
    name: "present-items-is-none-edge-swapped",
    breaks: "`KEY.with(Option::is_none)`'s true edge, where it's `None`, is taken to prove it `Some` (#1933)",
    file: "crates/rust-js-lower/src/analysis/nulled_params/present_items.rs",
    find: "                Some(false) => no,\n",
    replace: "                Some(false) => targets.otherwise(),\n",
    tests: unproved,
  },
  {
    name: "present-items-maker-unasked",
    breaks: "a closure made in a closure that's made where the test held, React DOM's `SCHEDULE_TIMEOUT.with(|s| s.unwrap())` in `localRequestAnimationFrame`, is taken to be made where nothing's proved, so `$unwrap(scheduleTimeout)` (#1933)",
    file: "crates/rust-js-lower/src/analysis/nulled_params/present_items.rs",
    find: "                present.extend(common);\n",
    replace: "                let _ = common;\n",
    tests: proved,
  },
];

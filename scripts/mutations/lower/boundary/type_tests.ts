// Mutations of crates/rust-js-lower/src/boundary/type_tests.rs (ADR 1192, amended).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/type_tests.rs";

export const mutations: Mutation[] = [
  {
    name: "type-test-ignored",
    breaks: "a parameter a body asks `js::is_function` of first is checked, so React's SchedulerMock's `null` callback is refused (#2403)",
    file,
    find: "        if tests.is_empty() {",
    replace: "        if true {",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::is_function of a value of a function's type"],
  },
  {
    name: "type-tested-result-checked",
    breaks: "a binding's result its body type-tests first is checked too, so `if js::is_number(depth) { depth() }` given text throws where it would take its `else` (#2403)",
    file,
    find: "            crate::bindings::is_type_test(self.tcx, callee)\n                && args",
    replace: "            false && crate::bindings::is_type_test(self.tcx, callee)\n                && args",
    tests: ["test/boundary-callbacks.test.ts", "-t", "type-tests first"],
  },
];

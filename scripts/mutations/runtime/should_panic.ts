// Mutations of src/runtime/should_panic.js (ADR 0026).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "should-panic-takes-any-throw",
    breaks: "a `#[should_panic]` test passes when the JS throws a `TypeError`, not only when it panics",
    file: "src/runtime/should_panic.js",
    find: "    if (!(e instanceof Error && e.constructor === Error)) {\n",
    replace: "    if (false) {\n",
    tests: ["test/browser.test.ts", "-t", "fails the way Rust's would"],
  },
];

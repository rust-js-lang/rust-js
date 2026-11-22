// Mutations of crates/rust-js-program/src/prepare/this_handles.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "this-handle-instance-shadowed",
    breaks: "the `const this$ = this;` a function's handles on its `this` read is spelled as it is, not by preparation apart from a nested function's `const this$ = this;` beside it: two `const this$` in one scope, JS's syntax error (ADR 1150)",
    file: "crates/rust-js-program/src/prepare/this_handles.rs",
    find: "    js::this_outside_accessors_of(body, &Name::made_apart(\"this$\"));\n",
    replace: "    js::this_outside_accessors_of(body, &Name::declared(\"this$\".to_string()));\n",
    tests: ["test/js-constructors.test.ts", "-t", "beside a nested function's this\\$ reads it apart"],
  },
];

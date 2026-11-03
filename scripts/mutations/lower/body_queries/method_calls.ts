// Mutations of src/lower/body_queries/method_calls.rs (ADR 0093): a member
// called as its value's method, `function::call(js::method((value, key)),
// Some(value), args)` (rd334, ADR 1025 amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "method-call-other-variable",
    breaks:
      "a key's `js::method` called given another `this`, `function::call(js::method((listener, \"handleEvent\")), Some(other), ())`, is let through, `listener.handleEvent.call(other)`, which calls an object's own `call` where the member is no function (rd334)",
    file: "src/lower/body_queries/method_calls.rs",
    find: "        if variable(thir, value).is_some_and(|of| variable(thir, given.expr) == Some(of)) {\n",
    replace: "        if variable(thir, value).is_some() {\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::method of a key read other than as its value's method"],
  },
  {
    name: "method-call-function-unrecognized",
    breaks:
      "no `function::call` is seen as a method's call: `instance._applyProps(instance, props)` of a key is refused, and `element.focus.call(element, options)` stays (rd334)",
    file: "src/lower/body_queries/method_calls.rs",
    find: "        if !is_function_call(tcx, call) {\n",
    replace: "        if !is_function_call(tcx, call) || true {\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "function::call of js::method whose this is its value"],
  },
];

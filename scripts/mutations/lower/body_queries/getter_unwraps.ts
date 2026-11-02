// Mutations of src/lower/body_queries/getter_unwraps.rs (ADR 0093): a
// binding getter's function unwrapped where it's called (rd519, ADR 1066).
import type { Mutation } from "../../../mutations";

const tests = ["test/js-globals.test.ts", "-t", "a getter's function unwrapped where it's called"];

export const mutations: Mutation[] = [
  {
    name: "getter-unwrapped-callee-unseen",
    breaks: "a getter's function unwrapped where it's called keeps `$unwrap`, `$unwrap(console.createTask)(name)` (rd519)",
    file: "src/lower/body_queries/getter_unwraps.rs",
    find: "        let called = (tcx.trait_of_assoc(def_id)).is_some_and(|t| {\n",
    replace: "        let called = false && (tcx.trait_of_assoc(def_id)).is_some_and(|t| {\n",
    tests,
  },
  {
    name: "getter-unwrapped-receiver-unseen",
    breaks: "a getter's function unwrapped as a method's `this` keeps `$unwrap`, `$unwrap(console.createTask).bind(console, ..)` (rd519)",
    file: "src/lower/body_queries/getter_unwraps.rs",
    find: "        let receiver = bindings::is_binding(tcx, def_id) && bindings::is_method(tcx, def_id);\n",
    replace: "        let receiver = false;\n",
    tests,
  },
  {
    name: "getter-unwrapped-of-any-option",
    breaks:
      "an `unwrap()` of any call's `Option` called is its value, `maker()(1)`, its `None` a `TypeError` where nothing decided so (rd519)",
    file: "src/lower/body_queries/getter_unwraps.rs",
    find: "    (bindings::is_binding(tcx, getter) && matches!(bindings::js_form(tcx, getter), JsForm::Get(_))).then_some(fn_span)\n",
    replace: "    let _ = getter;\n    Some(fn_span)\n",
    tests,
  },
];

// Mutations of src/lower/mir/calls/method_paths.rs (ADR 0093): `js::method`'s
// read of a path from a value (rd181, rd334, ADR 1025).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "optional-call-safe-receiver-unchecked",
    breaks: "a safe optional call accepts an unrelated native receiver without an unsafe promise",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "if unchecked {",
    replace: "if true {",
    tests: ["test/js-optional-call.test.ts", "-t", "a safe optional native call"],
  },
  {
    name: "optional-call-native-receiver-checked",
    breaks: "the documented optional Node member is rejected as though it must be declared on Node",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "PathEnd::UncheckedMethod",
    replace: "PathEnd::Checked",
    tests: ["test/js-optional-call.test.ts", "-t", "documented Node receiver"],
  },
  {
    name: "method-path-step-unread",
    breaks:
      "`js::method((element, HTMLElement::focus))` is `element`, its method's step never read (rd181)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "        let method = path.split('.').fold(value, Expr::member);\n",
    replace: "        let method = value;\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::method"],
  },
  {
    name: "keyed-method-anywhere",
    breaks:
      "a key's `js::method` held as a function, `let f = js::method((listener, \"handleEvent\")); f()`, compiles, a `&dyn Fn` of what may be no function (rd334)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "        let called = |at| self.frame.facts.called_methods.contains(&at);\n",
    replace: "        let called = |_| true;\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "js::method of a key read other than as its value's method"],
  },
  {
    name: "keyed-method-unmarked",
    breaks:
      "a key's `js::method` isn't known as one, so react-art's `let fill = js::get(props, \"fill\");` before its `applyFill` call stays a `const`, not `props.fill.applyFill(instance)` (rd226b)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "                    .keyed_method()\n",
    replace: "",
    tests: ["test/lowering-is-function.test.ts", "-t", "keyed method call is read there"],
  },
  {
    name: "called-method-by-a-shared-span",
    breaks:
      "a `js::method` of a key is read as its value's method where any THIR call at its span is, of two a macro's repetition writes there, one given another `this` (#1671 change 3 M5)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "        if !crate::lower::body_queries::each_call_at(self.frame.thir, span, called) {\n",
    replace: "        if !self.frame.thir.exprs.iter_enumerated().any(|(at, e)| matches!(e.kind, rustc_middle::thir::ExprKind::Call { fn_span, .. } if fn_span == span) && called(at)) {\n",
    tests: ["test/lowering-is-function.test.ts", "-t", "a macro's repetition writes is asked of its own call"],
  },
  {
    name: "optional-call-arguments-in-place",
    breaks: "an optional call's argument that runs code is written in it, which JS's `?.` wouldn't run where the method isn't there, where Rust runs it either way (#1859)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "                true => self.spill(Role::Arg, item, out),\n",
    replace: "                true => item,\n",
    tests: ["test/js-optional-call.test.ts", "-t", "upstream's"],
  },
  {
    name: "optional-call-nested-unboxed",
    breaks: "an optional call of a method whose `None` is `null`, `closest`'s, gives its `null` unboxed, which reads as the outer `None`: a hop whose `closest` finds none is taken for one without `closest` (#1859)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "            true => Expr::call(Expr::helper(\"$someDefined\"), vec![call]),\n",
    replace: "            true => call,\n",
    tests: ["test/js-optional-call.test.ts", "-t", "upstream's"],
  },
  {
    name: "optional-call-of-undefined-results",
    breaks: "an optional call of a method that may give `undefined` compiles, where `?.`'s `undefined` couldn't tell it from a method that isn't there (#1859)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "        if boxed && !null_alone {\n",
    replace: "        if false {\n",
    tests: ["test/js-optional-call.test.ts", "-t", "is refused"],
  },
  {
    name: "optional-call-of-a-getter",
    breaks: "an optional call of a getter binding calls what it reads, `hop.tagName?.()` of a string (#1859)",
    file: "src/lower/mir/calls/method_paths.rs",
    find: "        if !matches!(bindings::js_form(tcx, method), bindings::JsForm::Call(_)) || !bindings::is_method(tcx, method) {\n",
    replace: "        if false {\n",
    tests: ["test/js-optional-call.test.ts", "-t", "is refused"],
  },
];

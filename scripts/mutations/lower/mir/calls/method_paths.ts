// Mutations of src/lower/mir/calls/method_paths.rs (ADR 0093): `js::method`'s
// read of a path from a value (rd181, rd334, ADR 1025).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
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
    find: "        if !self.frame.facts.called_methods.contains(&span) {\n",
    replace: "        if false {\n",
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
];

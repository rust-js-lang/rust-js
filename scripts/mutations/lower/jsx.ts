// Mutations of src/lower/jsx.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "binding-component-as-value",
    breaks: "a JS module's component is lowered as a value, an arrow, which isn't a JSX tag",
    file: "src/lower/jsx.rs",
    find: "                let tag = match self.binding_component(component) {\n",
    replace: "                let tag = match self.binding_component(component).filter(|_| false) {\n",
    tests: ["test/jsx.test.ts", "-t", "a binding is a value"],
  },
  {
    name: "undefined-children-written",
    breaks: "\`<Frame {..Default::default()} />\` is \`<Frame>{undefined}</Frame>\`",
    file: "src/lower/jsx.rs",
    find: "                Prop::Field(name, value) if name == \"children\" && matches!(value.kind, js::ExprKind::Undefined) => {}",
    replace: "                Prop::Field(name, value) if name == \"children\" && false && matches!(value.kind, js::ExprKind::Undefined) => {}",
    tests: ["test/jsx.test.ts", "-t", "named props and the rest from a base"],
  },
  {
    name: "spilled-attribute-copied",
    breaks: "an attribute read into a `const` is copied to another, `const className$1 = className`, before a child",
    file: "src/lower/jsx.rs",
    find: "                    Prop::Spread(value) => (\"props\", value),\n                };\n                if !self.reads_alike(value, out) {",
    replace: "                    Prop::Spread(value) => (\"props\", value),\n                };\n                if !value.is_constant() {",
    tests: ["test/jsx.test.ts", "-t", "attribute before a child once"],
  },
  {
    name: "rest-field-not-spread",
    breaks: "a `Rest` a component passes on is a `rest` prop, not `{...rest}`",
    file: "src/lower/jsx.rs",
    find: "                        attrs.push(Prop::Spread(value));",
    replace: "                        attrs.push(Prop::Field(name, value));",
    tests: ["test/jsx.test.ts", "-t", "doesn't name as ...rest"],
  },
  {
    name: "rest-spread-refused",
    breaks: "`<a {...rest}>` of a `Rest` is an error, as a spread of what isn't a struct",
    file: "src/lower/jsx.rs",
    find: "                && !super::bindings::is_rest(self.tcx, self.thir[value].ty)\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "doesn't name as ...rest"],
  },
  {
    name: "passed-handler-wrapped",
    breaks: "an optional handler passed on to an element is wrapped, `(e) => { if (onClick != null) { onClick(e); } }`",
    file: "src/lower/jsx.rs",
    find: "    (given && called && handler != param).then(|| (**tested).clone())",
    replace: "    (false && given && called && handler != param).then(|| (**tested).clone())",
    tests: ["test/jsx.test.ts", "-t", "optional event handler on as it is"],
  },
];

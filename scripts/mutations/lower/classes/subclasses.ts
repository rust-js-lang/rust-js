// Mutations of src/lower/classes/subclasses.rs (ADR 0093): a class that
// extends a binding's class, an ES class (rd331, ADR 1041).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "subclass-impl-of-no-class",
    breaks: "a class's impl of react's `Component` is no class's: its methods an object's, `Counter.render(c)`, no `class Counter extends Component`",
    file: "src/lower/classes/subclasses.rs",
    find: "    base_class(tcx, tcx.impl_trait_id(imp)).is_some().then_some(class)\n",
    replace: "    base_class(tcx, tcx.impl_trait_id(imp)).is_none().then_some(class)\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "subclass-binding-method-written",
    breaks: "a class component may write `fn props(&self)`, its prototype's `props()` hidden by React's `this.props`",
    file: "src/lower/classes/subclasses.rs",
    find: "        if crate::lower::bindings::is_binding(tcx, item) {\n",
    replace: "        if false {\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "what a class that extends one is refused"
    ]
  },
  {
    name: "subclass-method-named-as-field-allowed",
    breaks: "a class component's `render` method beside a `render` field, which `this.render` hides",
    file: "src/lower/classes/subclasses.rs",
    find: "        } else if fields.contains(&named) {\n",
    replace: "        } else if false {\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "what a class that extends one is refused"
    ]
  },
  {
    name: "subclass-impl-anywhere",
    breaks: "a class component whose constructor is in one module may implement react's `Component` in another, its methods placed where its class isn't",
    file: "src/lower/classes/subclasses.rs",
    find: "    if home(tcx, class) != Some(module) {\n",
    replace: "    if false {\n",
    tests: ["test/class-components.test.ts", "-t", "the module its impl is"],
  },
  {
    name: "subclass-extends-nothing",
    breaks: "a class component's class extends nothing, printed as a constructor function whose `super(props)` is a SyntaxError",
    file: "src/lower/classes/subclasses.rs",
    find: "        extends: (class_of(tcx, member).and_then(|adt| superclass(tcx, adt)))\n",
    replace: "        extends: (class_of(tcx, member).and_then(|adt| superclass(tcx, adt)).filter(|_| false))\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
];

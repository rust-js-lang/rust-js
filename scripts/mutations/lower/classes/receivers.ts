// Mutations of src/lower/classes/receivers.rs (ADR 0093): what a function's
// JS takes its `self` as, its body's and a JS caller's (ADR 1091).
import type { Mutation } from "../../../mutations";

const file = "src/lower/classes/receivers.rs";

export const mutations: Mutation[] = [
  {
    name: "object-self-a-parameter",
    breaks:
      "an object's method's `self` is a parameter, `function containsNode(scopeInstance, fiber)`, where its callers give it as `this`, `scope.containsNode(fiber)` (ADR 0943)",
    file,
    find: "        Some(Member::Method | Member::Getter | Member::Own) => Receiver::This,\n",
    replace: "        Some(Member::Method | Member::Getter) => Receiver::This,\n",
    tests: ["test/classes.test.ts", "-t", "an object's methods are its literal's own"],
  },
  {
    name: "prototype-method-declared-self",
    breaks:
      "a trait of classes' method is declared of its `self`, `describe(self: Self): string`, where each class's prototype takes it as `this` (ADR 1035, 1091)",
    file,
    find: "    let prototype = (tcx.trait_of_assoc(function)).is_some_and(|of| is_class_trait(tcx, of));\n",
    replace: "    let prototype = (tcx.trait_of_assoc(function)).is_some_and(|of| is_class_trait(tcx, of)) && false;\n",
    tests: ["test/classes-traits.test.ts", "-t", "a trait of classes' methods are each class's prototype's"],
  },
  {
    name: "bound-method-declared-self",
    breaks:
      "a binding's method a struct's interface declares is declared of its `self`, `isPersistent(self: Event): boolean`, where its object is called by it (rd144, ADR 1091)",
    file,
    find: "    let bound = crate::lower::bindings::is_binding(tcx, function)\n",
    replace: "    let bound = false && crate::lower::bindings::is_binding(tcx, function)\n",
    tests: ["test/declarations-bound-methods.test.ts"],
  },
];

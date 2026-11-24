// Mutations of crates/rust-js-lower/src/classes/defaults.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-defaults-unshared",
    breaks:
      "each class takes its own copy of a trait's default that prints alike, not `B.prototype.render = A.prototype.render = function` (rd283)",
    file: "crates/rust-js-lower/src/classes/defaults.rs",
    find: "                .any(|f| f.name == function.name && js::scopes::alpha_eq_functions(f, &function))\n",
    replace: "                .any(|_| false)\n",
    tests: ["test/classes-traits.test.ts", "-t", "one function of constructor functions"],
  },
  {
    name: "class-defaults-shared-unlike",
    breaks:
      "a trait default a class's own impl writes otherwise is shared with another's, so `unmount` is one function (rd283)",
    file: "crates/rust-js-lower/src/classes/defaults.rs",
    find: "                .any(|f| f.name == function.name && js::scopes::alpha_eq_functions(f, &function))\n",
    replace: "                .any(|f| f.name == function.name)\n",
    tests: ["test/classes-traits.test.ts", "-t", "one function of constructor functions"],
    snapshots: true,
  },
  {
    name: "class-default-self-unread",
    breaks:
      "a trait default on a class's prototype reads its `self` as a variable, not `this` (rd283)",
    file: "crates/rust-js-lower/src/classes/defaults.rs",
    find: "            && js::read_as_this_of(&mut body, &receiver)\n",
    replace: "            && false\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-defaults-of-own-entries",
    breaks:
      "an impl's own method of a trait of classes whose dictionary entry is an arrow, a `&mut self` method's that boxes its `self`, is taken for a default too, so the class is refused for holding it twice (#1074)",
    file: "crates/rust-js-lower/src/classes/defaults.rs",
    find: "        if own.contains(&name) {\n            continue;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_method_before_its_field.rs$"],
  },
  {
    name: "es-class-default-shared",
    breaks: "an ES class's provided method is another class's function, assigned to it outside its body, which an ES class has none of: ReactDOMHydrationRoot has no `render` (#2495)",
    file: "crates/rust-js-lower/src/classes/defaults.rs",
    find: "        !owner_es\n            && !es(n)\n",
    replace: "        (owner_es || !owner_es)\n            && (es(n) || !es(n))\n",
    tests: ["test/classes-traits.test.ts", "-t", "a trait of classes' methods are each class's prototype's"],
  },
];

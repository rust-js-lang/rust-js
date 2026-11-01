// Mutations of src/lower/classes/defaults.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-defaults-unshared",
    breaks:
      "each class takes its own copy of a trait's default that prints alike, not `B.prototype.render = A.prototype.render = function` (rd283)",
    file: "src/lower/classes/defaults.rs",
    find: "                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)\n",
    replace: "                .any(|_| false)\n",
    tests: ["test/classes-traits.test.ts"],
  },
  {
    name: "class-defaults-shared-unlike",
    breaks:
      "a trait default a class's own impl writes otherwise is shared with another's, so `unmount` is one function (rd283)",
    file: "src/lower/classes/defaults.rs",
    find: "                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)\n",
    replace: "                .any(|f| f.name == function.name)\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_methods.rs$"],
    snapshots: true,
  },
  {
    name: "class-default-self-unread",
    breaks:
      "a trait default on a class's prototype reads its `self` as a variable, not `this` (rd283)",
    file: "src/lower/classes/defaults.rs",
    find: "            && js::read_as_this(&mut body, &receiver)\n",
    replace: "            && false\n",
    tests: ["test/classes-traits.test.ts"],
  },
];

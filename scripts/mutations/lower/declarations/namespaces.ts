// Mutations of src/lower/declarations/namespaces.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-methods-undeclared",
    breaks:
      "a namespace's methods aren't declared as its module's functions (ADR 1037)",
    file: "src/lower/declarations/namespaces.rs",
    find: "                items.push(documented(function, tcx, method));\n",
    replace: "                let _ = function;\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a namespace's methods are its module's functions",
    ],
  },
];

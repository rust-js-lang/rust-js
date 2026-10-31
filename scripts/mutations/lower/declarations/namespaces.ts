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
  {
    name: "forwarded-method-declared",
    breaks:
      "a namespace's method that's the function it passes its parameters on to is declared a function of its module beside the re-export of its name (rd292b)",
    file: "src/lower/declarations/namespaces.rs",
    find: "                if !tcx.visibility(method).is_public() || forwarded.contains_key(&method) {\n",
    replace: "                if !tcx.visibility(method).is_public() {\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
];

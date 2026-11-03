// Mutations of src/lower/declarations/namespaces.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-constants-declared-in-part",
    breaks: "an object of constants and methods is declared with its constants alone (#498, #507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "        if constants.is_empty() || methods {\n",
    replace: "        if constants.is_empty() {\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-constants-writable",
    breaks: "an object's constants are declared writable (#498)",
    file: "src/lower/declarations/namespaces.rs",
    find: '"optional": false, "readonly": true, "type": ty',
    replace: '"optional": false, "readonly": false, "type": ty',
    tests: ["test/impl-constants.test.ts"],
  },
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

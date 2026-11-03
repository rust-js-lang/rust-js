// Mutations of src/lower/declarations/namespaces.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-object-methods-undeclared",
    breaks: "a type's object's methods aren't declared, only its constants (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "            members.push(documented(declared, tcx, method));\n",
    replace: "            let _ = declared;\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-private-methods",
    breaks: "a type's object's private methods are declared, which no other crate can call (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "tcx.def_kind(item) == DefKind::AssocFn && tcx.visibility(item).is_public());\n",
    replace: "tcx.def_kind(item) == DefKind::AssocFn);\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-impl-generics-dropped",
    breaks: "a generic impl's methods are declared without its type parameters, `get(wrapped: Wrapped<T>): T` of no `T` (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "            let mut type_parameters = self.generics(imp);\n",
    replace: "            let mut type_parameters = Vec::new();\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-impl-erasures-dropped",
    breaks: "a generic impl's parameter of a TypeScript type, `C: ReactNode`, is declared `C` in its methods, which no type parameter names (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "            let mut erased = self.erased(imp);\n",
    replace: "            let mut erased = std::collections::HashMap::new();\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-of-a-class",
    breaks: "a class's methods and constants are declared an object too, which its prototype holds (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "                .filter(|_| crate::lower::classes::member(tcx, *item).is_none())\n",
    replace: "",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-of-another-module",
    breaks: "an impl in another module than its type's is declared an object in its type's module, where its JS has none (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "                .filter(|info| info.module == self.module && info.owner.is_some())\n",
    replace: "                .filter(|info| info.owner.is_some())\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-object-of-a-foreign-impl-undeclared",
    breaks: "an impl of another module's type isn't declared where its object is (#507)",
    file: "src/lower/declarations/namespaces.rs",
    find: "        (!own && tcx.visibility(adt.did()).is_public()).then_some(adt.did())\n",
    replace: "        (false && !own && tcx.visibility(adt.did()).is_public()).then_some(adt.did())\n",
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

// Mutations of src/lower/declarations/imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reexport-undeclared",
    breaks:
      "a module's `pub use` isn't in its `.d.ts`, and TypeScript that imports it finds nothing",
    file: "src/lower/declarations/imports.rs",
    find: "    for reexport in reexports {",
    replace: "    for reexport in reexports.iter().take(0) {",
    tests: ["test/declarations.test.ts", "-t", "re-export what a module"],
  },
  {
    name: "type-import-default-beside-names",
    breaks:
      "a module's declarations import another's default class and one of its other types as one, `import type Complex, { Parts } from \"./complex.js\";`, which TypeScript refuses, so the build fails (#373)",
    file: "src/lower/declarations/imports.rs",
    find: "        Some(default) if !names.is_empty() => vec![import(Vec::new(), Some(default)), import(names, None)],\n",
    replace: "",
    tests: [
      "test/classes.test.ts",
      "-t",
      "a class is its module's default export",
    ],
  },
  {
    name: "default-class-type-imported-by-name",
    breaks:
      "a module's declarations import another's default class `import type { Complex }`",
    file: "src/lower/declarations/imports.rs",
    find: '        let default = crate::lower::default_exports::exported_as(self.tcx, did, &name) == "default";\n',
    replace: "        let default = false;\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "a class is its module's default export",
    ],
  },
  {
    name: "type-reexports-ungrouped",
    breaks: "a `pub use` of two types of one module is two `export type` statements, not one (rd642)",
    file: "src/lower/declarations/imports.rs",
    find: "if was.lo() == at.lo() && *was_from == from =>",
    replace: "if false && *was_from == from =>",
    tests: ["test/declarations-libraries.test.ts", "-t", "type-only re-export"],
  },
  {
    name: "type-reexports-of-values",
    breaks: "a type's re-export is a value's, `export { Kind } from`, which names no value of the module (rd642)",
    file: "src/lower/declarations/imports.rs",
    find: "        let mut declaration = json!({ \"kind\": \"export\", \"names\": names, \"typeOnly\": true });\n",
    replace: "        let mut declaration = json!({ \"kind\": \"export\", \"names\": names });\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "type-only re-export"],
  },
  {
    name: "package-default-type-imported-as-a-name",
    breaks: "a JS module's default type is imported among its names, `import type { ModuleDependency } from ..`, which the module doesn't export (rd678)",
    file: "src/lower/declarations/imports.rs",
    find: "        declarations.extend(type_imports(json!({ \"from\": from }), names, default));\n",
    replace: "        declarations.extend(type_imports(json!({ \"from\": from }), [names, default.into_iter().collect()].concat(), None));\n",
    tests: ["test/binding-subclasses.test.ts", "-t", "its module's default is imported as its default"],
  },
];

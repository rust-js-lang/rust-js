// Mutations of src/lower/declarations/imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reexport-undeclared",
    breaks: "a module's `pub use` isn't in its `.d.ts`, and TypeScript that imports it finds nothing",
    file: "src/lower/declarations/imports.rs",
    find: "    for reexport in reexports {",
    replace: "    for reexport in reexports.iter().take(0) {",
    tests: ["test/declarations.test.ts", "-t", "re-export what a module"],
  },
  {
    name: "type-import-default-beside-names",
    breaks: "a module's declarations import another's default class and one of its other types as one, `import type Complex, { Parts } from \"./complex.js\";`, which TypeScript refuses, so the build fails (#373)",
    file: "src/lower/declarations/imports.rs",
    find: "        Some(default) if !names.is_empty() => vec![import(Vec::new(), Some(default)), import(names, None)],\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "a class is its module's default export"],
  },
];

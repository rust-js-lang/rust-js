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
];

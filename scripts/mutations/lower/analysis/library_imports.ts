// Mutations of crates/rust-js-lower/src/analysis/library_imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-default-named-after-file",
    breaks:
      "a library's module's default is imported named after the module's file, `import version`, not its item, `import VERSION`",
    file: "crates/rust-js-lower/src/analysis/library_imports.rs",
    find: "        uses.named.insert(export, imported.name.clone());\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "default export is imported as its default",
    ],
  },
  {
    name: "reexported-imports-unnamed",
    breaks:
      "an import through a library's re-exporting module is named nowhere before lowering, which panics rustc reading it (rd704)",
    file: "crates/rust-js-lower/src/analysis/library_imports.rs",
    find: "    for export in foreign.all_reexported() {\n",
    replace: "    for export in foreign.all_reexported().filter(|_| false) {\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a library's module that re-exports its item imports it from there"],
  },
];

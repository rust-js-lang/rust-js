// Mutations of crates/rust-js-lower/src/analysis/library_imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-default-named-after-file",
    breaks:
      "a library's module's default is imported named after the module's file, `import version`, not its item, `import VERSION`",
    file: "crates/rust-js-lower/src/analysis/library_imports.rs",
    find: "            uses.named.insert(export, imported.name.clone());\n        }\n        // A split function's body",
    replace: "        }\n        // A split function's body",
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
  {
    name: "split-body-named-apart",
    breaks: "a library's split function's body is imported under a name of its own, `quadruple$`, not its entry's, so a local of the export's name takes the entry's and another library's body is named apart, `createRequest$$1` (#2403, ADR 1193)",
    file: "crates/rust-js-lower/src/analysis/library_imports.rs",
    find: "            uses.named.insert(export, imported.name.clone());\n        }\n    }\n",
    replace: "            uses.named.insert(export, unchecked.clone());\n        }\n    }\n",
    tests: ["test/boundary-split.test.ts", "-t", "a consumer's renaming use of a library's split function"],
  },
];

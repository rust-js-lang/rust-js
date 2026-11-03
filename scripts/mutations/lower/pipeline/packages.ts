// Mutations of src/lower/pipeline/packages.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-import-unread",
    breaks: '`js::import!("./app.css");` imports nothing',
    file: "src/lower/pipeline/packages.rs",
    find: '    for attr in super::super::bindings::marks(tcx, module, "import") {\n',
    replace:
      '    for attr in super::super::bindings::marks(tcx, module, "none") {\n',
    tests: ["test/modules.test.ts", "-t", "js::import"],
  },
  {
    name: "unread-library-item-imported",
    breaks:
      "a module imports a library's item it doesn't read, a numbered enum's variant whose test an exhaustive `match` needn't write, `import { Pending, Resolved, Uninitialized }`",
    file: "src/lower/pipeline/packages.rs",
    find: "            read.contains(local) || (analyzed(&package.from, export) && !only_forwarded(local))\n",
    replace: "            true\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a library's repr enum's variant is imported by its name",
    ],
  },
  {
    name: "forwarded-imports-kept",
    breaks:
      "what only a function re-exported in its place read is still imported, `import * as Server` of nothing (rd575)",
    file: "src/lower/pipeline/packages.rs",
    find: "    let only_forwarded = |local: &str| forwarded.contains(local) && !read.contains(local);\n",
    replace: "    let only_forwarded = |_: &str| false;\n",
    tests: ["test/declarations-libraries.test.ts", "test/lowering-forwarding.test.ts", "-t", "is its re-export"],
  },
  {
    name: "reimport-left-in-package",
    breaks:
      "a library's item imported from the crate's module that re-exports it is imported from the library's file too, `import { createState, other }` (rd650)",
    file: "src/lower/pipeline/packages.rs",
    find: ".map(|at| package.named.remove(at).1),\n",
    replace: ".map(|at| package.named[at].1.clone()),\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a module"],
  },
];

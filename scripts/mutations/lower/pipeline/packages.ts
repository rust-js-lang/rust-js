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
    name: "js-import-comments-lost",
    breaks:
      "the comments above a `js::import!` are lost, \"// Imports are resolved statically ..\" of React DOM's ReactDOMServerExternalRuntime (rd666)",
    file: "src/lower/pipeline/packages.rs",
    find: "                span: written(Some(attr.span())),\n",
    replace: "                span: js::Span::NONE,\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "comments above a js::import"],
  },
  {
    name: "js-import-placed-where-written",
    breaks:
      "an import for its effect is placed where its `js::import!` is written, after the code before it, not where ADR 1063 orders it (rd666)",
    file: "src/lower/pipeline/packages.rs",
    find: "                span: written(Some(attr.span())),\n                named_at: js::Span::NONE,\n",
    replace: "                span: written(Some(attr.span())),\n                named_at: written(Some(attr.span())),\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "comments above a js::import"],
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
    find: ".map(|at| package.named.remove(at).1)\n",
    replace: ".map(|at| package.named[at].1.clone())\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a module"],
  },
  {
    name: "package-uses-one-import",
    breaks:
      "two `use`s of one JS module are one import at the first, `import { create, destroy, update } from \"./host.js\"` (rd649)",
    file: "src/lower/pipeline/packages.rs",
    find: "        let key = (from.as_str(), at.map(|at| (at.lo(), at.hi())));\n",
    replace: "        let key = (from.as_str(), first.map(|at| (at.lo(), at.hi())));\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "two uses of one module"],
  },
  {
    name: "package-later-use-ordered-first",
    breaks:
      "a JS module's later `use`'s import is ordered where its first is, so `import { destroy } from \"./host.js\"` prints at the top (rd649)",
    file: "src/lower/pipeline/packages.rs",
    find: "            named_at: match at == first {\n",
    replace: "            named_at: match true {\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "two uses of one module"],
  },
];

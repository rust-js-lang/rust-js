// Mutations of src/to_oxc/header.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "named-imports-unwritten",
    breaks: "a module's named imports of a JS module, `import { readFileSync } from \"fs\"`, are left out, and what they name is unbound",
    file: "src/to_oxc/header.rs",
    find: "        if !clause.is_empty() {",
    replace: "        if !clause.is_empty() && package.default.is_some() {",
    tests: ["test/node-bindings.test.ts", "-t", "node's modules are imported, and its globals called"],
  },
  {
    name: "imports-unsorted",
    breaks: "imports stay in the order they were found, `./local` before `@scope/alpha`",
    file: "src/to_oxc/header.rs",
    find: "    named.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "imports are packages"],
  },
  {
    name: "imports-relative-mixed",
    breaks: "relative modules sort among packages, `./helper.js` before `@scope/alpha`",
    file: "src/to_oxc/header.rs",
    find: "    named.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));",
    replace: "    named.sort_by(|a, b| a.1.cmp(b.1));",
    tests: ["test/modules.test.ts", "-t", "imports are packages"],
  },
  {
    name: "imports-effects-first",
    breaks: "what a module imports for its effect comes first, its CSS before what it styles",
    file: "src/to_oxc/header.rs",
    find: "    let lines: Vec<String> = named.chain(effects).chain(runtime).collect();\n",
    replace: "    let lines: Vec<String> = named.chain(runtime).chain(effects).collect();\n",
    tests: ["test/modules.test.ts", "-t", "imports are packages"],
  },
  {
    name: "default-import-braced",
    breaks: "a default import is printed as a named one, `import { default as Label }`",
    file: "src/to_oxc/header.rs",
    find: "            (Some(default), true) => default.clone(),",
    replace: "            (Some(default), true) => format!(\"{{ default as {default} }}\"),",
    tests: ["test/declarations.test.ts","-t","default export is imported as one"],
  },
  {
    name: "use-comment-twice",
    breaks: "of two imports one `use` makes, its comment is above both",
    file: "src/to_oxc/header.rs",
    find: "            .filter(|_| !span.is_none() && said.insert(span.lo))\n",
    replace: "            .filter(|_| !span.is_none())\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "above a use"],
  },
  {
    name: "namespace-import-unwritten",
    breaks: "a module read through its namespace, `Plugin.extractEvents(..)`, imports none, so `Plugin` is undefined (rd114, ADR 1018)",
    file: "src/to_oxc/header.rs",
    find: "        named.extend(namespace);\n    }\n",
    replace: "    }\n",
    tests: ["test/corpus.test.ts", "-t", "^module_path_namespace.rs$"],
  },
];

// Mutations of crates/rust-js-lower/src/boundary/split.rs (ADR 1193).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/split.rs";

export const mutations: Mutation[] = [
  {
    name: "program-split-uncalled",
    breaks: "a program splits a function only JS calls, a wrapper nobody's Rust calls (#2403)",
    file,
    find: "                    && (self.krate.library || called.contains(&id))",
    replace: "                    && true",
    tests: ["test/boundary-split.test.ts", "-t", "a program's function only JS calls"],
  },
  {
    name: "body-keeps-its-checks",
    breaks: "a split function's body keeps the checks its entry makes, so Rust's calls pay them again (#2403)",
    file,
    find: "    body.body.drain(..checks);\n",
    replace: "",
    tests: ["test/boundary-split.test.ts", "-t", "its entry, which checks, and its body"],
  },
  {
    name: "library-body-unexported",
    breaks: "a library's split function's body isn't exported, so its consumer's import of it is of nothing (#2403)",
    file,
    find: "(library && export) || (called_elsewhere.iter())",
    replace: "(called_elsewhere.iter())",
    tests: ["test/boundary-split.test.ts", "-t", "its entry, which checks, and its body"],
  },
  {
    name: "library-body-uncalled",
    breaks: "a consumer's direct call of a library's split function calls its entry, paying its checks (#2403)",
    file,
    find: "        (imported_from == from && entry != \"default\").then_some((from, body))",
    replace: "        None::<(String, String)>.filter(|_| imported_from == from && entry != \"default\" && !body.is_empty())",
    tests: ["test/boundary-split.test.ts", "-t", "a consumer's direct call"],
  },
  {
    name: "library-body-past-a-reexport",
    breaks: "a consumer naming a library's split function through its package root imports the body by the library's inner file, `../../kids/src/inner.js`, past what its package exports (#2403, ADR 1193)",
    file,
    find: "        (imported_from == from && entry != \"default\").then_some((from, body))",
    replace: "        (entry != \"default\" || imported_from.is_empty()).then_some((from, body))",
    tests: ["test/libraries.test.ts", "-t", "an item a package's entry re-exports through its module's re-export"],
  },
  {
    name: "library-default-body-by-name",
    breaks: "a library's split function that's its file's default is called by its body's name, `enqueueTask$`, not imported as the module it is (#2403, ADR 1193)",
    file,
    find: "        (imported_from == from && entry != \"default\").then_some((from, body))",
    replace: "        (imported_from == from || entry.is_empty()).then_some((from, body))",
    tests: ["test/cross-crate-items.test.ts", "-t", "a library's item is imported by the name a use renames it to"],
  },
  {
    name: "body-import-unrenamed",
    breaks: "a body imported where the module's `use` renames its entry, `close as close_flight`, is imported by the body's name, `close$ as close`, past the `use` (#2403, ADR 1193)",
    file,
    find: "            let local = crate::analysis::renamed_in(tcx, from)\n                .swap_remove(&id)\n                .unwrap_or_else(|| name.clone());\n",
    replace: "            let local = name.clone();\n",
    tests: ["test/boundary-split.test.ts", "-t", "through a renaming use"],
  },
  {
    name: "reexported-call-past-its-path",
    breaks: "a call of the crate's split function through another module that re-exports it, `plugins::dispatch()`, calls its body in its own module, past the path Rust names, so `Plugins` isn't imported (#2403, rd137)",
    file,
    find: "        if symbol.export == js::NAMESPACE && *symbol != crate::module_symbol(target, js::NAMESPACE) {\n",
    replace: "        if false && symbol.export == js::NAMESPACE && *symbol != crate::module_symbol(target, js::NAMESPACE) {\n",
    tests: ["test/boundary-split.test.ts", "-t", "through a renaming use"],
  },
  {
    name: "namespace-method-unsplit",
    breaks: "a namespace's split method has no body beside it, so its entry's `Perms.fromBitsRetain$(bits)` is no function (#2403 lever (3))",
    file,
    find: "            namespace.methods.insert(at + 1, body);\n",
    replace: "            drop(body);\n",
    tests: ["test/boundary-reach.test.ts", "-t", "a namespace's method JS may call splits"],
  },
  {
    name: "library-namespace-body-imported",
    breaks: "a library's namespace method's body is imported as its file's own export, `import { new$ }`, which it isn't: a member of its object (#2403)",
    file,
    find: "        if self.krate.foreign.item(def_id)?.member.is_some() {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/cross-crate-private-state.test.ts"],
  },
];

// Mutations of src/output.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "should-panic-takes-any-throw",
    breaks: "a `#[should_panic]` test passes when the JS throws a `TypeError`, not only when it panics",
    file: "src/output.rs",
    find: "    if (!(e instanceof Error && e.constructor === Error)) {\n",
    replace: "    if (false) {\n",
    tests: ["test/browser.test.ts", "-t", "fails the way Rust's would"],
  },
  {
    name: "library-metadata-unplanned",
    breaks: "a library's metadata is published where it's asked for, unchecked, over a source of the crate",
    file: "src/output.rs",
    find: "                planned.push(path.clone());\n",
    replace: "                drop(path.clone());\n",
    tests: ["test/crates.test.ts", "-t", "a source of the crate"],
  },
  {
    name: "test-result-ignored",
    breaks: "a `#[test]` returning an `Err` passes",
    file: "src/output.rs",
    find: '                None if test.returns_result => format!("() => testResult({f}())"),\n',
    replace: "",
    tests: ["test/browser.test.ts", "-t", "fails the way"],
  },
  {
    name: "runtime-import-missing",
    breaks: "a module compiled against @rust-js/runtime calls its helpers, and neither defines nor imports them",
    file: "src/output.rs",
    find: "            js_module.helpers = crate::runtime::imported_helpers(&helper_sources, &js_module.read_vars());\n",
    replace: "",
    tests: ["test/runtime-package.test.ts"],
  },
  {
    name: "declarations-setting-ignored",
    breaks: "a crate's `declarations = true` writes no `.d.ts`, and TypeScript types its modules from their JS",
    file: "src/output.rs",
    find: "            if self.settings.declarations\n                && let Some(declarations) = &module.declarations",
    replace: "            if false\n                && let Some(declarations) = &module.declarations",
    tests: ["test/declarations.test.ts"],
  },
];

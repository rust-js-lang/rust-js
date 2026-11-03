// Mutations of src/lower/analysis/tests.rs (ADR 1076).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "module-of-tests-own-file",
    breaks: "a module's one module of tests, `#[cfg(test)] mod tests`, is a file of its own, `shapes/tests.js`, not its parent's test file, `shapes.test.js`",
    file: "src/lower/analysis/tests.rs",
    find: "        .filter(|modules| modules.len() == 1)\n",
    replace: "        .filter(|modules| modules.len() == 0)\n",
    tests: ["test/test-files.test.ts", "-t", "one module of tests is its test file"],
  },
  {
    name: "several-modules-of-tests-one-file",
    breaks: "a module's several modules of tests are each written as its one test file, which the build refuses as a collision",
    file: "src/lower/analysis/tests.rs",
    find: "        .filter(|modules| modules.len() == 1)\n",
    replace: "        .filter(|modules| !modules.is_empty())\n",
    tests: ["test/test-files.test.ts", "-t", "several modules of tests"],
  },
  {
    name: "module-of-tests-any-cfg",
    breaks: "a module of `#[test]`s not compiled for tests alone is taken for a module of tests, its parent's test file",
    file: "src/lower/analysis/tests.rs",
    find: "        CfgEntry::NameValue { name, value: None, .. } => *name == sym::test,\n",
    replace: "        CfgEntry::NameValue { .. } => true,\n",
    tests: ["test/test-files.test.ts", "-t", "several modules of tests"],
  },
  {
    name: "module-of-tests-all-unread",
    breaks: "a module of `#[cfg(all(test, ..))]` isn't a module of tests, its tests registered in a test file of its own",
    file: "src/lower/analysis/tests.rs",
    find: "        CfgEntry::All(all, _) => all.iter().any(needs_test),\n",
    replace: "",
    tests: ["test/test-files.test.ts", "-t", "several modules of tests"],
  },
  {
    name: "nested-tests-own-test-file",
    breaks: "a test in a module inside a module of tests is registered in a test file of its module's own, `shapes/tests/nested.test.js`, not the module of tests'",
    file: "src/lower/analysis/tests.rs",
    find: "        .find(|m| of_tests.contains(m))\n",
    replace: "        .find(|m| of_tests.contains(m) && *m == module)\n",
    tests: ["test/test-files.test.ts", "-t", "one module of tests is its test file"],
  },
  {
    name: "module-of-tests-without-tests",
    breaks: "a `#[cfg(test)]` module of no `#[test]`s, its tests' fixtures, is taken for a module of tests, and its parent's one module of tests is a file of its own",
    file: "src/lower/analysis/tests.rs",
    find: "    let with_tests: HashSet<LocalModId> = found.iter().map(|&(module, ..)| module).collect();\n",
    replace: "    let with_tests: HashSet<LocalModId> = tcx.hir_crate_items(()).definitions().filter(|&d| tcx.def_kind(d) == rustc_hir::def::DefKind::Mod).map(LocalModId::new_unchecked).collect();\n",
    tests: ["test/test-files.test.ts", "-t", "one module of tests is its test file"],
  },
  {
    name: "placed-test-exported",
    breaks: "a test its own module's test file registers, by its name, is exported too, as if another file imported it",
    file: "src/lower/analysis/tests.rs",
    find: "        if !placed.contains(&module) {\n",
    replace: "        if !placed.contains(&home) {\n",
    tests: ["test/test-files.test.ts", "-t", "one module of tests is its test file"],
  },
  {
    name: "registered-test-unexported",
    breaks: "a test another module's test file registers isn't exported, so what it registers is undefined",
    file: "src/lower/analysis/tests.rs",
    find: "        if !placed.contains(&module) {\n",
    replace: "        if false {\n",
    tests: ["test/test-files.test.ts", "-t", "several modules of tests"],
  },
];

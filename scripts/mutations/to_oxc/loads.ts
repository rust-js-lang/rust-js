// Mutations of src/to_oxc/loads.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const test = ["test/lowering-constants.test.ts", "-t", "a bound JS file imports back"];

export const mutations: Mutation[] = [
  {
    name: "export-from-loads-nothing",
    breaks: "a JS file's `export { x } from './generated.js'` loads nothing, so a module that binds it, where what it re-exports isn't there and may import the crate back, reads another module's constant by its name where it loads",
    file: "src/to_oxc/loads.rs",
    find: "        fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {\n            if !it.export_kind.is_type() {\n                self.named.push(it.source.value.to_string());\n",
    replace: "        fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {\n            if !it.export_kind.is_type() {\n",
    tests: test,
  },
  {
    name: "import-loads-nothing",
    breaks: "a JS file's `import` loads nothing, so a bound file that imports a module that imports its binder back is no cycle, and the module loaded first reads a constant before it's made",
    file: "src/to_oxc/loads.rs",
    find: "        fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {\n            if !it.import_kind.is_type() {\n",
    replace: "        fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {\n            if it.import_kind.is_type() {\n",
    tests: test,
  },
];

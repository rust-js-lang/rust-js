// Mutations of crates/rust-js-program/src/prepare/import_names.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "module-load-bindings-unreserved",
    breaks: "an import alias collides with a module-load statement's local binding",
    file: "crates/rust-js-program/src/prepare/import_names.rs",
    find: "            Item::Statements(stmts) => block(stmts, visitor),\n",
    replace: "            Item::Statements(_) => {}\n",
    tests: ["test/link.test.ts", "-t", "module-load statements"],
  },
  {
    name: "arrow-params-unreserved",
    breaks: "an import alias isn't named around what a nested arrow binds, so the arrow's parameter, `Card$1`, shadows the import `<Card$1 />` there",
    file: "crates/rust-js-program/src/prepare/import_names.rs",
    find: "            for param in params {\n                pattern(param, visitor);\n            }\n            block(body, visitor);\n",
    replace: "            let _ = (params, body);\n",
    tests: ["test/snapshots.test.ts", "-t", "playground"],
  },
  {
    name: "link-locals-not-apart-from-helpers",
    breaks:
      "a local of a helper's word its scope reads keeps it, `const copiedClone = ..` beside std's `copiedClone(..)`: rust-js refuses its own output (#1975)",
    file: "crates/rust-js-program/src/prepare/import_names.rs",
    find: "    js::scopes::locals_apart_from_module_reads(&top, &mut module.items);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(local_named_as_a_helper_its_scope_reads|clones_beside_a_generic_clone_of_a_copied_map).rs$"],
  },
  {
    name: "import-of-the-programs-unspelled-word-kept",
    breaks: "an import keeps the word a name of the program's not spelled yet will be spelled, an `on_load!` body's `value` beside `super::value`, so the program's is given apart, `value$2`, where the import gives way (#2070)",
    file: "crates/rust-js-program/src/prepare/import_names.rs",
    find: "        match name.made_base().filter(|_| name.is_the_programs()) {\n",
    replace: "        match None::<&str> {\n",
    tests: ["test/link.test.ts", "-t", "module-load statements resolve imports around their local bindings"],
  },
];

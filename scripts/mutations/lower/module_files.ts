// Mutations of crates/rust-js-lower/src/module_files.rs: where a module of
// the crate is, its path, its file and whether a `#[path]` placed it.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "path-directory-ignored",
    breaks: "a module in a `#[path]` directory has its JS where its module path says",
    file: "crates/rust-js-lower/src/module_files.rs",
    find: "        at = tcx.parent_module_from_def_id(at.to_local_def_id());\n",
    replace: "        return false;\n",
    tests: ["test/modules.test.ts", "-t", "path"],
  },
  {
    name: "path-inline-located",
    breaks: "an inline module in a `#[path]` directory is placed as if it had a file",
    file: "crates/rust-js-lower/src/module_files.rs",
    find: "    if Arc::ptr_eq(&module_file(tcx, module), &module_file(tcx, parent)) {\n        return false;\n    }\n",
    replace: "",
    tests: ["test/modules.test.ts", "-t", "path"],
  },
  {
    name: "block-modules-one-file",
    breaks: "two modules of one name in blocks of a module, bitflags' macro's, are one file",
    file: "crates/rust-js-lower/src/module_files.rs",
    find: "        Some(at) if at > 0 => {\n            let apart = js::Name::declared(name.to_string()).given_apart(at);\n",
    replace: "        Some(at) if false && at > 0 => {\n            let apart = js::Name::declared(name.to_string()).given_apart(at);\n",
    tests: ["test/corpus.test.ts","-t","block_modules"],
  },
  {
    name: "block-module-before-declared",
    breaks: "a module its parent declares is numbered after a block's of its name written before it",
    file: "crates/rust-js-lower/src/module_files.rs",
    find: "    namesakes.sort_by_key(|&m| (in_block(m), m.to_local_def_id().local_def_index));",
    replace: "    namesakes.sort_by_key(|&m| (false && in_block(m), m.to_local_def_id().local_def_index));",
    tests: ["test/corpus.test.ts","-t","block_modules"],
    snapshots: true,
  },
];

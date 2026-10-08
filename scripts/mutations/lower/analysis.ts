// Mutations of src/lower/analysis.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "thread-local-storage-static",
    breaks: "std's storage for a `thread_local!` is taken as a static of the crate's, and rejected",
    file: "src/lower/analysis.rs",
    find: "            DefKind::Static { .. } => !tcx.is_foreign_item(d) && in_thread_local(tcx, d).is_none(),",
    replace: "            DefKind::Static { .. } => !tcx.is_foreign_item(d),",
    tests: ["test/corpus.test.ts", "-t", "thread_local_syntax"],
  },
  {
    name: "js-import-const-written",
    breaks: "`js::import!`'s `const _` is written to the JS as a constant",
    file: "src/lower/analysis.rs",
    find: " && !bindings::is_mark(tcx, d.to_def_id())\n",
    replace: "\n",
    tests: ["test/compiler.test.ts","-t","js::import"],
  },
  {
    name: "pretty-flag-off",
    breaks: "a crate that shows a value with `{:#?}` gives its `Debug`s no `alternate`, and they're plain",
    file: "src/lower/analysis.rs",
    find: "    let pretty_debug = uses_pretty_debug(tcx, all_bodies);",
    replace: "    let pretty_debug = false && uses_pretty_debug(tcx, all_bodies);",
    tests: ["test/corpus.test.ts","-t","pretty_debug"],
  },
  {
    name: "user-hash-lowered",
    breaks: "a type's own `Hash`'s `hash` is lowered, and its `Hasher` calls refused, though nothing calls it",
    file: "src/lower/analysis.rs",
    find: "                    && !is_hash_impl(tcx, parent)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "user_hash"],
  },
  {
    name: "generic-impl-const-bodies-uncollected",
    breaks: "a generic impl's constant of its parameters, `Wrapping(T::ZERO)`, is an error",
    file: "src/lower/analysis.rs",
    find: "            DefKind::AssocConst { .. } => tcx.trait_impl_of_assoc(def_id.to_def_id()).is_some_and(|imp| {",
    replace: "            DefKind::AssocConst { .. } => tcx.trait_impl_of_assoc(def_id.to_def_id()).is_some_and(|imp| false && {",
    tests: ["test/corpus.test.ts", "-t", "generic_impl_consts"],
  },
  {
    name: "js-object-deref-lowered",
    breaks: "a JS class's `Deref` to the class it extends is lowered, a pointer cast rust-js refuses",
    file: "src/lower/analysis.rs",
    find: "                    && !is_js_object_deref(tcx, parent)\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "untagged enum"],
  },
  {
    name: "reexport-only-module-unwritten",
    breaks: "a module of only `pub use`s, Sidebar/index, gets no file, and what imports it finds nothing",
    file: "src/lower/analysis.rs",
    find: "if reexports && seen_modules.insert(module)",
    replace: "if false && reexports && seen_modules.insert(module)",
    tests: ["test/compiler.test.ts", "-t", "only pub uses"],
  },
];

// Mutations of src/lower/reexports.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reexport-alias-unread",
    breaks:
      "`pub use inner::other as renamed;` re-exports `other`, not `renamed`",
    file: "src/lower/reexports.rs",
    find: "            false => super::bindings::alias_name(tcx, item.owner_id.to_def_id(), ident.name),\n        };\n        // Its value",
    replace:
      "            false => name.to_string(),\n        };\n        // Its value",
    tests: ["test/modules.test.ts", "-t", "re-exported from it"],
  },
  {
    name: "default-of-another-unexported",
    breaks:
      "another module's function named the default isn't re-exported as it",
    file: "src/lower/reexports.rs",
    find: "        found.push((\n            span,\n",
    replace: "        drop((\n            span,\n",
    tests: ["test/modules.test.ts", "-t", "default of another module"],
  },
  {
    name: "binding-reexport-unwritten",
    breaks:
      "a `pub use` of a JS module's binding writes no `export { .. } from` it",
    file: "src/lower/reexports.rs",
    find: "            if super::bindings::is_binding(tcx, def_id) {\n",
    replace:
      "            if false && super::bindings::is_binding(tcx, def_id) {\n",
    tests: ["test/modules.test.ts", "-t", "JS module's binding is re-exported"],
  },
  {
    name: "reexported-default-by-name",
    breaks:
      "a `pub use` of another module's default re-exports it by its name, which that module doesn't export",
    file: "src/lower/reexports.rs",
    find: "                        super::default_exports::exported_as(tcx, def_id, info.exported()),\n                        alias(info.exported()),\n",
    replace:
      "                        info.name.clone(),\n                        alias(info.exported()),\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "default export is imported as its default",
    ],
  },
  {
    name: "library-reexport-refused",
    breaks:
      "a `pub use` of a library's constant, function or default is refused, where it's `export { .. } from` the library's file (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "            if let Some(imported) = foreign.item(def_id)\n",
    replace:
      "            if let Some(imported) = foreign.item(def_id).filter(|_| false)\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "re-exports it from the library's file",
    ],
  },
  {
    name: "library-held-binding-reexport-dropped",
    breaks:
      "a `pub use` of a binding a library's module holds re-exports nothing, `shared::assign::assign` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "            super::bindings::Holder::Library(from) => js::ImportFrom::Specifier(from.clone()),\n",
    replace: "            super::bindings::Holder::Library(_) => continue,\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "re-exports it from the library's file",
    ],
  },
  {
    name: "class-reexport-refused",
    breaks:
      "a `pub use` of a class is refused, where it's its constructor's `export { Circle } from` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "                let Some((from, export)) = crate_class().or_else(library_class) else {\n",
    replace:
      "                let Some((from, export)) = crate_class().or_else(library_class).filter(|_| false) else {\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "module-reexport-named",
    breaks:
      "a `pub use` of a module is `export { * as ns } from`, which isn't JS, not `export * as ns from` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: '                            ("*".to_string(), alias),\n',
    replace: '                            ("default".to_string(), alias),\n',
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-from-where-declared",
    breaks:
      "a `pub use` of what another module re-exports names the module that declares it, `./cache_impl.js`, not the one its path names, `./cache_server.js` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "                    .filter(|&through| through != info.module)\n",
    replace: "                    .filter(|_| false)\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-through-renamed",
    breaks:
      "a `pub use` that keeps the name a module re-exports it by renames it camelCase, `unstable_cache as unstableCache` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "                    Some((through, export)) if Some(ident.name) == last_name(path) => (through, export.clone(), export),\n",
    replace:
      "                    Some((through, export)) if false => (through, export.clone(), export),\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-grouped-by-module",
    breaks:
      "a module's `pub use`s from one module are one statement wherever they're written, their comments and order lost (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "            .find(|(was, reexport)| was.lo() == at.lo() && reexport.from == from)\n",
    replace: "            .find(|(_, reexport)| reexport.from == from)\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-merged-across-comments",
    breaks:
      "two `pub use`s of one module with a comment between are one statement, the comment lost (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "                    .is_some_and(only_attributes) =>\n",
    replace: "                    .is_some() =>\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-run-unmerged",
    breaks:
      "a run of `pub use`s of one module is a statement each, `export { helper } from` and `export { other as renamed } from` (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "                    .is_some_and(only_attributes) =>\n",
    replace: "                    .is_some_and(|_| false) =>\n",
    tests: ["test/modules.test.ts", "-t", "re-exported from it"],
  },
  {
    name: "reexport-attributes-part-runs",
    breaks:
      "a `pub use`'s `#[rust_js::name]` parts it from the run before it, a statement each (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: '        let Some(attribute) = trimmed.strip_prefix("#[") else {\n',
    replace:
      '        let Some(attribute) = trimmed.strip_prefix("#[").filter(|_| false) else {\n',
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "reexport-tree-leaves-apart",
    breaks:
      "a `pub use m::{a, b};` is a statement for each name, and its comments are lost (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "            false => item.vis_span.to(item.span),\n",
    replace: "            false => item.span,\n",
    tests: ["test/modules.test.ts", "-t", "re-exported as written"],
  },
  {
    name: "unexpressed-reexport-unrefused",
    breaks:
      "a `pub use` of std's function re-exports nothing, silently (ADR 0240, amended)",
    file: "src/lower/reexports.rs",
    find: "            refuse(\n                statement,\n                &what,\n",
    replace:
      "            let _ = (\n                statement,\n                &what,\n",
    tests: ["test/diagnostics.test.ts", "-t", "a pub use of std's function"],
  },
  {
    name: "reexports-alone-of-anything",
    breaks:
      "a crate's root with an item of its own is taken as re-exporting alone, so it writes no JS where its package's entry re-exports its `pub use`s, its item lost (ADR 1008, amended)",
    file: "src/lower/reexports.rs",
    find: "            _ => super::analysis::is_reexport(tcx, item),\n",
    replace: "            _ => true,\n",
    tests: ["test/libraries.test.ts", "-t", "entry re-exports"],
  },
  {
    name: "export-named-apart-reexported-apart",
    breaks:
      "`pub use crate::delete;` re-exports it as `export { delete as delete$ }`, not by its own name (#338)",
    file: "src/lower/reexports.rs",
    find: "                        alias(info.exported()),\n",
    replace: "                        alias(&info.name),\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "glob-names-any-glob",
    breaks:
      "a module's `pub use m::*;` re-exports what another glob of it gives too, each name twice, a duplicate export (rd153)",
    file: "src/lower/reexports.rs",
    find: "        if !matches!(child.reexport_chain.first(), Some(Via::Glob(used)) if *used == glob) {\n",
    replace:
      "        if !matches!(child.reexport_chain.first(), Some(Via::Glob(_))) {\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a pub use of every name of a module re-exports each it gives",
    ],
  },
  {
    name: "glob-names-unresolved",
    breaks:
      "a module's `pub use m::*;` re-exports each of the module's names, its own items too, not what the glob gives it, past what they shadow (rd153)",
    file: "src/lower/reexports.rs",
    find: "        if !matches!(child.reexport_chain.first(), Some(Via::Glob(used)) if *used == glob) {\n",
    replace: "        if child.reexport_chain.is_empty() {\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a pub use of every name of a module re-exports each it gives",
    ],
  },
  {
    name: "glob-library-names-refused",
    breaks:
      "a `pub use m::*;` of a library's module refuses its functions and constants, which its file exports (rd153)",
    file: "src/lower/reexports.rs",
    find: "                    && let Some((from, export)) = foreign.import_key(id, module)\n                {\n                    found.push((\n",
    replace:
      "                    && let Some((from, export)) = foreign.import_key(id, module)\n                    && false\n                {\n                    found.push((\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a pub use of every name of a module re-exports each it gives",
    ],
  },
  {
    name: "glob-module-left-out",
    breaks:
      "a `pub use m::*;` of a module holding a module leaves its namespace out, not refused (rd153)",
    file: "src/lower/reexports.rs",
    find: "            DefKind::Mod | DefKind::Struct if matches!(kind, DefKind::Mod) || super::classes::is_class(tcx, id) => {\n",
    replace:
      "            DefKind::Struct if super::classes::is_class(tcx, id) => {\n",
    tests: ["test/diagnostics.test.ts", "-t", "every name of a module"],
  },
  {
    name: "forwarded-methods-unexported",
    breaks:
      "a namespace's method that's the function it passes its parameters on to isn't re-exported, so its module has no `batchedUpdates` (rd292b)",
    file: "src/lower/reexports.rs",
    find: "    forwarded_methods(tcx, module, (fns, forwarded), paths, &mut found);\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
  {
    name: "forwarded-methods-each-a-statement",
    breaks:
      "a run of a namespace's methods each the function it passes its parameters on to, from one module, is a statement each, not one `export { .. } from` as a `pub use` of a list is (rd292b)",
    file: "src/lower/reexports.rs",
    find: "                (Some(from), Some(run)) if last == Some(from) => run.push((method, from)),\n",
    replace: "                (Some(from), Some(run)) if false && last == Some(from) => run.push((method, from)),\n",
    tests: ["test/namespaces.test.ts", "-t", "only passes its parameters on"],
  },
];

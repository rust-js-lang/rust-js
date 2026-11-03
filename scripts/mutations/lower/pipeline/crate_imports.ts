// Mutations of src/lower/pipeline/crate_imports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-class-imported-by-name",
    breaks:
      "a module that uses another's default class imports `{ Complex }`, which isn't exported",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "            Some(_) if let Some(class) = crate::lower::classes::class_of(tcx, id) => (\n",
    replace:
      "            Some(_) if let Some(class) = crate::lower::classes::class_of(tcx, id).filter(|_| false) => (\n",
    tests: [
      "test/classes.test.ts",
      "-t",
      "a class is its module's default export",
    ],
  },
  {
    name: "through-function-from-home",
    breaks:
      "a crate's function a `use` names through a module that re-exports it is imported from its home, `./home.js`, where upstream's react-markup imports from the re-exporting module (rd650)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "            Some(through) if info.owner.is_none() => (through.export.clone(), through.module),\n",
    replace: "            Some(through) if false && info.owner.is_none() => (through.export.clone(), through.module),\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a module"],
  },
  {
    name: "reimported-after-the-others",
    breaks:
      "a library's item imported from the crate's module that re-exports it is listed after that module's others, `{ helper, own, createState }` (rd650)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "    requests.sort_by(|a, b| (&a.path, &a.export).cmp(&(&b.path, &b.export)));\n",
    replace: "",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a module"],
  },
  {
    name: "own-import-sorted-by-local",
    breaks:
      "an import lists what it imports by the names it imports them by, `{ MAX as LIMIT, unstable_now, toArray }`",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "    requests.sort_by(|a, b| (&a.path, &a.export).cmp(&(&b.path, &b.export)));\n",
    replace: "    requests.sort_by(|a, b| (&a.path, &a.local).cmp(&(&b.path, &b.local)));\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a module's own item a pub use renames is imported by that name",
    ],
  },
  {
    name: "default-imported-by-name",
    breaks:
      "a module's default export is imported by its name, `import { Memoized as Label }`",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                crate::lower::default_exports::exported_as(tcx, id, info.exported()),\n",
    replace: "                name.clone(),\n",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "default export is imported as one",
    ],
  },
  {
    name: "default-import-unrenamed",
    breaks:
      "a default import is named as its item is, `import Memoized`, not as the importer's `use .. as Label` names it",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone()),",
    replace:
      "                (crate::lower::default_exports::defaults(tcx).into_iter().any(|(_, item)| item == id)).then(|| name.clone()).unwrap_or_else(|| renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone())),",
    tests: [
      "test/declarations.test.ts",
      "-t",
      "default export is imported as one",
    ],
  },
  {
    name: "named-import-alias-dropped",
    breaks:
      "a `use` that renames another module's function imports it by its own name",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone()),",
    replace:
      "                (!crate::lower::default_exports::defaults(tcx).into_iter().any(|(_, item)| item == id)).then(|| name.clone()).unwrap_or_else(|| renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone())),",
    tests: ["test/modules.test.ts", "-t", "renames another module's function"],
  },
  {
    name: "js-given-copy-unimported",
    breaks:
      "a copy of another module's function a module calls by the name its `use` gives isn't imported, so `relay_named`'s `wrappedUnboxed` has no import, and the case fails",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "            .insert((target, name, export, local, target, None));",
    replace: "            .get(&(target, name, export, local, target, None));",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
  },
  {
    name: "export-named-apart-imported-apart",
    breaks:
      "another module of the crate imports an export named apart by its module's name for it, `import { delete$ }`, which its module doesn't export (#338)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                crate::lower::default_exports::exported_as(tcx, id, info.exported()),\n",
    replace:
      "                crate::lower::default_exports::exported_as(tcx, id, &name),\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "reexporter-namespace-unimported",
    breaks:
      "a module that reads a function only through a module that re-exports it, `Plugins.doubled(2)`, isn't offered that module's namespace import, and the linker panics (rd137)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "let targets = imports.iter().map(|&(target, ..)| target).chain(reexporters);\n",
    replace:
      "let targets = imports.iter().map(|&(target, ..)| target).chain(reexporters.take(0));\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a function called through its module is read through its namespace",
    ],
  },
  {
    name: "namespace-at-first-use",
    breaks:
      "a module's namespace, `import * as Lanes`, is at the first `use` of the module, not at `use crate::lanes;` (rd649)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                Some(target.to_def_id()),\n",
    replace: "                None,\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "two uses of one module"],
  },
  {
    name: "crate-item-use-unnamed",
    breaks:
      "a crate's item is imported at its module's first `use`, not at the `use` naming it, so two `use`s of a module are one import (rd649)",
    file: "src/lower/pipeline/crate_imports.rs",
    find: "                Some(id),\n            ),\n        };\n",
    replace: "                None,\n            ),\n        };\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "two uses of one module"],
  },
];

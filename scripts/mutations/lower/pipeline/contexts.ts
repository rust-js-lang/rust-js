// Mutations of src/pipeline/contexts.rs (ADR 0093): the context a
// body is lowered in, and the crate's facts found in one before any body.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-callback-params-none",
    breaks:
      "a `dyn Fn` parameter each of the crate's calls fills with its own function isn't one, `on(() => { onEnd(); })` where React Flight's Node client gives `stream.on('end', onEnd)` (rd616)",
    file: "src/pipeline/contexts.rs",
    find: "    item_cx((&shared, &settled), function.to_def_id(), (module, &names)).find_own_callbacks()\n",
    replace: "    drop((shared, settled, names, module));\n    IndexSet::default()\n",
    tests: ["test/corpus.test.ts", "-t", "^own_callback_params.rs$"],
    snapshots: true,
  },
  {
    name: "library-namespace-a-directory-deeper",
    breaks:
      "a library's module of unchecked functions a module of the crate's re-exports, the app's module in its place beside its owner's file, imports the library's file one `../` too far up, `../../../dep/rng.js` from `app/rng_alone/rng.js`, which throws \"Cannot find module\" (#2403 (Y), #2536)",
    file: "src/pipeline/contexts.rs",
    find: "        let from = specifier.clone();\n",
    replace: "        let from = match paths[module].is_empty() || !specifier.starts_with('.') {\n            true => specifier.clone(),\n            false => format!(\"../{}\", specifier.trim_start_matches(\"./\")),\n        };\n",
    tests: ["test/glob-reexports.test.ts", "-t", "a module whose only item re-exports a library's module is a file"],
  },
  {
    name: "library-namespace-declared-from-unplaced",
    breaks:
      "a library's module of unchecked functions the app's module stands in for, a module deep or more, declares `export * from` the library's file as the crate's root names it, not from its own file, so TypeScript finds no types there (#2403 (Y), #2536)",
    file: "src/pipeline/contexts.rs",
    find: '"declarations": [{ "kind": "export", "specifier": from, "names": [["*", "*"]] }],',
    replace: '"declarations": [{ "kind": "export", "from": from, "names": [["*", "*"]] }],',
    tests: ["test/glob-reexports.test.ts", "-t", "a module whose only item re-exports a library's module is a file"],
  },
];

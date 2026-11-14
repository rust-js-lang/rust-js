// Mutations of src/lower/pipeline/contexts.rs (ADR 0093): the context a
// body is lowered in, and the crate's facts found in one before any body.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-callback-params-none",
    breaks:
      "a `dyn Fn` parameter each of the crate's calls fills with its own function isn't one, `on(() => { onEnd(); })` where React Flight's Node client gives `stream.on('end', onEnd)` (rd616)",
    file: "src/lower/pipeline/contexts.rs",
    find: "    item_cx((&shared, &settled), function.to_def_id(), (module, &names)).find_own_callbacks()\n",
    replace: "    drop((shared, settled, names, module));\n    IndexSet::default()\n",
    tests: ["test/corpus.test.ts", "-t", "^own_callback_params.rs$"],
    snapshots: true,
  },
];

// Mutations of src/lower/body_queries/ranges/empty.rs (ADR 0847).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "once-drop-runs-nothing",
    breaks:
      "a struct whose `Drop` sets a cell it reaches is taken as running nothing where it's dropped, so a cell its guard set is still known empty",
    file: "src/lower/body_queries/ranges/empty.rs",
    find: "                if tcx.adt_destructor(adt.did()).is_some() {\n                    return true;\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-made-by-any-call",
    breaks:
      "a `OnceCell` any call gives is taken as made empty, so one `filled()` set is assigned again, and `Ok`",
    file: "src/lower/body_queries/ranges/empty.rs",
    find: "                if once_method(tcx, def_id) == Some(OnceOp::New) {\n",
    replace:
      "                if once_method(tcx, def_id) == Some(OnceOp::New) || !is_pointer_new(tcx, def_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
  },
  {
    name: "once-rc-clone-gives-cells",
    breaks:
      "a clone of an `Rc` a struct holds isn't one that reaches its cells, so the read of it gives them away, and one kept is set where it's empty",
    file: "src/lower/body_queries/ranges/empty.rs",
    find: "                    found.push((read, id));\n",
    replace: "                    let _ = (read, id);\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"],
    snapshots: true,
  },
  {
    name: "once-rc-deref-unfollowed",
    breaks:
      "a cell through an `Rc`, `payload.io_info`, isn't one a variable is of, so React's `payload._ioInfo = ioInfo` is `if (payload._ioInfo === undefined) ..`",
    file: "src/lower/body_queries/ranges/empty.rs",
    find: "                    && is_rc_deref(tcx, def_id, generic_args)\n",
    replace: "                    && !is_rc_deref(tcx, def_id, generic_args)\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
    snapshots: true,
  },
];

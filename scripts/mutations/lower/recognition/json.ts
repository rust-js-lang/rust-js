// Mutations of src/lower/recognition/json.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "serde-op-unknown",
    breaks:
      "a hand-written `Serialize`'s `s.serialize_str(..)` isn't known, and is refused",
    file: "src/lower/recognition/json.rs",
    find: "        if let Some(op) = serde_op(tcx, def_id) {\n",
    replace: "        if let Some(op) = None::<SerdeOp> {\n",
    tests: ["test/serde.test.ts", "-t", "hand-written"],
  },
];

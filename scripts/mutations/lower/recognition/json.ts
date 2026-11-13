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
  {
    name: "json-index-mut-unrecognized",
    breaks: "`value[key] = ..` of a `Value` isn't recognized, so it's refused where serde_json writes through its `index_or_insert` (#1624)",
    file: "src/lower/recognition/json.rs",
    find: "            if tcx.is_lang_item(trait_id, LangItem::IndexMut) && this_value {\n",
    replace: "            if false && tcx.is_lang_item(trait_id, LangItem::IndexMut) && this_value {\n",
    tests: ["test/serde-values.test.ts", "-t", "IndexMut is serde_json"],
  },
];

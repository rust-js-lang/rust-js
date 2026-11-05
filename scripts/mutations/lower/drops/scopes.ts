// Mutations of src/lower/drops/scopes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "default-drops-impl-indices",
    breaks:
      "a default copied into an impl takes the impl's drops by their indices, so `Self` drops as the impl's `T` does",
    file: "src/lower/drops/scopes.rs",
    find: "            params: std::mem::replace(&mut self.drop_state.param_drops, drops),\n",
    replace:
      "            params: { drop(drops); self.drop_state.param_drops.clone() },\n",
    tests: ["test/corpus.test.ts", "-t", "^(drop_impl_indirect|default_own_param_drops).rs$"],
  },
  {
    name: "hoisted-drops-lost",
    breaks: "a body's drop functions aren't declared at its top",
    file: "src/lower/drops/scopes.rs",
    find: "        std::mem::replace(&mut self.drop_state.hoisted, enclosing).map_or_else(Vec::new, |hoisted| hoisted.defs)\n",
    replace:
      "        std::mem::replace(&mut self.drop_state.hoisted, enclosing).map_or_else(Vec::new, |_| Vec::new())\n",
    tests: ["test/corpus.test.ts", "-t", "drop_functions"],
  },
  {
    name: "given-item-drop-unfound",
    breaks:
      "a std trait's associated type's drop, given, isn't found by its type, and isn't called",
    file: "src/lower/drops/scopes.rs",
    find: "            ty::Alias(_, alias) => self.drop_query().slot_of(*alias)?,\n",
    replace: "            ty::Alias(..) => return None,\n",
    tests: ["test/corpus.test.ts", "-t", "^std_projection_drops.rs$"],
  },
];

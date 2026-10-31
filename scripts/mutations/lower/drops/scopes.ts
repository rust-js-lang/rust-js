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
    tests: ["test/corpus.test.ts", "-t", "drop_impl_indirect"],
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
  {
    name: "slot-default-env",
    breaks:
      "a copied default's slots are normalized in the trait's typing environment, which can't see `Holder<I>`'s impl",
    file: "src/lower/drops/scopes.rs",
    find: "                self.impl_typing_env().unwrap_or(self.typing_env),\n                ty::GenericArgs::identity_for_item",
    replace:
      "                self.typing_env,\n                ty::GenericArgs::identity_for_item",
    tests: ["test/corpus.test.ts", "-t", "^copied_iterator_defaults.rs$"],
  },
  {
    name: "impl-drops-not-exchanged",
    breaks:
      "what's in a copied default's impl's terms is asked of its trait's drops",
    file: "src/lower/drops/scopes.rs",
    find: "        self.exchange_drops(&mut outer);\n        let made = f(self);\n        self.exchange_drops(&mut outer);\n",
    replace: "        let made = f(self);\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_default_impl_drops.rs$"],
  },
];

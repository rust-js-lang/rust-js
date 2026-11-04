// Mutations of src/lower/declarations/dictionaries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trait-interface-unparameterized",
    breaks: "a trait's dictionaries are declared of no `Self`, `interface Shape`, whose methods' `self: Self` TypeScript can't name (#550)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "            \"typeParameters\": [{ \"name\": \"Self\" }],\n",
    replace: "            \"typeParameters\": [],\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "generic-impl-dictionary-declared",
    breaks: "a generic impl's dictionary, `wrapperShape(TDisplay)`, a function of its bounds' dictionaries, is declared a `const` of the trait (#550)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "                .all(|param| matches!(param.kind, ty::GenericParamDefKind::Lifetime))\n",
    replace: "                .all(|_| true)\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "dictionary-trait-unimported",
    breaks: "the type of a dictionary of another module's trait, `Shape<S>`, is named without importing it, which TypeScript can't find (#550)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "            self.imported(self.files[&home].clone(), trait_id, name.clone());\n",
    replace: "            let _ = &name;\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "undeclared-trait-dictionary-declared",
    breaks: "a function's dictionary of a trait that isn't declared, `labels(l, LLabeled)`'s, is declared of it, `LLabeled: Labeled<L>`, which names nothing, and a JS caller's has no `Display` (#550)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "        (home == self.module || self.files.contains_key(&home)) && self.declared_methods(trait_id).is_some()\n",
    replace: "        home == self.module || self.files.contains_key(&home)\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "type of its dictionaries"],
  },
  {
    name: "pair-dictionary-unimported",
    breaks: "a `dyn` of another module's trait names `Shape<unknown>`, which its declarations don't import (#620)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "        let dictionary = self.dictionary_of(trait_id, keyword(\"unknown\"));\n",
    replace: "        let dictionary = reference(self.tcx.item_name(trait_id).as_str(), vec![keyword(\"unknown\")]);\n",
    tests: ["test/declarations-ungiven.test.ts", "-t", "box is declared"],
  },
];

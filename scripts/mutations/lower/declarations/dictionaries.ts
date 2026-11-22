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
  {
    name: "library-trait-dictionary-undeclared",
    breaks: "a `dyn` of a library's trait whose declarations declare its dictionaries is `any`, its contract unread (#1870)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "            return self.foreign.declared_type(trait_id).is_some();\n",
    replace: "            return false;\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "a dyn of a library"],
  },
  {
    name: "library-trait-dictionary-unimported",
    breaks: "a library's trait's dictionary type is named but not imported from its file, so `Middleware` is no name TypeScript has (#1870)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "            self.library_imports.insert((from.clone(), name.clone(), *default));\n",
    replace: "            let _ = (from, default);\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "a dyn of a library"],
  },
  {
    name: "trait-interface-unreexported",
    breaks: "a `pub use` of a trait leaves out the type of its dictionaries, `export type { Shape as Form }`, which its module's declarations declare (#2095)",
    file: "src/lower/declarations/dictionaries.rs",
    find: "    let interface = declared_methods(tcx, foreign, trait_id).map(|_| tcx.item_name(trait_id).to_string());\n",
    replace: "    let interface = declared_methods(tcx, foreign, trait_id).and(None::<String>);\n",
    tests: ["test/reexport-docs.test.ts", "-t", "re-exports its interface"],
  },
];

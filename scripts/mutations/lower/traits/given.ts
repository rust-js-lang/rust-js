// Mutations of src/lower/traits/given.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-fact-of-wrong-kind",
    breaks: "`align_of::<T>()` reads the size its caller gave",
    file: "src/lower/traits/given.rs",
    find: "            .find(|&&(of, given, _)| matches!(of.kind(), ty::Param(p) if p.index == index) && given == fact)",
    replace:
      "            .find(|&&(of, _, _)| matches!(of.kind(), ty::Param(p) if p.index == index))",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "location-param-unnamed",
    breaks:
      "a function that takes its caller's location reads none it was given",
    file: "src/lower/traits/given.rs",
    find: "            self.given.caller_location = Some((id, Expr::var(&name)));\n",
    replace: "            let _ = &name;\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "closure-reads-given-location",
    breaks:
      "`Location::caller()` in a closure in a tracked function is the function's caller's, not where it's written",
    file: "src/lower/traits/given.rs",
    find: "        (*owner == self.frame.owner).then(|| location.clone())\n",
    replace: "        let _ = owner;\n        Some(location.clone())\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "default-drops-without-method-bounds",
    breaks:
      "a copied default's type naming the trait method's own const parameter is asked about where only the impl's are, and rustc crashes",
    file: "src/lower/traits/given.rs",
    find: "        let bounds = (impl_env.param_env.caller_bounds().iter()).chain(own).chain(sized);",
    replace:
      "        let bounds = (impl_env.param_env.caller_bounds().iter()).chain(sized);",
    tests: ["test/corpus.test.ts", "-t", "^array_defaults.rs$"],
  },
  {
    name: "drop-slots-ungiven",
    breaks:
      "a function's associated types a caller could give a drop for aren't known as such, and one none gives is refused",
    file: "src/lower/traits/given.rs",
    find: "        self.give_drop_slots(drop_items(self.tcx, self.krate.foreign, id));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(associated_into_iter|back_steps).rs$",
    ],
  },
  {
    name: "default-sized-not-impls",
    breaks:
      "a default's `Self: Sized` isn't the impl's `T: Sized`, and its trait on an adapter of `Self` is refused",
    file: "src/lower/traits/given.rs",
    find: "        let bounds = (impl_env.param_env.caller_bounds().iter()).chain(own).chain(sized);",
    replace:
      "        let bounds = (impl_env.param_env.caller_bounds().iter()).chain(own).chain(sized.filter(|_| false));",
    tests: ["test/corpus.test.ts", "-t", "^default_on_an_adapter_of_self.rs$"],
  },
  {
    name: "default-resolved-in-trait-env",
    breaks:
      "a default copied into `impl<T> Getter<T> for Option<T>` calls the impl's methods through its dictionary, not directly: right, but not the JS a person writes",
    file: "src/lower/traits/given.rs",
    find: "        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);\n        crate::lower::items::resolve_instance_in(",
    replace:
      "        let typing_env = self.typing_env;\n        crate::lower::items::resolve_instance_in(",
    tests: ["test/corpus.test.ts", "-t", "evidence_paths"],
    snapshots: true,
  },
  {
    name: "default-resolved-without-own-bounds",
    breaks:
      "a call in a copied default resolves in the impl's environment alone, and the crate's `Index<Id> for [T]` of the default's own `[B]` isn't found",
    file: "src/lower/traits/given.rs",
    find: "        let typing_env = self.impl_typing_env().unwrap_or(self.typing_env);\n        crate::lower::items::resolve_instance_in(",
    replace:
      "        let typing_env = self.given.self_env.unwrap_or(self.typing_env);\n        crate::lower::items::resolve_instance_in(",
    tests: ["test/corpus.test.ts", "-t", "^default_crate_index.rs$"],
  },
  {
    name: "drop-evidence-unseen",
    breaks:
      "a dictionary given only its type parameter's drop is taken to take nothing, its function read as its object",
    file: "src/lower/traits/given.rs",
    find: "        self.takes_more_than_drops(id) || !self.drops_taken(id).is_empty()\n",
    replace: "        self.takes_more_than_drops(id)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_consts.rs$"],
  },
  {
    name: "bound-evidence-unseen",
    breaks:
      "a dictionary given its type parameter's dictionary is taken to take nothing, its function read as its object",
    file: "src/lower/traits/given.rs",
    find: "            || !bounds(self.tcx, self.krate.foreign, self.krate.given, id).is_empty()\n",
    replace: "",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },
  {
    name: "const-param-evidence-unseen",
    breaks:
      "a dictionary given a const parameter's value is taken to take nothing",
    file: "src/lower/traits/given.rs",
    find: "        !const_params(self.tcx, id).is_empty()\n            || ",
    replace: "        ",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(array_vec_raw|associated_type_facts|borrow_mut_supertrait).rs$",
    ],
  },
  {
    name: "type-fact-evidence-unseen",
    breaks:
      "a dictionary given a type's fact, its size, is taken to take nothing",
    file: "src/lower/traits/given.rs",
    find: "            || !self.facts_asked(id).is_empty()\n",
    replace: "",
    tests: [
      "test/lowering-dictionaries.test.ts",
      "-t",
      "a dictionary that takes nothing",
    ],
  },

  {
    name: "default-facts-kept",
    breaks:
      "a copied default's facts of its own type parameters stay given past it, so the next default's fact is refused as given twice",
    file: "src/lower/traits/given.rs",
    find: "        self.given.type_facts.truncate(scope.facts);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(generic_maps|item_keyed_map).rs$"],
  },
  {
    name: "default-consts-kept",
    breaks:
      "a copied default's const parameters stay given past it, read by what's lowered after it",
    file: "src/lower/traits/given.rs",
    find: "        self.given.const_params = scope.const_params;\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(const_generic_traits|copied_default_own_params).rs$",
    ],
    snapshots: true,
  },
];

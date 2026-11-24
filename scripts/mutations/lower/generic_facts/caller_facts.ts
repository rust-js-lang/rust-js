// Mutations of crates/rust-js-lower/src/generic_facts/caller_facts.rs (ADR 0093): what a caller gives generic code that only the caller knows.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-error-impl-type-id",
    breaks: "a generic `Error` impl's dictionary is given a `TypeId`, of a type with a type parameter, which no caller gives: every program with one is refused",
    file: "crates/rust-js-lower/src/generic_facts/caller_facts.rs",
    find: "    !tcx.type_of(imp).instantiate_identity().skip_normalization().has_param()\n",
    replace: "    let _ = imp;\n    true\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_error_downcast.rs$"],
  },
  {
    name: "every-param-given-a-drop",
    breaks: "every type parameter is taken as given a drop, so a map of items no caller gives one is made `$mapEnds` where it's stepped from its back, its JS other than the snapshot's (#1406)",
    file: "crates/rust-js-lower/src/generic_facts/caller_facts.rs",
    find: "            if given.is_none_or(|given| given(tcx.typeck_root_def_id(owner), param.index)))\n",
    replace: "            if given.is_none_or(|_| true))\n",
    tests: ["test/corpus.test.ts", "-t", "^map_as_it_is_stepped_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "derived-clone-of-its-last-field-drops",
    breaks: "a derived `Clone` whose only field of a type parameter's is its last is taken to drop it, so it's given drops nothing calls, `Fuse::clone`'s (#1517)",
    file: "crates/rust-js-lower/src/generic_facts/caller_facts.rs",
    find: "        (fields.iter().rev().skip(1)).any(|field| field.ty(tcx, args).skip_normalization().has_param())\n",
    replace: "        (fields.iter()).any(|field| field.ty(tcx, args).skip_normalization().has_param())\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "trusted-part-decided-here-asked",
    breaks: "a `Chain`'s side rustc says is `TrustedLen` here is asked of its callers too, `ITrustedLen && true` (#1614)",
    file: "crates/rust-js-lower/src/generic_facts/caller_facts.rs",
    find: "    facts.into_iter().filter(|&(part, _)| part.has_param()).collect()\n",
    replace: "    facts\n",
    tests: ["test/corpus.test.ts", "-t", "^capacity_of_text_patterns.rs$"],
    snapshots: true,
  },
];

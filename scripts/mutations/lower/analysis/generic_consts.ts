// Mutations of src/lower/analysis/generic_consts.rs.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trait-default-reads-generic",
    breaks:
      "a trait default's `Self::B` counts as generic code reading `B`, so `()`'s defaults of each other are computed, a cycle",
    file: "src/lower/analysis/generic_consts.rs",
    find: "        !(matches!(tcx.def_kind(body.def_id), DefKind::AssocConst { .. }) && tcx.trait_of_assoc(body.def_id).is_some())\n",
    replace:
      "        true || !(matches!(tcx.def_kind(body.def_id), DefKind::AssocConst { .. }) && tcx.trait_of_assoc(body.def_id).is_some())\n",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "defaulting to each other",
    ],
  },
  {
    name: "generic-consts-of-instances-unread",
    breaks:
      "the trait constants a library's code the crate compiles reads, of its MIR alone, aren't asked of the dictionaries it compiles beside it, so a program's `Stack<Loud, 2>` is full at once, its `CAPACITY` undefined (#468, ADR 0176, amended)",
    file: "src/lower/analysis/generic_consts.rs",
    find: "        if body.root().is_none()\n            && let Some(mir) = &body.mir\n",
    replace: "        if false\n            && let Some(mir) = &body.mir\n",
    tests: ["test/crates.test.ts", "-t", "three crates: generic_impl_const_of_instance"],
  },
];

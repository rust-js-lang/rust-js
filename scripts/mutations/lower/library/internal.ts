// Mutations of src/lower/library/internal.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "internal-atomic-exported",
    breaks:
      "a library's private atomic `static`, which its module unboxes and sets in place, is exported by an internal name, so a consumer's copy of its `fetch_add` assigns the import, a `TypeError`, where it was refused (#2017)",
    file: "src/lower/library/internal.rs",
    find: "            && !ty.is_freeze(tcx, rustc_middle::ty::TypingEnv::fully_monomorphized());\n",
    replace: "            && false;\n",
    tests: ["test/crates-compiled-here.test.ts", "-t", "merge what its consumer compares"],
  },
  {
    name: "internal-state-unrooted",
    breaks:
      "a library exports none of its private state, so a consumer that compiles `Holder::copies` itself, which reads a private `static`, is refused (#2017)",
    file: "src/lower/library/internal.rs",
    find: "        .filter(|&id| function(id) && escapes(tcx, library, id))\n",
    replace: "        .filter(|&id| function(id) && false && escapes(tcx, library, id))\n",
    tests: ["test/cross-crate-private-state.test.ts"],
  },
];

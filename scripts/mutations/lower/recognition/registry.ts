// Mutations of src/lower/recognition/registry.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "registry-without-vec",
    breaks:
      "`Vec` isn't one of std's data structures counted, so its methods leave the ratchet unseen",
    file: "src/lower/recognition/registry.rs",
    find: '    "Vec",\n    "VecDeque",\n',
    replace: '    "VecDeque",\n',
    tests: ["test/std-coverage.test.ts"],
  },
  {
    name: "by-path-through-any-child",
    breaks: "a path's part is any child of that name, a module or not, so a `#![no_std]` crate's walk asks rustc for a type's children, and rustc panics",
    file: "src/lower/recognition/registry.rs",
    find: "            .find(|&id| last || tcx.def_kind(id) == rustc_hir::def::DefKind::Mod)?;\n",
    replace: "            .find(|_| true)?;\n",
    tests: ["test/crates.test.ts", "-t", "no_std"],
  },
];

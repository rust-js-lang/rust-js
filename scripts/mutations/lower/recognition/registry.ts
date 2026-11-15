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
    find: "        let (DefKind::Mod, module) = at else {\n            return None;\n        };\n",
    replace: "        let module = at.1;\n",
    tests: ["test/crates.test.ts", "-t", "no_std"],
  },
  {
    name: "by-path-last-a-module",
    breaks: "a path's last part is the first child of that name, a module where it has the item's name too: `std::future::ready` is `core::future`'s module `ready`, not the function it re-exports, so `future::ready(v)` is no promise's",
    file: "src/lower/recognition/registry.rs",
    find: "                (named.iter().find(|(kind, _)| *kind != DefKind::Mod).or(named.first())).copied()?\n",
    replace: "                named.first().copied()?\n",
    tests: ["test/corpus.test.ts", "-t", "^future_held_where_nothing_shows.rs$"],
  },
];

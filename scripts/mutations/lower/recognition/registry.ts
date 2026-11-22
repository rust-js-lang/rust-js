// Mutations of crates/rust-js-lower/src/recognition/registry.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "registry-without-vec",
    breaks:
      "`Vec` isn't one of std's data structures counted, so its methods leave the ratchet unseen",
    file: "crates/rust-js-lower/src/recognition/registry.rs",
    find: '    "Vec",\n    "VecDeque",\n',
    replace: '    "VecDeque",\n',
    tests: ["test/std-coverage.test.ts"],
  },
  {
    name: "by-path-through-any-item",
    breaks: "a path's part but the last is the first child of its name, a macro where a module has its name too, std's `vec!` for its module `vec`, so `std::vec::Vec`'s row resolves to nothing, an internal error every compile",
    file: "crates/rust-js-lower/src/recognition/registry.rs",
    find: "            .find(|(kind, _)| *kind == DefKind::Mod)?\n",
    replace: "            .next()?\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "by-path-last-a-module",
    breaks: "a path's last part is the first child of that name, a module where it has the item's name too: `std::future::ready` is `core::future`'s module `ready`, not the function it re-exports, so `future::ready(v)` is no promise's",
    file: "crates/rust-js-lower/src/recognition/registry.rs",
    find: "    (named.iter().find(|(kind, _)| *kind != DefKind::Mod).or(named.first())).copied()\n",
    replace: "    named.first().copied()\n",
    tests: ["test/corpus.test.ts", "-t", "^future_held_where_nothing_shows.rs$"],
  },
];

// Mutations of crates/rust-js-lower/src/recognition/compiled_std/cloned_at.rs (ADR 0427): where std's clones raise a borrowed `RefCell`'s panic (#2509).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cloned-at-oracle-unread",
    breaks: "a std type whose clone reaches a `RefCell` by several paths in std's MIR, a `VecDeque`'s, isn't settled by the native oracle, so `bun run std` fails (#2509)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/cloned_at.rs",
    find: "        (Some(_), Some(_)) => Some(cloned_at_oracle(&key, position).ok_or_else(|| {\n",
    replace: "        (Some(_), Some(_)) => Some(None::<String>.ok_or_else(|| {\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "cloned-at-defaults-unused",
    breaks: "the clone walk gives a `Vec`'s allocator a `u8`, not its default, so its clone resolves no instance and has no row (#2509)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/cloned_at.rs",
    find: "                && let ty::GenericParamDefKind::Type { has_default: true, .. } = adt_generics.param_at(each, tcx).kind\n",
    replace: "                && let ty::GenericParamDefKind::Type { has_default: false, .. } = adt_generics.param_at(each, tcx).kind\n",
    tests: ["test/std-sites.test.ts"],
  },
  {
    name: "cloned-at-key-shapes-unsaid",
    breaks: "`Box<[T]>`'s clone is keyed as `Box<T>`'s, so `bun run std` finds two impls of one key (#2509)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/cloned_at.rs",
    find: "            \"[_]\".to_string()\n",
    replace: "            \"_\".to_string()\n",
    tests: ["test/std-sites.test.ts"],
  },
];

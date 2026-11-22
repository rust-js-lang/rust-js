// Mutations of src/lower/recognition/compiled_std/stored.rs (ADR 0093): the
// catalogue of std's bodies `@rust-js/std` compiles, kept with std and read
// where its crates are the compile's (#2305).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "catalogue-crates-unchecked",
    breaks: "a compile reads the kept catalogue whatever its std's crates are: std's own build, of a `core` built to unwind, maps hashes of crates it hasn't loaded (#2305)",
    file: "src/lower/recognition/compiled_std/stored.rs",
    find: "    if !(catalogue.crates.iter()).all(|(name, hash)| loaded.get(name) == Some(hash)) {\n",
    replace: "    if false {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "catalogue-groups-unread",
    breaks: "a build that shows a `MEANING_ONLY` adapter, a `Successors`' `{:?}`, isn't given that adapter's bodies, so its JS calls none @rust-js/std has (#2305)",
    file: "src/lower/recognition/compiled_std/stored.rs",
    find: "    for &adapter in MEANING_ONLY.iter().filter(|&&adapter| called(adapter)) {\n",
    replace: "    for &adapter in MEANING_ONLY.iter().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "adapter_shown_and_cloned_generically|clone_of_a_compiled_adapter_knowing_its_back|clones_beside_a_generic_clone_of_a_copied_map|debug_of_adapters_compiled_as_their_meaning|debug_of_once_and_successors|local_named_as_a_helper_its_scope_reads"],
    snapshots: true,
  },
  {
    name: "catalogue-groups-unkept",
    breaks: "`bun run std` keeps no adapter's group, so a compile that shows one reads none and walks every time, or, of a catalogue kept so, is given none (#2305)",
    file: "src/lower/recognition/compiled_std/stored.rs",
    find: "                named(&mut added.iter().copied().filter(|id| !base.contains(id))),\n",
    replace: "                named(&mut added.iter().copied().filter(|_| false)),\n",
    tests: ["test/compiled-std.test.ts"],
  },
];

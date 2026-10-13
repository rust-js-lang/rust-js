// Mutations of src/names.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "fresh-name-stacks-suffixes",
    breaks: "a name given apart from `caption$1` is `caption$1$1`, which reclaiming can't make `caption`",
    file: "src/names.rs",
    find: "        Some((root, k)) if !root.is_empty() && !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) => root,\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a name given apart twice is reclaimed"],
  },
  {
    name: "dictionary-cache-is-a-helpers-name",
    breaks: "a `Duration` of the crate's own adding is cached in `$durationAdd`, which its module imports too",
    file: "src/names.rs",
    find: "        if !crate::runtime::exports(&cache) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "dictionary_helper_names"],
  },
];

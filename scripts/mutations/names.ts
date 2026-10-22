// Mutations of src/names.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "fresh-name-stacks-suffixes",
    breaks: "a name given apart from `caption$1` is `caption$1$1`, which reclaiming can't make `caption`",
    file: "src/names.rs",
    find: "    let root = apart(base).map_or(base, |(root, _)| root);\n",
    replace: "    let root = base;\n",
    tests: ["test/lowering-names.test.ts", "-t", "a name given apart twice is reclaimed"],
  },
  {
    name: "dictionary-cache-is-a-helpers-name",
    breaks: "a `Duration` of the crate's own adding is cached in `$durationAdd`, which its module imports too",
    file: "src/names.rs",
    find: "        if !crate::runtime::exports(&cache) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "dictionary_helper_names"],
  },
  {
    name: "globals-kept-for-good",
    breaks: "an import named apart from `Error` stays `Error$` where nothing reads the global",
    file: "src/names.rs",
    find: "    KEPT.contains(&name)\n}",
    replace: "    KEPT.contains(&name) || GLOBALS.contains(&name)\n}",
    tests: ["test/lowering-names.test.ts", "-t", "nothing it shadows"],
  },
];

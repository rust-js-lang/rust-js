// Mutations of src/names.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "fresh-name-stacks-suffixes",
    breaks: "a name given apart from `caption$1` is `caption$1$1`, which reclaiming can't make `caption`",
    file: "src/names.rs",
    find: "    let root = apart(base).map_or(base, |(root, _)| root);\n",
    replace: "    let root = base;\n",
    tests: ["test/lowering-mir-lets.test.ts", "-t", "a name given apart twice is reclaimed"],
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
  {
    name: "global-map-unnamed-apart",
    breaks: "a Rust name `Map` keeps it, hiding the global the output reads, `new Map(..)`",
    file: "src/names.rs",
    find: "    \"Map\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^js_global_names.rs$"],
  },
  {
    name: "global-console-unnamed-apart",
    breaks: "a Rust function `console` keeps its name, hiding the global `console.log` reads",
    file: "src/names.rs",
    find: "    \"console\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^js_global_names.rs$"],
  },
  {
    name: "unspelled-never-known",
    breaks: "a name lowering made not spelled yet, `value#1`, is taken for any other name: preparation spells none, and it's printed (#1681)",
    file: "src/names.rs",
    find: "    (!base.is_empty() && !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())).then_some(base)\n",
    replace: "    let _ = (base, k);\n    None\n",
    unitTests: "js::validate::tests::an_unspelled_name_is_refused",
  },
  {
    name: "given-apart-of-an-unspelled-name",
    breaks: "a temporary named after a name lowering made, `value#1`, is given apart from it as written, `value#1$1`, which nothing spells, and the module is refused (#1681)",
    file: "src/names.rs",
    find: "    let base = unspelled_base(base).unwrap_or(base);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nonzero.rs$"],
  },
];

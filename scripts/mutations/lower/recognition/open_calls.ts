// Mutations of src/lower/recognition/open_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "own-reader-names-no-issue",
    breaks: "a call of a crate's reader's method is refused naming no issue, where #1297 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::OwnReader);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a read of a crate's reader"],
  },
  {
    name: "std-hash-evidence-unnamed",
    breaks: "a std type's missing `Hash`, a `Duration`'s, names no issue, where #1447 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "    (is_std_def(tcx, trait_id, StdItem::Hash) && std_type).then_some(OpenCall::StdHash)\n",
    replace: "    (is_std_def(tcx, trait_id, StdItem::Hash) && std_type && false).then_some(OpenCall::StdHash)\n",
    tests: ["test/diagnostics.test.ts", "-t", "a set of a Duration beside"],
  },
];

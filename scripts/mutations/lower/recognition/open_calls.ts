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
    name: "saturating-call-unnamed",
    breaks: "a call on a `Saturating` names no issue, where #1434 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::Saturating);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a Saturating's arithmetic"],
  },
];

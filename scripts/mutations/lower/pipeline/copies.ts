// Mutations of src/lower/pipeline/copies.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-given-copy-kept",
    breaks: "a copy made alike its function is kept, so `echo` is two functions, `echoBoxed` beside it",
    file: "src/lower/pipeline/copies.rs",
    find: "            if let Some((to, _)) = found {",
    replace: "            if let Some((to, _)) = found.filter(|_| false) {",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
    snapshots: true,
  },
];

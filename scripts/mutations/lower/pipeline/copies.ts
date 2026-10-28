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
  {
    name: "js-given-copy-self-call-apart",
    breaks: "a copy that calls itself is read as calling a copy, never alike its function, so `count` is two functions, `countUnboxed` beside it",
    file: "src/lower/pipeline/copies.rs",
    find: "                as_if.insert(named(*id, Some(*at)), named(*id, settled(&same, *id, *to)).2);",
    replace: "                let _ = to;",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
    snapshots: true,
  },
];

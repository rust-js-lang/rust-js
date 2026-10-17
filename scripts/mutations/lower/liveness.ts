// Mutations of src/lower/liveness.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const binding = ["test/jsx.test.ts", "-t", "a binding needs no allow"];

export const mutations: Mutation[] = [
  {
    name: "js-read-fields-dead",
    breaks: "a struct given a binding has its fields said never read, as rustc alone sees it, so each needs `#[allow(dead_code)]`",
    file: "src/lower/liveness.rs",
    find: "    summary.final_result.live_symbols.extend(read);\n",
    replace: "",
    tests: binding,
  },
  {
    name: "js-read-nested-struct-dead",
    breaks: "a struct in a field of one given a binding has its fields said never read",
    file: "src/lower/liveness.rs",
    find: "            structs.extend(local_structs(ty));\n",
    replace: "            let _ = ty;\n",
    tests: binding,
  },
  {
    name: "js-read-of-any-call",
    breaks: "a struct given any function, not only a binding, has its fields counted read, so a field nothing reads is never flagged",
    file: "src/lower/liveness.rs",
    find: "        if callee.is_some_and(|callee| super::bindings::is_binding(self.tcx, callee)) {\n",
    replace: "        if callee.is_some() {\n",
    tests: binding,
  },
];

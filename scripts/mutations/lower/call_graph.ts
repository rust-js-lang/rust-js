// Mutations of src/lower/call_graph.rs, the crate's one call graph from MIR (ADR 1137, #1671 change 4 M3).
import type { Mutation } from "../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "call-graph-calls-unresolved",
    breaks: "no call is resolved to what it calls, so no function only calls reach is seen called and its parameters are given nothing",
    file: "src/lower/call_graph.rs",
    find: "                        Callee::Resolved(instance.def_id())\n",
    replace: "                        Callee::Unknown\n",
    tests,
  },
];

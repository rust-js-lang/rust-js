// Mutations of src/lower/call_graph.rs, the crate's one call graph from MIR and each body's names (ADR 1137, #1671 change 4 M3).
import type { Mutation } from "../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-constants-call-nothing",
    breaks: "a constant's MIR isn't one of its `const fn`s' callers, so `add`, only `const SUM` calls, is given nothing and `a + b` keeps its wrap, left to the bridge (#1742)",
    file: "src/lower/call_graph.rs",
    find: "            callers.insert(owner, tcx.mir_for_ctfe(owner));\n",
    replace: "            let _ = owner;\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "a const fn constants and patterns call"],
  },
  {
    name: "call-graph-calls-unresolved",
    breaks: "no call is resolved to what it calls, so no function only calls reach is seen called and its parameters are given nothing",
    file: "src/lower/call_graph.rs",
    find: "                        Callee::Resolved(instance.def_id())\n",
    replace: "                        Callee::Unknown\n",
    tests,
  },
  {
    name: "closure-constant-functions-uncalled",
    breaks: "a function a closure constant holds, std's `Iterator::last`'s `some` in `FlattenCompat`'s inlined `last` of `Option`s, is given no drop of its type parameter, so `flatten().last()` of `Some`s never drops the one it passes (#645)",
    file: "src/lower/call_graph.rs",
    find: "            } else if constant.ty().is_closure() {\n",
    replace: "            } else if false {\n",
    tests: ["test/std-package.test.ts"],
  },
];

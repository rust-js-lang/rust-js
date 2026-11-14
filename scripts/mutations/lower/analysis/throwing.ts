// Mutations of src/lower/analysis/throwing.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const tests = ["test/throwing-calls.test.ts", "-t", "is a throwing function"];

export const mutations: Mutation[] = [
  {
    name: "throwing-fn-before-its-callees",
    breaks: "the call graph's components are walked callers first, so a function whose result is another throwing function's call is asked before that one is found, and isn't one, its `$try` kept (#1671 change 4 M3)",
    file: "src/lower/analysis/throwing.rs",
    find: "    for component in &calls.graph.sccs {\n",
    replace: "    for component in calls.graph.sccs.iter().rev() {\n",
    tests,
  },
  {
    name: "throwing-fn-call-uncaught",
    breaks:
      "a throwing function's call isn't a throwing call: `testStringCoercion(value).is_err()` is a `TAG` of what it threw",
    file: "src/lower/analysis/throwing.rs",
    find: "    } else if throwing.contains(&id) {\n",
    replace: "    } else if false {\n",
    tests,
  },
  {
    name: "throwing-fn-of-its-own-ok",
    breaks:
      "a function that makes its own `Ok` too, `or_none`, is taken as throwing, its `Ok(None)` thrown on",
    file: "src/lower/analysis/throwing.rs",
    find: "        if (block.statements.iter()).any(|s| matches!(&s.kind, StatementKind::Assign(assign) if written(&assign.0))) {\n",
    replace: "        if false {\n",
    tests,
  },
  {
    name: "throwing-fn-any-call",
    breaks: "a function returning any call's `Result` is taken as throwing",
    file: "src/lower/analysis/throwing.rs",
    find: ".is_some_and(|(id, args)| matches!(call_throws(recognition, throwing, id, args), Catching::Result));\n",
    replace: ".is_some_and(|_| true);\n",
    tests,
  },
  {
    name: "throwing-fn-named-as-a-value",
    breaks: "a private function whose result is a binding's `Result`, named as a value, is a throwing function, so a call through a `fn` pointer reads its plain value as a `Result` and lets its throw go (#1670)",
    file: "src/lower/analysis/throwing.rs",
    find: "    let candidate = |id: &DefId| only_called.contains_key(id) && candidate(tcx, recognition, *id);\n",
    replace: "    let candidate = |id: &DefId| candidate(tcx, recognition, *id);\n",
    tests: ["test/throwing-calls.test.ts", "-t", "a function whose result is a throwing call's is a throwing function"],
  },
];

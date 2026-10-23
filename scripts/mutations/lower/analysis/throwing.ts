// Mutations of src/lower/analysis/throwing.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const tests = ["test/js-builtins.test.ts", "-t", "is a throwing function"];

export const mutations: Mutation[] = [
  {
    name: "throwing-fn-call-uncaught",
    breaks: "a throwing function's call isn't a throwing call: `testStringCoercion(value).is_err()` is a `TAG` of what it threw",
    file: "src/lower/analysis/throwing.rs",
    find: "    } else if throwing.contains(&id) {\n",
    replace: "    } else if false {\n",
    tests,
  },
  {
    name: "throwing-fn-exported",
    breaks: "a `pub` function JS calls, `coerced`, throws where its JS callers read a `Result`",
    file: "src/lower/analysis/throwing.rs",
    find: "        && !tcx.visibility(local).is_public()\n",
    replace: "",
    tests,
  },
  {
    name: "throwing-fn-of-its-own-ok",
    breaks: "a function that makes its own `Ok` too, `or_none`, is taken as throwing, its `Ok(None)` thrown on",
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
];

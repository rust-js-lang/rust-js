// Mutations of src/prepare.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "handler-of-one-call-blocked",
    breaks: "a handler of one call is a block, `() => { setCount(1); }`, where React ignores what it returns",
    file: "src/prepare.rs",
    find: "                            *body = vec![StmtKind::Return(Some(e.clone())).at(*span)];\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "try-kept-as-helper",
    breaks: "a `match` of what a JS call threw is `$try` and its `TAG`, not JS's `try`",
    file: "src/prepare.rs",
    find: "    try_catches(body);\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "a JS error is shown"],
  },
  {
    name: "try-with-error-read",
    breaks: "an `Err` that reads the error is a `catch` with none, `match` undefined there",
    file: "src/prepare.rs",
    find: "        || js::mentions_in(failed, result) > 0\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "a JS error is shown"],
  },
  {
    name: "try-around-more",
    breaks: "an `Ok` that gives something else, `\"parsed\"`, is `return JSON.parse(text)` in the `try`",
    file: "src/prepare.rs",
    find: "        StmtKind::Return(Some(e)) if of_result(e, \"_0\") => StmtKind::Return(Some(call.clone())),\n",
    replace: "        StmtKind::Return(Some(_)) => StmtKind::Return(Some(call.clone())),\n",
    tests: ["test/compiler.test.ts", "-t", "a JS error is shown"],
  },
  {
    name: "coalescing-if",
    breaks: "`if (SEEN == null) { SEEN = n; }`, not `SEEN ??= n`",
    file: "src/prepare.rs",
    find: "        if let Some(coalesced) = coalescing(stmt) {\n",
    replace: "        if let Some(coalesced) = coalescing(stmt).filter(|_| false) {\n",
    tests: ["test/compiler.test.ts", "-t", "only read and set is the module's let"],
  },
  {
    name: "coalescing-another",
    breaks: "`if (SEEN == null) { other = n; }` is `other ??= n`, set where `SEEN` isn't none",
    file: "src/prepare.rs",
    find: "    if !matches!(null.kind, ExprKind::Null) || !js::same_path(tested, target) {\n",
    replace: "    if !matches!(null.kind, ExprKind::Null) {\n",
    tests: ["test/compiler.test.ts", "-t", "only read and set is the module's let"],
  },
];

// Mutations of src/js/make/unwinding.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "uncaught-settle-kept",
    breaks: "`js::settle(p).await.unwrap()` is `$unwrapOk(await $settle(p))`, not `await p`",
    file: "src/js/make/unwinding.rs",
    find: "            (Some(\"$settle\"), [promise]) => Some(Expr {\n",
    replace: "            (Some(\"$settled\"), [promise]) => Some(Expr {\n",
    tests: ["test/throwing-calls.test.ts", "-t", "unwrap of a throwing JS call"],
  },
  {
    name: "caught-unwind-not-tried",
    breaks: "a `catch_unwind`'s `Result` matched is `$catchUnwind` and its `TAG`, not `try` and `catch`",
    file: "src/js/make/unwinding.rs",
    find: "        let (Some(\"$catchUnwind\"), [f]) = (callee.helper_name(), args.as_slice()) else {\n",
    replace: "        let (Some(\"$catchUnwindNot\"), [f]) = (callee.helper_name(), args.as_slice()) else {\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "getIODescription"],
  },
  {
    name: "caught-maker-tried",
    breaks: "`catch_unwind(made(n))` catches a panic of `made(n)`, which Rust makes before it catches anything",
    file: "src/js/make/unwinding.rs",
    find: "        if f.may_throw() {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/lowering-unwinds.test.ts", "-t", "resume_unwind throws what unwound"],
  },
  {
    name: "panic-of-nothing-written",
    breaks: "`panic!(\"\")` is `new Error(\"\")`, not `new Error()`",
    file: "src/js/make/unwinding.rs",
    find: "            ExprKind::Str(text) if text.is_empty() => Vec::new(),\n",
    replace: "",
    tests: ["test/lowering-unwinds.test.ts", "-t", "a port throws an Error by panic!"],
  },
  {
    name: "thrown-error-unrecognized",
    breaks: "a downcast of an unwind's payload to `&JsError`, `payload.error`, isn't known as what was thrown",
    file: "src/js/make/unwinding.rs",
    find: "            ExprKind::Member(pair, field) if field == THROWN_VALUE => Some(pair),\n",
    replace: "            ExprKind::Member(pair, field) if false && field == THROWN_VALUE => Some(pair),\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
];

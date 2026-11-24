// Mutations of crates/rust-js-program/src/js/make/unwinding.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "uncaught-settle-kept",
    breaks: "`js::settle(p).await.unwrap()` is `$unwrapOk(await $settle(p))`, not `await p`",
    file: "crates/rust-js-program/src/js/make/unwinding.rs",
    find: "        let promise = settled.helper_call_of(\"$settle\")?.arg(\"promise\")?;\n",
    replace: "        let promise = settled.helper_call_of(\"$settled\")?.arg(\"promise\")?;\n",
    tests: ["test/throwing-calls.test.ts", "-t", "unwrap of a throwing JS call"],
  },
  {
    name: "caught-unwind-not-tried",
    breaks: "a `catch_unwind`'s `Result` matched is `$catchUnwind` and its `TAG`, not `try` and `catch`",
    file: "crates/rust-js-program/src/js/make/unwinding.rs",
    find: "        let f = self.helper_call_of(\"$catchUnwind\")?.arg(\"f\")?;\n",
    replace: "        let f = self.helper_call_of(\"$catchUnwindNot\")?.arg(\"f\")?;\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "getIODescription"],
  },
  {
    name: "caught-maker-tried",
    breaks: "`catch_unwind(made(n))` catches a panic of `made(n)`, which Rust makes before it catches anything",
    file: "crates/rust-js-program/src/js/make/unwinding.rs",
    find: "        if f.may_throw() {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/lowering-unwinds.test.ts", "-t", "resume_unwind throws what unwound"],
  },
  {
    name: "thrown-error-unrecognized",
    breaks: "a downcast of an unwind's payload to `&JsError`, `payload.error`, isn't known as what was thrown",
    file: "crates/rust-js-program/src/js/make/unwinding.rs",
    find: "            ExprKind::Member(pair, field) if field == THROWN_VALUE => Some(pair),\n",
    replace: "            ExprKind::Member(pair, field) if false && field == THROWN_VALUE => Some(pair),\n",
    tests: ["test/lowering-unwind-downcasts.test.ts", "-t", "every unwind"],
  },
];

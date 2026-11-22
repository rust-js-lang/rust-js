// Mutations of crates/rust-js-lower/src/std_types/backtrace.rs (ADR 0568).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "backtrace-captured",
    breaks:
      "`Backtrace::capture()` captures JS's stack, where Rust's is disabled unless the environment asks",
    file: "crates/rust-js-lower/src/std_types/backtrace.rs",
    find: "            BacktraceOp::Capture | BacktraceOp::Disabled => Expr::js_undefined(),\n",
    replace:
      '            BacktraceOp::Disabled => Expr::js_undefined(),\n            BacktraceOp::Capture => Expr::call_helper("$backtrace", Vec::new()),\n',
    tests: ["test/corpus.test.ts", "-t", "^backtrace.rs$"],
  },
];

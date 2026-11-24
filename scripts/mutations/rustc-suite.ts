// Mutations of scripts/rustc-suite.ts, rustc's run-pass tests run as JS
// against native Rust (ADRs 0089, 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "rustc-run-refusal-taken-for-a-crash",
    breaks: "a test whose run ends by `$unsupported`, uncaught, is said by the stack's first line, so a listed rejection reads as wrong, worse than listed (#2447's first CI run)",
    file: "scripts/rustc-suite.ts",
    find: '  if (refused !== undefined) return `ended {"error":"Error: ${refused.slice(0, 200)}"}`;\n',
    replace: "",
    tests: ["test/rustc-suite.test.ts", "-t", "a failure is rejected, crashed or wrong"],
  },
  {
    name: "rustc-run-refusal-of-any-status",
    breaks: "a run that writes rust-js's refusal line but exits otherwise than `$main`'s 1, a panic's 101, is said to be a rejection",
    file: "scripts/rustc-suite.ts",
    find: "  const refused = code === 1 ? ",
    replace: "  const refused = code !== null ? ",
    tests: ["test/rustc-suite.test.ts", "-t", "a failure is rejected, crashed or wrong"],
  },
];

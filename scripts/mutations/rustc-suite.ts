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
];

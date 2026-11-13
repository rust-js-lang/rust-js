// Mutations of src/lower/display/sink_debug.rs (ADR 1135, #1652).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sink-debug-list-never-pretty",
    breaks: "a sequence's `{:#?}` written into a writer isn't pretty, `[1, 2]` (#1652)",
    file: "src/lower/display/sink_debug.rs",
    find: '            return helper("$writeDebugList", vec![value, write, pretty.clone()]);\n',
    replace: '            return helper("$writeDebugList", vec![value, write, Expr::bool(false)]);\n',
    tests: ["test/writer-sink.test.ts"],
  },
];

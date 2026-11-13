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
  {
    name: "sink-map-set-debug-unfollowed",
    breaks: "a map's or a set's `Debug` written into a writer is refused, where std writes it by `debug_map` or `debug_set`",
    file: "src/lower/display/sink_debug.rs",
    find: "            && (self.is_set(ty) || self.is_map(ty))\n",
    replace: "            && false\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of the rest's writes is native Rust's, in the plain build"],
  },
];

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
  {
    name: "sink-cell-debug-refused",
    breaks: "a `Cell`'s or a `RefCell`'s `Debug` written into a writer is refused, where std writes it by `debug_struct`",
    file: "src/lower/display/sink_debug.rs",
    find: "            && (cell(crate::lower::recognition::StdItem::Cell) || cell(crate::lower::recognition::StdItem::RefCell))\n",
    replace: "            && false\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of the rest's writes is native Rust's, in the plain build"],
  },
  {
    name: "sink-layout-debug-refused",
    breaks: "a `Layout`'s `Debug` in a framed writer, smallvec's derived `Debug` of a `&Layout`, is refused",
    file: "src/lower/display/sink_debug.rs",
    find: "        if let Some(layout) = self.recognition().layout(ty) {\n",
    replace: "        if let Some(layout) = self.recognition().layout(ty).filter(|_| false) {\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of the rest's writes is native Rust's, in the plain build"],
  },
];

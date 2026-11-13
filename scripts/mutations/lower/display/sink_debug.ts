// Mutations of src/lower/display/sink_debug.rs (ADR 1135, #1652).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sink-debug-list-never-pretty",
    breaks: "a sequence's `{:#?}` written into a writer isn't pretty, `[1, 2]` (#1652)",
    file: "src/lower/display/sink_debug.rs",
    find: '            return helper("$writeDebugList", vec![value, write, pretty.clone()]);\n',
    replace: '            return helper("$writeDebugList", vec![value, write, Expr::bool(false)]);\n',
    tests: ["test/writer-sink.test.ts", "test/corpus.test.ts", "-t", "^writer_sink_|^(?!.*\\.rs$)"],
  },
  {
    name: "sink-map-set-debug-unfollowed",
    breaks: "a map's or a set's `Debug` written into a writer is refused, where std writes it by `debug_map` or `debug_set`",
    file: "src/lower/display/sink_debug.rs",
    find: "            && (self.is_set(ty) || self.is_map(ty))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
  {
    name: "sink-cell-debug-refused",
    breaks: "a `Cell`'s or a `RefCell`'s `Debug` written into a writer is refused, where std writes it by `debug_struct`",
    file: "src/lower/display/sink_debug.rs",
    find: "            && (cell(crate::lower::recognition::StdItem::Cell) || cell(crate::lower::recognition::StdItem::RefCell))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
  {
    name: "sink-layout-debug-refused",
    breaks: "a `Layout`'s `Debug` in a framed writer, smallvec's derived `Debug` of a `&Layout`, is refused",
    file: "src/lower/display/sink_debug.rs",
    find: "        if let Some(layout) = self.recognition().layout(ty) {\n",
    replace: "        if let Some(layout) = self.recognition().layout(ty).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
  {
    name: "sink-parse-error-debug-refused",
    breaks: "a parse error's `Debug` in a framed writer, thiserror's derived `Debug` of a `&ParseIntError`, is refused",
    file: "src/lower/display/sink_debug.rs",
    find: "        if self.recognition().is_parse_error(ty)\n",
    replace: "        if self.recognition().is_parse_error(ty) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_rest.rs$"],
  },
];

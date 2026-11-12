// Mutations of src/runtime/debug_builders.js (ADR 0093): std's `Debug`
// builders, written step by step through a writer's sink (#1533, #1557).
import type { Mutation } from "../../mutations";

const grids = ["test/corpus.test.ts", "-t", "^(printing_builder_written_step_by_step|failing_debug_part_keeps_what_was_written)\\.rs$"];

export const mutations: Mutation[] = [
  {
    name: "builder-step-after-error-taken",
    breaks: "a builder's step after one that gave `Err` is taken, its value shown and written, where std's `and_then` skips it",
    file: "src/runtime/debug_builders.js",
    find: "      if (builder.result !== undefined) return;\n",
    replace: "",
    tests: grids,
  },
  {
    name: "builder-entries-stop-stepping-after-error",
    breaks: "`entries` stops stepping its iterator once a step gave `Err`, so what its steps print after it is lost, where std's `for entry in entries` steps on",
    file: "src/runtime/debug_builders.js",
    find: "    entries(items, show, drop) {\n      $eachOf(items, (item) => {\n",
    replace: "    entries(items, show, drop) {\n      $eachOf(items, (item) => {\n        if (inner.result !== undefined) throw inner.result;\n",
    tests: grids,
  },
  {
    name: "builder-separator-before-first",
    breaks: "a list's first entry is written after `, `, as if it had fields before it",
    file: "src/runtime/debug_builders.js",
    find: "        if (inner.fields > 0) f.write(\", \");\n",
    replace: "        if (inner.fields >= 0) f.write(\", \");\n",
    tests: grids,
  },
  {
    name: "builder-pretty-unpadded",
    breaks: "`{:#?}`'s parts of a streamed builder are written without std's `PadAdapter`'s indent",
    file: "src/runtime/debug_builders.js",
    find: "      const written = (text) => f.write(pretty ? $padded(text, adapter) : text);\n",
    replace: "      const written = (text) => f.write(text);\n",
    tests: grids,
  },
  {
    name: "builder-adapter-state-fresh",
    breaks: "a field's name and its value go through fresh adapters, so a value of many lines is indented at its first line too, `bad:     B`",
    file: "src/runtime/debug_builders.js",
    find: "      const written = (text) => f.write(pretty ? $padded(text, adapter) : text);\n",
    replace: "      const written = (text) => f.write(pretty ? $padded(text) : text);\n",
    tests: grids,
  },
  {
    name: "builder-error-partial-lost",
    breaks: "what a part wrote before its `Err` isn't written, `Holder { n: 1, bad: ` without its `B`",
    file: "src/runtime/debug_builders.js",
    find: "        if (error.written) written(error.written);\n",
    replace: "",
    tests: grids,
  },
  {
    name: "builder-panicked-part-unpadded",
    breaks: "what a part wrote before its panic is folded into the line as it is, not through `{:#?}`'s adapter, `<3>!Grid(` where std writes `<3>!    Grid(`",
    file: "src/runtime/debug_builders.js",
    find: "          const unfinished = $unfinishedAbove(depth);\n          if (unfinished !== \"\") written(unfinished);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^printing_derived_debug_stopped_part_way\\.rs$"],
  },
  {
    name: "builder-unnamed-tuple-without-comma",
    breaks: "a tuple builder of no name and one field is written `(1)`, not std's `(1,)`",
    file: "src/runtime/debug_builders.js",
    find: "        if (tuple.fields === 1 && name === \"\" && !pretty) f.write(\",\");\n",
    replace: "",
    tests: grids,
  },
  {
    name: "builder-non-exhaustive-empty-spaced",
    breaks: "a struct builder of no field's `finish_non_exhaustive` is `Rest {..}`, not std's `Rest { .. }`",
    file: "src/runtime/debug_builders.js",
    find: "        if (struct.fields === 0) f.write(\" { .. }\");\n",
    replace: "        if (struct.fields === 0) f.write(\" {..}\");\n",
    tests: grids,
  },
  {
    name: "composed-partial-lost",
    breaks: "a std value's `Debug` through std's builders loses what it wrote before a part's `Err`, `B1` where native Rust keeps `Some(B1` (#1561)",
    file: "src/runtime/debug_builders.js",
    find: "    if (error instanceof $FmtError) throw $fmtWritten(error, f.done());\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^failing_std_value_debug_keeps_what_was_written.rs$"],
  },
  {
    name: "pretty-part-written-ahead-unpadded",
    breaks: "a `{:#?}` part's line written ahead of a write to standard error isn't through its adapter, so its text, padded, doesn't start with what was written (#1490)",
    file: "src/runtime/debug_builders.js",
    find: "      if (pretty) $padAt.set(depth, adapter);\n",
    replace: "",
    tests: [
      "test/stream-order.test.ts",
      "-t",
      "stderr_between"
    ]
  },
];

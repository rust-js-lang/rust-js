// Mutations of src/runtime/string_writer.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mut-writer-not-through",
    breaks: "`$mutWriter` gives its writer the box a `&mut` to it is given in, not what's in it",
    file: "src/runtime/string_writer.js",
    find: "    writeFmt: (w, text) => writer.writeFmt(w.value, text),\n",
    replace: "    writeFmt: (w, text) => writer.writeFmt(w, text),\n",
    tests: ["test/corpus.test.ts", "-t", "^(generic_writers|mut_given_as_type_param).rs$"],
  },
  {
    name: "mut-writer-writes-no-bytes",
    breaks: "`$mutWriter` of an `io::Write` has no `writeAll`, so generic code given `&mut Vec<u8>` can't write to it",
    file: "src/runtime/string_writer.js",
    find: "    writeAll: (w, bytes) => writer.writeAll(w.value, bytes),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_write_mut_generic.rs$"],
  },
  {
    name: "string-writer-args-called-twice",
    breaks: "a `String` given generic code's `write!` makes its text twice, so a value's `Display` runs twice, `shown 1` printed again",
    file: "src/runtime/string_writer.js",
    find: "    else w.value += $fmtTextOr(args, (text) => (w.value += text));\n",
    replace: "    else w.value += $fmtTextOr(() => (args(), args()), (text) => (w.value += text));\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of Arguments given on's writes is native Rust's, in the plain build"],
  },
  {
    name: "string-writer-sink-written-whole",
    breaks: "a `Formatter` given as a `W: fmt::Write`, of a writer shown into a writer, writes a `write!`'s text whole, one `write_str`",
    file: "src/runtime/string_writer.js",
    find: "    else if (w.writeChar !== undefined) $writingInto(w, args);\n",
    replace: "",
    tests: ["test/writer-sink.test.ts", "-t", "each of own write_fmt's through generic code's writes is native Rust's, in the plain build"],
  },
  {
    name: "string-writer-sink-char-as-str",
    breaks: "a `Formatter` given as a `W: fmt::Write`, of a writer shown into a writer, gives a `char` to its writer's `write_str`, not its own `write_char`",
    file: "src/runtime/string_writer.js",
    find: "    if (w.writeChar !== undefined) w.writeChar(c);\n    else w.value += c;\n",
    replace: "    w.value += c;\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of own write_fmt's through generic code's writes is native Rust's, in the plain build"],
  },
  {
    name: "string-writer-partial-lost",
    breaks: "a `String` given generic code's `write!` of a value that fails loses what was written before it, `\"\"` where native Rust has `\"x1y\"` (#1696)",
    file: "src/runtime/string_writer.js",
    find: "    else w.value += $fmtTextOr(args, (text) => (w.value += text));\n",
    replace: "    else w.value += $fmtTextOr(args, () => {});\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of Arguments given on's writes is native Rust's, in the plain build"],
  },
];

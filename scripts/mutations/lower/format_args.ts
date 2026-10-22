// Mutations of src/lower/format_args.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "format-lines-continuation-counted",
    breaks: "a line a `\\` continues is taken as written across, so a `\\n` written so is a line break",
    file: "src/lower/format_args.rs",
    find: "                    return text.match_indices('\\n').any(|(i, _)| !text[..i].ends_with('\\\\'));",
    replace: "                    return text.contains('\\n');",
    tests: ["test/lowering-strings.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "format-lines-raw-unseen",
    breaks: "a raw format string written across lines is a template of `\\n`s on one line",
    file: "src/lower/format_args.rs",
    find: "                    return text.contains('\\n');\n",
    replace: "                    return false;\n",
    tests: ["test/lowering-strings.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "format-lines-println-dropped",
    breaks: "a `println!` written across lines is a template of `\\n`s on one line",
    file: "src/lower/format_args.rs",
    find: "            Ok(Expr::template(texts, values.clone()).with_lines(*lines))",
    replace: "            Ok(Expr::template(texts, values.clone()).with_lines(false))",
    tests: ["test/lowering-strings.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "begin-panic-payload",
    breaks: "`panic!(5)` before edition 2021 throws a message Rust never shows",
    file: "src/lower/format_args.rs",
    find: "                if !text {",
    replace: "                if false && !text {",
    tests: ["test/corpus.test.ts", "-t", "begin_panic_value"],
  },
  {
    name: "stream-write-count-bare",
    breaks: "a stream's `write(bytes)` is the count, not `Ok` of it",
    file: "src/lower/format_args.rs",
    find: "                    StreamOp::Written { .. } => super::std_types::result::ok(written),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
    snapshots: true,
  },
  {
    name: "unreachable-hint-falls-through",
    breaks: "`hint::unreachable_unchecked()` throws nothing: what follows runs",
    file: "src/lower/format_args.rs",
    find: "            Std::Unreachable => {\n                out.push(self.unreachable_code(span));\n",
    replace: "            Std::Unreachable => {\n",
    tests: ["test/corpus.test.ts", "-t", "^unreachable_hint.rs$"],
    snapshots: true,
  },
  {
    name: "unreachable-display-unprefixed",
    breaks: "an edition 2018 `unreachable!(x)` panics with what `x` shows alone, not after std's words",
    file: "src/lower/format_args.rs",
    find: "                    true => join(vec![Expr::str(\"internal error: entered unreachable code: \"), shown]),\n",
    replace: "                    true => shown,\n",
    tests: ["test/corpus.test.ts", "-t", "^unreachable_display.rs$"],
  },
];

// Mutations of src/lower/format_args.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "format-lines-continuation-counted",
    breaks: "a line a `\\` continues is taken as written across, so a `\\n` written so is a line break",
    file: "src/lower/format_args.rs",
    find: "                    return text.match_indices('\\n').any(|(i, _)| !text[..i].ends_with('\\\\'));",
    replace: "                    return text.contains('\\n');",
    tests: ["test/lowering.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "format-lines-raw-unseen",
    breaks: "a raw format string written across lines is a template of `\\n`s on one line",
    file: "src/lower/format_args.rs",
    find: "                    return text.contains('\\n');\n",
    replace: "                    return false;\n",
    tests: ["test/lowering.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "format-lines-println-dropped",
    breaks: "a `println!` written across lines is a template of `\\n`s on one line",
    file: "src/lower/format_args.rs",
    find: "            Ok(Expr::template(texts, values.clone()).with_lines(*lines))",
    replace: "            Ok(Expr::template(texts, values.clone()).with_lines(false))",
    tests: ["test/lowering.test.ts", "-t", "a format string written across lines"],
  },
  {
    name: "begin-panic-payload",
    breaks: "`panic!(5)` before edition 2021 throws a message Rust never shows",
    file: "src/lower/format_args.rs",
    find: "                if !text {",
    replace: "                if false && !text {",
    tests: ["test/corpus.test.ts", "-t", "begin_panic_value"],
  },
];

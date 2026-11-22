// Mutations of src/lower/mir/std_calls/fmt_parts.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-format-width-dropped",
    breaks: "`{:5?}` from MIR pads nothing",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                        None => spec.width.map(|w| Expr::int(w.into())),\n",
    replace: "                        None => None,\n",
    tests: ["test/corpus.test.ts", "-t", "debug_options"],
  },
  {
    name: "mir-format-precision-dropped",
    breaks: "`{:.1?}` from MIR shows every digit",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                        None => spec.precision.map(|p| Expr::int(p.into())),\n",
    replace: "                        None => None,\n",
    tests: ["test/corpus.test.ts", "-t", "debug_options"],
  },
  {
    name: "mir-format-width-argument-dropped",
    breaks: "`{:>w$}` from MIR pads nothing",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                    let width = match spec.width_from {\n                        Some(from) => Some(item(from).ok_or_else(bad)?.2),\n",
    replace:
      "                    let width = match spec.width_from {\n                        Some(_) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "format_width_arguments"],
  },
  {
    name: "mir-format-pieces-unwritten",
    breaks:
      "a format string's pieces, `concat!`'s, are each formatted, but only the first is written",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "            .map(|(template, lines)| Ok((self.formatted_parts(items, template, &mut arguments, span)?, *lines)))\n",
    replace:
      "            .take(1)\n            .map(|(template, lines)| Ok((self.formatted_parts(items, template, &mut arguments, span)?, *lines)))\n",
    tests: [
      "test/lowering-concat.test.ts",
      "-t",
      "a format string of a concat! is its pieces joined by",
    ],
  },
  {
    name: "formatting-runs-nothing",
    breaks:
      "a `Debug` of the crate's isn't taken to run code, so an argument after it is made after it runs",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "            formatted |= written_apart || part.runs_program_code();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "pieces-written-apart-unordered",
    breaks:
      "a piece of a line written apart, `$out(\"lead \")` through stdout's line or `<` into a text that may fail, isn't taken as written, so an argument made by a call that prints is made after it, where Rust makes each first (#1883)",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "            formatted |= written_apart || part.runs_program_code();\n",
    replace: "            formatted |= part.runs_program_code();\n",
    tests: ["test/corpus.test.ts", "-t", "^format_arguments_first.rs$"],
  },
  {
    name: "format-argument-unplaced",
    breaks:
      "a format argument written across lines isn't told so, so its value is on the template's line however long (ADR 0641, amended)",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                    (value, code.is_some_and(|code| code.contains('\\n')))\n",
    replace: "                    (value, code.is_some_and(|_| false))\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a format argument written across lines",
    ],
  },
  {
    name: "formatted-arguments-literal",
    breaks: "a panic of an `Arguments` made with arguments gives a `&str` payload",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                    Std::FmtNew => Some(false),\n",
    replace: "                    Std::FmtNew => Some(true),\n",
    tests: ["test/corpus.test.ts", "-t", "^panic_payload_kinds\\.rs$"],
  },
];

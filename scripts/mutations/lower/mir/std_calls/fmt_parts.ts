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
      "test/lowering-strings.test.ts",
      "-t",
      "a format string of a concat! is its pieces joined by",
    ],
  },
  {
    name: "mir-pretty-dyn-debug-plain",
    breaks: "MIR's `{:#?}` of a `&dyn Debug` made elsewhere shows it plain",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "                    if spec.alternate && kind == Std::FmtDebug && self.is_dyn_debug(ty) {\n",
    replace: "                    if false {\n",
    tests: ["test/lowering.test.ts", "-t", "MIR refuses .....? of a kept"],
  },
  {
    name: "formatting-runs-nothing",
    breaks:
      "a `Debug` of the crate's isn't taken to run code, so an argument after it is made after it runs",
    file: "src/lower/mir/std_calls/fmt_parts.rs",
    find: "            formatted |= part.runs_program_code();\n",
    replace: "",
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
];

// Mutations of src/lower/display/sink_writes.rs (ADR 1135, #1652).
import type { Mutation } from "../../../mutations";

const tests = ["test/writer-sink.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "sink-char-written-as-str",
    breaks: "a `char`'s `{}` into a writer is given to its `write_str`, not its `write_char`",
    file: "src/lower/display/sink_writes.rs",
    find: "                true => SinkWrite::Char(value),\n",
    replace: "                true => SinkWrite::Str(value),\n",
    tests,
  },
  {
    name: "sink-signed-integer-whole",
    breaks: "a signed integer's `{}` into a writer is one `write_str`, its sign not apart",
    file: "src/lower/display/sink_writes.rs",
    find: "            Std::FmtDisplay | Std::FmtDebug if let Some(num) = int => match options || num.signed() {\n",
    replace: "            Std::FmtDisplay | Std::FmtDebug if let Some(num) = int => match options || (num.signed() && false) {\n",
    tests,
  },
  {
    name: "sink-radix-prefix-dropped",
    breaks: "`{:#x}` into a writer writes no `0x`",
    file: "src/lower/display/sink_writes.rs",
    find: "                        vec![Expr::bool(true), Expr::str(radix.prefix()), digits, options_given()],\n",
    replace: "                        vec![Expr::bool(true), Expr::str(\"\"), digits, options_given()],\n",
    tests,
  },
  {
    name: "sink-string-never-padded",
    breaks: "a string's `{:>6}` into a writer isn't padded",
    file: "src/lower/display/sink_writes.rs",
    find: "        let padded = |text: Expr| match width.is_none() && precision.is_none() {\n",
    replace: "        let padded = |text: Expr| match true {\n",
    tests,
  },
];

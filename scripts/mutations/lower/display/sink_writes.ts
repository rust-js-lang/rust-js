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
  {
    name: "sink-given-integer-whole",
    breaks: "an integer's `Display` given a framed writer's `Formatter` is one `write_str`, its sign not apart",
    file: "src/lower/display/sink_writes.rs",
    find: '            Std::FmtDisplay if int.is_some() => SinkWrite::Helper("$writeInteger", given(value)),\n',
    replace: '            Std::FmtDisplay if int.is_some() => SinkWrite::Helper("$writeIntegral", vec![Expr::bool(true), Expr::str(""), value, options]),\n',
    tests,
  },
  {
    name: "sink-given-char-unpadded",
    breaks: "a `char`'s `Display` given a framed writer's `Formatter` is its `write_char`, whatever its width",
    file: "src/lower/display/sink_writes.rs",
    find: '            Std::FmtDisplay if ty.is_char() => SinkWrite::Helper("$writeCharOf", given(value)),\n',
    replace: "            Std::FmtDisplay if ty.is_char() => SinkWrite::Char(value),\n",
    tests,
  },
  {
    name: "sink-float-exp-case-swapped",
    breaks: "a float's `{:e}` into a writer is written by `UpperExp`, `1E2`",
    file: "src/lower/display/sink_writes.rs",
    find: '        Std::FmtExp(true) => "E",\n',
    replace: '        Std::FmtExp(true) => "e",\n',
    tests,
  },
  {
    name: "sink-float-as-f64",
    breaks: "an `f32` written into a writer is written by an `f64`'s digits, `0.10000000149011612`",
    file: "src/lower/display/sink_writes.rs",
    find: "                let mut args = vec![value, Expr::str(float_trait(kind)), Expr::bool(num == Num::F32)];\n",
    replace: "                let mut args = vec![value, Expr::str(float_trait(kind)), Expr::bool(num != num)];\n",
    tests,
  },
  {
    name: "sink-given-float-refused",
    breaks: "a float's `fmt` given a framed writer's `Formatter` is refused (#1652)",
    file: "src/lower/display/sink_writes.rs",
    find: "                if let Some(num) = shown_num(ty).filter(|num| num.float()) =>\n",
    replace: "                if let Some(num) = shown_num(ty).filter(|num| num.float() && false) =>\n",
    tests,
  },
  {
    name: "sink-given-radix-prefix-dropped",
    breaks: "`{:#06x}` of a `T` written by its dictionary's `fmtInto` writes no `0x` (#1652)",
    file: "src/lower/display/sink_writes.rs",
    find: "                    vec![Expr::bool(true), Expr::str(radix.prefix()), digits, options],\n",
    replace: "                    vec![Expr::bool(true), Expr::str(\"\"), digits, options],\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of generic code's and dyns' writes is native Rust's, in the plain build"],
  },
];

// Mutations of src/lower/mir/given_arguments.rs (ADR 0093): an
// `fmt::Arguments` given on, a function of its text (#1565).
import type { Mutation } from "../../../mutations";

const grid = ["test/corpus.test.ts", "-t", "^format_args_given_on_formatted_where_written\\.rs$"];

export const mutations: Mutation[] = [
  {
    name: "given-on-arguments-made-whole",
    breaks: "an `Arguments` given on is its text, made where the macro is written, so its writers print before the callee's own output, and once though it's written twice",
    file: "src/lower/mir/given_arguments.rs",
    find: "        matches!(known, Some(Std::FmtNew | Std::FmtStr))\n",
    replace: "        false && matches!(known, Some(Std::FmtNew | Std::FmtStr))\n",
    tests: grid,
  },
  {
    name: "given-on-operands-made-late",
    breaks: "an `Arguments` given on evaluates its operands where it's formatted, each time, so an operand's effect runs again and a `Cell` read where it's made is read late",
    file: "src/lower/mir/given_arguments.rs",
    find: "                    && !value.is_constant()\n",
    replace: "                    && !value.is_constant()\n                    && false\n",
    tests: grid,
  },
  {
    name: "given-on-arguments-uncalled",
    breaks: "std's formatting given an `Arguments` that's a function writes the function, not its text",
    file: "src/lower/mir/given_arguments.rs",
    find: "            let called = Expr::call(given, Vec::new());\n",
    replace: "            let called = given;\n",
    tests: grid,
  },
  {
    name: "given-on-failing-partial-lost",
    breaks: "an `Arguments` given on whose writer fails loses what it made before the `Err`, `[B` where native Rust has `[aB`",
    file: "src/lower/mir/given_arguments.rs",
    find: "            Value::Fmt(_, _, text) => (text, true),\n",
    replace: "            Value::Fmt(_, _, text) => (text, false),\n",
    tests: grid,
  },
  {
    name: "own-write-fmt-given-called",
    breaks: "the function of a `write!`'s `Arguments` given a writer's own `write_fmt` is called first, its text given, which the body calls (#1695)",
    file: "src/lower/mir/given_arguments.rs",
    find: "            || resolves_to_own(&self.recognition(), callee, generic_args)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^writer_with_its_own_write_fmt.rs$"],
  },
  {
    name: "given-to-a-writer-failing-text-lost",
    breaks: "an `Arguments` given to a writer whose value fails loses what it wrote before the `Err` where it's made a `String`'s text, `\"\"` where native Rust has `\"x1y\"`",
    file: "src/lower/mir/given_arguments.rs",
    find: "            match fails || self.krate.failing.any {\n",
    replace: "            match false {\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of Arguments given on's writes is native Rust's, in the plain build"],
  },
  {
    name: "given-to-writers-unrefused",
    breaks: "an `Arguments` a function returned, given to a function that writes it into a writer, compiles, and throws where it's written",
    file: "src/lower/mir/given_arguments.rs",
    find: "                && !self.sink_form_given(state.body, operand)\n",
    replace: "                && false\n",
    tests: ["test/writer-sink.test.ts", "-t", "an Arguments made elsewhere given to a function"],
  },
  {
    name: "sink-form-given-of-text",
    breaks: "an `Arguments` made where it's written into a writer, its text, is taken for the function that writes into the sink it claims, `$writingInto(f, \"b1\")`",
    file: "src/lower/mir/given_arguments.rs",
    find: "                return arguments_use(&recognition, body, local) == ArgumentsUse::GivenOn;\n",
    replace: "                return true;\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of writes is native Rust's, in the plain build"],
  },
  {
    name: "partial-not-written-through-a-dictionary",
    breaks: "a `write!` through generic code's writer of a value that fails makes its text whole, so what came before the value is lost (#1696)",
    file: "src/lower/mir/given_arguments.rs",
    find: "        if crate::lower::recognition::writes_piecewise() || !crate::lower::recognition::is_write_fmt(tcx, callee) {\n",
    replace: "        if true || !crate::lower::recognition::is_write_fmt(tcx, callee) {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_write_into_a_string_keeps_text_before_a_failure.rs$"],
  },
];

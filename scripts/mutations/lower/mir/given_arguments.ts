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
    find: "        let called = Expr::call(given, Vec::new());\n",
    replace: "        let called = given;\n",
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
    name: "given-to-a-writer-failing-text-lost",
    breaks: "an `Arguments` given to a writer whose value fails loses what it wrote before the `Err` where it's made a `String`'s text, `\"\"` where native Rust has `\"x1y\"`",
    file: "src/lower/mir/given_arguments.rs",
    find: "            match fails || self.krate.failing.any {\n",
    replace: "            match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_arguments_given_on.rs$"],
  },
  {
    name: "sink-form-given-of-text",
    breaks: "an `Arguments` made where it's written into a writer, its text, is taken for the function that writes into the sink it claims, `$writingInto(f, \"b1\")`",
    file: "src/lower/mir/given_arguments.rs",
    find: "                    if arguments_use(&recognition, body, local) != ArgumentsUse::GivenOn {\n",
    replace: "                    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_pieces.rs$"],
  },
  {
    name: "sink-form-of-a-place-unseen",
    breaks: "an `Arguments` read of a field, `Display::fmt(&self.0, f)`, kept so, is refused as made elsewhere",
    file: "src/lower/mir/given_arguments.rs",
    find: "                    Some(from) if read(from) => {}\n",
    replace: "                    Some(from) if read(from) => return false,\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_arguments_kept.rs$"],
  },
  {
    name: "sink-form-of-a-constant-unseen",
    breaks: "a constant `Arguments` written into a writer is refused as made elsewhere",
    file: "src/lower/mir/given_arguments.rs",
    find: "            return operand.constant().is_some();\n",
    replace: "            return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_arguments_kept.rs$"],
  },
  {
    name: "sink-form-of-a-call-result-unseen",
    breaks: "an `Arguments` a function returned, `given(format_args!(..))`, written into a writer, is refused as made elsewhere",
    file: "src/lower/mir/given_arguments.rs",
    find: "                Some(_) => continue,\n",
    replace: "                Some(_) => return false,\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_sink_arguments_kept.rs$"],
  },
  {
    name: "arguments-sink-form-where-nothing-arms",
    breaks: "an `Arguments` given on is the function that writes into a sink where nothing arms one, its only use its text, where its template reads plainly",
    file: "src/lower/mir/given_arguments.rs",
    find: "        marks.arms && crate::lower::analysis::given_to_a_write(&self.recognition(), marks, body, local)\n",
    replace: "        crate::lower::analysis::given_to_a_write(&self.recognition(), marks, body, local)\n",
    tests: ["test/corpus.test.ts", "-t", "^arguments_kept_by_a_generic_std_function.rs$"],
  },
];

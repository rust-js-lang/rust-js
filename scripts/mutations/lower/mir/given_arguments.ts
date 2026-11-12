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
];

// Mutations of src/lower/mir/arguments_use.rs (ADR 0093): whether an
// `fmt::Arguments` a body makes is formatted where it's made (#1565).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "arguments-given-to-crate-taken-in-place",
    breaks: "an `Arguments` given to a function of the crate's is taken to be formatted where it's made, its text, which the callee calls as a function",
    file: "src/lower/mir/arguments_use.rs",
    find: "                std && args.iter().all(|arg| match &arg.node {\n",
    replace: "                args.iter().all(|arg| match &arg.node {\n",
    tests: ["test/corpus.test.ts", "-t", "^format_args_given_on_formatted_where_written\\.rs$"],
  },
  {
    name: "assert-message-given-on",
    breaks: "`assert_eq!`'s message, held in a `Some` std alone is given, is taken as given on, a function its failure message writes as code",
    file: "src/lower/mir/arguments_use.rs",
    find: "                    Rvalue::Aggregate(_, items) if depth > 0 => {\n",
    replace: "                    Rvalue::Aggregate(_, items) if depth > 9 => {\n",
    tests: ["test/corpus.test.ts", "-t", "^assert_failure\\.rs$"],
  },
  {
    name: "as-str-lent-given-on",
    breaks: "an `Arguments` lent to its `as_str()` is taken as given on, and `as_str()` of a literal gives the function, not its text",
    file: "src/lower/mir/arguments_use.rs",
    find: "    reads > 0 && reads == asked\n",
    replace: "    reads > 0 && reads == asked && false\n",
    tests: ["test/corpus.test.ts", "-t", "^(fmt_arguments_as_str|format_args_given_on_formatted_where_written)\\.rs$"],
    snapshots: true,
  },
  {
    name: "formatter-arguments-whole",
    breaks: "a framed writer's own `write!` is its text whole, one `write_str` of the writer's, where std writes each piece (#1652)",
    file: "src/lower/mir/arguments_use.rs",
    find: "    arguments_use(recognition, body, local) == ArgumentsUse::InPlace\n",
    replace: "    arguments_use(recognition, body, local) == ArgumentsUse::InPlace && false\n",
    tests: ["test/writer-sink.test.ts"],
  },
];

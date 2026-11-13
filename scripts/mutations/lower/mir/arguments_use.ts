// Mutations of src/lower/mir/arguments_use.rs (ADR 0093): whether an
// `fmt::Arguments` a body makes is formatted where it's made (#1565).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "arguments-given-to-crate-taken-in-place",
    breaks: "an `Arguments` given to a function of the crate's is taken to be formatted where it's made, its text, which the callee calls as a function",
    file: "src/lower/mir/arguments_use.rs",
    find: "                std && args.iter().enumerate().all(|(at, arg)| match &arg.node {\n",
    replace: "                args.iter().enumerate().all(|(at, arg)| match &arg.node {\n",
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
    find: "    let in_place = arguments_use(recognition, body, local) == ArgumentsUse::InPlace;\n",
    replace: "    let in_place = false;\n",
    tests: ["test/writer-sink.test.ts"],
  },
  {
    name: "own-write-fmt-given-text",
    breaks: "a writer's own `write_fmt` is given a `write!`'s text, which its body calls as a function, `args is not a function` (#1695)",
    file: "src/lower/mir/arguments_use.rs",
    find: "                        && !resolves_to_own(recognition, id, generic_args)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^writer_with_its_own_write_fmt.rs$"],
  },
  {
    name: "dictionary-write-arguments-as-text",
    breaks: "a `format_args!` given generic code's `write_fmt` is its text, not the function that writes into the sink it claims, which the writer's dictionary's `writeFmt` writes",
    file: "src/lower/mir/arguments_use.rs",
    find: "                        && !recognition.writes_through_a_dictionary(id, generic_args)\n                });\n",
    replace: "                });\n",
    tests: ["test/writer-sink.test.ts", "-t", "each of own write_fmt's through generic code's writes is native Rust's, in the plain build"],
  },
  {
    name: "std-formats-every-param",
    breaks: "an `Arguments` a generic std function keeps, `Box::new(args)`, is made its text, and reading it calls a string (#1726)",
    file: "src/lower/mir/arguments_use.rs",
    find: "    inputs.get(at).is_some_and(|input| !input.has_non_region_param())\n",
    replace: "    inputs.get(at).is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^arguments_kept_by_a_generic_std_function.rs$"],
  },
];

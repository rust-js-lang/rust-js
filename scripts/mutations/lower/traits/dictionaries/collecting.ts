// Mutations of src/lower/traits/dictionaries/collecting.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "from-iter-dictionary-unclosed",
    breaks: "generic code's `collect` into a std collection closes nothing of what it's given where collecting it panics, where std drops the iterator its collect holds as it unwinds (#1141, #1239)",
    file: "src/lower/traits/dictionaries/collecting.rs",
    find: "        let made = Expr::call_helper(\n            \"$collecting\",\n            vec![\n                Expr::var(\"items\"),\n                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]),\n            ],\n        );\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^consumer_closes_what_it_steps_on_a_panic.rs$"],
  },
  {
    name: "from-iter-entry-folds-to-an-array",
    breaks: "a std collection's `from_iter` given generic code is its own fold into an array, `$foldToArray(items)`, not the collect generic code's own `Vec` collect is, so `collect::<B>()` of `B = Vec<u32>` asks no hint where std's asks it as it grows (#1239)",
    file: "src/lower/traits/dictionaries/collecting.rs",
    find: "            cx.collected_into((Expr::var(\"items\"), None), output, Some(item), Some(param), span)\n",
    replace: "            cx.collected_into((Expr::call_helper(\"$foldToArray\", vec![Expr::var(\"items\")]), None), output, Some(item), None, span)\n",
    tests: ["test/corpus.test.ts", "-t", "^collect_into_a_type_parameter_is_stds\\.rs$"],
  },
];

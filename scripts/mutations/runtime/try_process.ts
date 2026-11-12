// Mutations of src/runtime/try_process.js, std's `try_process` of a
// fallible `collect()` whose `from_iter` runs code (#1588).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "try-process-made-kept",
    breaks: "what the crate's `from_iter` made is never dropped where a `None` or an `Err` ended its shunt, where `try_process` drops it (#1588)",
    file: "src/runtime/try_process.js",
    find: "  if (!ended) return someBoxed ? $some(made) : made;\n  drop?.(made);\n",
    replace: "  if (!ended) return someBoxed ? $some(made) : made;\n",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
  {
    name: "try-process-shunt-fieldless",
    breaks: "the shunt has no `iter` field, so std's drop glue of a `GenericShunt`, which the crate's `from_iter` drops what it's given by, throws (#1588)",
    file: "src/runtime/try_process.js",
    find: "  shunt.iter = it;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
  {
    name: "try-process-err-kept-unwinding",
    breaks: "the `Err` a shunt ended at isn't dropped where the crate's `from_iter` panics, a local of `try_process`'s frame (#1588)",
    file: "src/runtime/try_process.js",
    find: "    dropResidual && (() => ended && dropResidual(residual)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
  {
    name: "try-process-err-replaced-kept",
    breaks: "an `Err` kept isn't dropped where a later one, stepped on past it, replaces it (#1588)",
    file: "src/runtime/try_process.js",
    find: "        if (ended) dropResidual?.(before);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fallible_collect_into_own_from_iter.rs$"],
  },
];

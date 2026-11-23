// Mutations of crates/rust-js-lower/src/iterators/iterator_form.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "stepped-back-as-boxed",
    breaks:
      "`$iterator(it, next, nextBack)` reads its `nextBack` as whether its `Some` is boxed",
    file: "crates/rust-js-lower/src/iterators/iterator_form.rs",
    find: "        .map(|(at, arg)| {\n            arg.unwrap_or_else(|| match at {\n                0 => Expr::bool(false),\n                _ => Expr::js_undefined(),\n            })\n        })\n",
    replace: "        .filter_map(|(_, arg)| arg)\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^generic_double_ended.rs$"],
  },
];

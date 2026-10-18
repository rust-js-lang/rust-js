// Mutations of src/lower/std_types/result.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "question-box-error-unconverted",
    breaks: "`?` into a `Box<dyn Error>` returns the error as it is, not as a pair",
    file: "src/lower/std_types/result.rs",
    find: "            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    replace: "            Conversion::Dyn(_) => error,\n            #[allow(unreachable_patterns)]\n            Conversion::Dyn(dictionary) => Expr::object(vec![\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "question-null-test",
    breaks: "`?` of an Option never falsy tests `item == null`, not `!item`",
    file: "src/lower/std_types/result.rs",
    find: "            Some(inner) => self.absent(subject, inner),\n",
    replace: "            Some(_) => Expr::bin(Op::LooseEq, subject, Expr::null()),\n",
    tests: ["test/lowering.test.ts", "-t", "of an Option never falsy tests its truth"],
  },
  {
    name: "result-test-negation-unseen",
    breaks: "`r.TAG !== \"Err\"` is read as `===`, so tidy doesn't put a `?`'s `return` first",
    file: "src/lower/std_types/result.rs",
    find: "    (key == \"TAG\").then_some((&**result, ok, *op == Op::Eq))\n",
    replace: "    (key == \"TAG\").then_some((&**result, ok, true))\n",
    tests: ["test/snapshots.test.ts"],
  },
  {
    name: "unwrapped-made-effects-lost",
    breaks: "`unwrap_or(loud())` of an `Ok` just made doesn't run `loud()`, which Rust runs first",
    file: "src/lower/std_types/result.rs",
    find: "        .filter(|_| given.iter().all(|e| !e.has_effects()))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^poison_error.rs$"],
  },
];

// Mutations of src/js/result.rs (ADR 0093): a `Result`'s JS, made,
// tested and read.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "result-test-negation-unseen",
    breaks: "`r.TAG !== \"Err\"` is read as `===`, so tidy doesn't put a `?`'s `return` first",
    file: "src/js/result.rs",
    find: "    (key == TAG).then_some((&**result, ok, *op == Op::Eq))\n",
    replace: "    (key == TAG).then_some((&**result, ok, true))\n",
    tests: ["test/snapshots.test.ts"],
  },
  {
    name: "unwrapped-made-effects-lost",
    breaks: "`unwrap_or(loud())` of an `Ok` just made doesn't run `loud()`, which Rust runs first",
    file: "src/js/result.rs",
    find: "        .filter(|_| given.iter().all(|e| !e.has_effects()))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^poison_error.rs$"],
  },
];

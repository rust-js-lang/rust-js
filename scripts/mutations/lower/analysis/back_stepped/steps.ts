// Mutations of crates/rust-js-lower/src/analysis/back_stepped/steps.rs (ADR 0093): what a value's later uses ask of it, by std's steps (#1458).
// What's asked of a value moved or lent into another local, and a use the
// walk doesn't read, which asks every step, have no mutation: no in-place
// form is chosen of such a value today, std's struct being made of what's
// held or escapes, so no program tells them apart yet. P4's flip, which
// asks each choice of its steps, holds them.
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/back_stepped/steps.rs";
const flattenHint = ["test/corpus.test.ts", "-t", "^flatten_asks_its_inner_iterators_hint\\.rs$"];

export const mutations: Mutation[] = [
  {
    name: "steps-of-what-is-held-unasked",
    breaks:
      "what an adapter's own asked steps ask of what it holds is never asked of it: a flatten whose `count` std's `Filter` asks its hint first is JS's own `flatMap`, which has no hint, so the crate's inner iterators' `size_hint` never runs where native's does (#1458)",
    file,
    find: '                Reached::Held(name) => crate::recognition::asked_of_held(name, of_to, "src"),\n',
    replace: "                Reached::Held(_) => Steps::NONE,\n",
    tests: flattenHint,
  },
];

// Mutations of src/lower/mir/tidy/decisions/presence.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "^presence_said_by_chain.rs$"];

export const mutations: Mutation[] = [
  {
    name: "presence-said-never",
    breaks:
      "a `match` of `Some`s of constants whose other arm breaks its loop keeps its presence test in a labeled block, `bb11: { if (value != null) { .. else { break bb11; } } }` (bb11)",
    file: "src/lower/mir/tidy/decisions/presence.rs",
    find: "        if let Some(said) = said_at(stmts, i, switches) {\n",
    replace:
      "        if let Some(said) = said_at(stmts, i, switches).filter(|_| false) {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "presence-said-of-every-test",
    breaks:
      "a test that holds of `None` too, `value !== 3 && value !== 9`, isn't the presence test's too, so `None` takes its arm, `level = undefined` (bb11)",
    file: "src/lower/mir/tidy/decisions/presence.rs",
    find: "        if !says(subject, test, &valued) {\n",
    replace: "        if false {\n",
    tests: corpus,
  },
];

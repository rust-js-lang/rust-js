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
  {
    name: "presence-guard-link-unturned",
    breaks:
      "a guard's link whose failing breaks the block, `Some(x) if x.is_nan()`, keeps the labeled block, `bb10: { if (value != null) { .. else if (!Number.isNaN(value)) { break bb10; } else { level = 0.5; } .. } }`",
    file: "src/lower/mir/tidy/decisions/presence.rs",
    find: "        if matches!(&then[..], [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label) {\n",
    replace: "        if false {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "presence-guard-link-turned-untested",
    breaks:
      "a guard's link turned keeps its failing's test, `else if (value != null && !Number.isNaN(value)) { level = 0.5; }`, so a number that isn't NaN takes the guard's arm",
    file: "src/lower/mir/tidy/decisions/presence.rs",
    find: "            *test = std::mem::replace(test, Expr::undefined()).negated_test();\n",
    replace: "",
    tests: corpus,
  },
];

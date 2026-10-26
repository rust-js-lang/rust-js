// Mutations of src/lower/mir/tidy/options.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tested-option-undefined-value",
    breaks: "`const x = t ? v : undefined; if (x)` is `if (t)` of a `v` that may be `None` itself, so `maybe(None, true)` reads `undefined.name` (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) || changing.contains(v) {\n",
    replace: "    if !matches!(none.kind, ExprKind::Undefined) || changing.contains(v) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
  {
    name: "tested-option-changed-value",
    breaks: "an `Option` of a variable that changes where it's read is read as the variable, so `changed(true)` says `after then after` (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) || changing.contains(v) {\n",
    replace: "    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
  {
    name: "tested-option-read-elsewhere",
    breaks: "an `Option` read past its test is folded away, so what reads it after reads a variable no longer declared (ADR 0910)",
    file: "src/lower/mir/tidy/options.rs",
    find: "    if read_elsewhere || js::declares(reads, x) || js::declares(reads, v) {\n",
    replace: "    if js::declares(reads, x) || js::declares(reads, v) {\n",
    tests: ["test/corpus.test.ts", "-t", "tested_options"],
  },
];

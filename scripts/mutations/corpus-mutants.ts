// Mutations of test/corpus-mutants.ts, the corpus harness's own mutants
// (#918, ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "corpus-mutant-edit-appended",
    breaks: "a corpus mutant's JS edit goes at the module's end, after a binary's `$main(main)`, which ends the process at once on a run-fail case, so the edit never runs there (#2447's first CI run)",
    file: "test/corpus-mutants.ts",
    find: 'writeFileSync(out, `${edit.js}\\n${readFileSync(out, "utf8")}`);',
    replace: 'writeFileSync(out, `${readFileSync(out, "utf8")}\\n${edit.js}\\n`);',
    tests: ["test/mutations.test.ts", "-t", "changes a run that fails at once"],
  },
];

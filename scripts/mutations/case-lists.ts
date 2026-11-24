// A list of what to run naming what its runner hasn't is refused, naming
// each entry (test/case-lists.ts): the metamorphic net given its cases as
// paths ran none and passed (2026-11-25).
import type { Mutation } from "../mutations";

const tests = ["test/case-lists.test.ts", "-t", "a list naming a case by its path, or misspelt, fails"];

export const mutations: Mutation[] = [
  {
    name: "case-list-unmatched-run",
    breaks: "a list naming what its runner hasn't runs what it matches, and passes: the metamorphic net given `test/corpus/...` paths ran none of its cases and said 25 pass",
    file: "test/case-lists.ts",
    find: "  if (unmatched.length === 0) return;",
    replace: "  if (unmatched.length >= 0) return;",
    tests,
  },
  {
    name: "case-list-only-none-matched-refused",
    breaks: "a list is refused only where it matches nothing, so one misspelt name among the rest runs fewer cases than it names, and passes",
    file: "test/case-lists.ts",
    find: "  if (unmatched.length === 0) return;",
    replace: "  if (unmatched.length < new Set(named).size) return;",
    tests,
  },
  {
    name: "case-list-path-unexplained",
    breaks: "a list of paths is refused without saying a case is named by its name, so the person who wrote it can't tell what to write",
    file: "test/case-lists.ts",
    find: "  const pathed = unmatched.find((entry) => entry.includes(\"/\") && matcher(known)(basename(entry)));",
    replace: "  const pathed = undefined as string | undefined;",
    tests,
  },
];

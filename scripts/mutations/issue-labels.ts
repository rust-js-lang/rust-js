// The labels check (scripts/issue-labels.ts, ADR 1085): one that asks less
// lets an issue in without what the counts read.
import type { Mutation } from "../mutations";

const file = "scripts/issue-labels.ts";
const of = (name: string) => ["test/issue-labels.test.ts", "-t", name];

export const mutations: Mutation[] = [
  {
    name: "issue-labels-p0-found-by-unasked",
    breaks: "a P0 is let in with no `found-by: …`, so which nets find wrong answers stops being a count (#2481)",
    file,
    find: 'if (names.includes("P0: wrong answer")) {',
    replace: "if (false) {",
    tests: of("a P0 carries one found-by"),
  },
  {
    name: "issue-labels-p0-found-by-any-number",
    breaks: "a P0 with two `found-by: …` passes, so it's counted for two nets",
    file,
    find: "if (found.length !== 1) needs.push(",
    replace: "if (found.length === 0) needs.push(",
    tests: of("a P0 carries one found-by"),
  },
];

// The nightly's audit of the test selector (scripts/selector-audit.ts, ADR
// 1101, amended): an audit that finds no miss leaves a skipped test's
// failure unfiled.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "select-audit-blind",
    breaks: "the nightly's audit finds no miss: a test no push's selection ran fails at night unfiled",
    file: "scripts/selector-audit.ts",
    find: "    return !ran;",
    replace: "    return false;",
    tests: ["test/select.test.ts", "-t", "a selector's miss"],
  },
];

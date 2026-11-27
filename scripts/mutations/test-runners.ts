// Mutations of tooling/runners.js: a run again of the tests a refusal at run time
// came before in their process, by their whole names
// (ADR 0692, amended).
import type { Mutation } from "../mutations";

const refusedFirst = ["test/rust-js-test-runners.test.ts", "-t", "a test after one refused at run time is run again"];

export const mutations: Mutation[] = [
  {
    name: "run-again-names-unfiltered",
    breaks: "a run again of the tests a refusal came before runs every test, the refused one first, so each is refused again (#2625)",
    file: "tooling/runners.js",
    find: '  const args = pattern === undefined ? given : [...given, ...(runner === "node" ? [`--test-name-pattern=${pattern}`] : ["-t", pattern])];\n',
    replace: "  const args = given;\n",
    tests: refusedFirst,
  },
];

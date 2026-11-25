// The guide's test of each page, asked by its name alone (test/docs.test.ts).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "docs-page-test-checks-the-whole-guide",
    breaks: "a guide page's test, asked alone by `-t`, checks every page, so a mutation one page reaches runs the whole guide's 40 s a mutant, not its page's 5 s (worker-02 agent's `offered-symbol-field-named`)",
    file: "test/docs.test.ts",
    find: '  const at = argv.findIndex((arg) => arg === "-t" || arg === "--test-name-pattern");\n',
    replace: "  const at = -1;\n",
    tests: ["test/docs.test.ts", "-t", "a guide page's test is asked by its name alone"],
  },
];

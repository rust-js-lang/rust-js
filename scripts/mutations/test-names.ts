// The tests a mutation's `tests` may name (test/test-names.ts): a file
// there, and of a `-t`, a test it has, by Bun's own listing of a file whose
// tests aren't all named by literals. #2542 took away the test
// `never-holding-fact-asked` names, of such a file, and `main` went red.
import type { Mutation } from "../mutations";

const tests = ["test/mutations.test.ts", "-t", "a pattern names a test of a file by its literal names"];

export const mutations: Mutation[] = [
  {
    name: "test-names-gone-file-named",
    breaks: "a mutation's `tests` may name a file that isn't there, whose tests run nothing as the compiler is",
    file: "test/test-names.ts",
    find: "  if (!exists) return false;\n",
    replace: "  if (!exists) return true;\n",
    tests,
  },
  {
    name: "test-names-listing-unread",
    breaks: "a file whose tests are named by templates is never asked of Bun's listing, so a pattern naming a test it took away passes, #2542's",
    file: "test/test-names.ts",
    find: "  if (known !== undefined) return known.some((name) => matching.test(name));\n",
    replace: "",
    tests,
  },
  {
    name: "test-names-template-taken-as-literal",
    breaks: "a file whose tests are named by templates is read as all literals, so a pattern naming one of them is refused",
    file: "test/test-names.ts",
    find: "!/^\\s*(?:test|it)(?:\\.\\w+)?\\(\\s*[^\"\\s]/m.test(source)",
    replace: "!/^\\s*(?:test|it)(?:\\.\\w+)?\\(\\s*[^\"\\s`]/m.test(source)",
    tests,
  },
  {
    name: "test-moved-to-the-first-of-several",
    breaks: "a test whose name several other files have is said to have moved to the first of them, and `--remap` writes a guess into its lists",
    file: "test/test-names.ts",
    find: "  return found.length === 1 ? found[0] : undefined;\n",
    replace: "  return found[0];\n",
    tests: ["test/mutations.test.ts", "-t", "a test moved to another file is found by its name"],
  },
];

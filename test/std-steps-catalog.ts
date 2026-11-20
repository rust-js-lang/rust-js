// std's steps catalog, src/lower/recognition/std_steps.txt (#2112), as the
// tests read it: which steps each std adapter and wrapper writes itself,
// and what each step asked of what it holds in each native run. The one
// reader of it in the tests: test/iterator-steps.test.ts holds the
// runtime's adapters to its runs, and test/iterator-paths.ts makes the
// paths table's rows of its writes.

import { readFileSync } from "node:fs";
import { join } from "node:path";

const text = readFileSync(join(import.meta.dir, "../src/lower/recognition/std_steps.txt"), "utf8");

/** Of each adapter or wrapper, by std's name, `Map` or `&mut I`, the steps
 * it writes, its `<name> writes:` line's, in their order. */
export const stdWrites: Map<string, string[]> = (() => {
  const writes = new Map<string, string[]>();
  for (const line of text.split("\n")) {
    const m = /^(.+) writes: (.*)$/.exec(line);
    if (m && !line.startsWith("#")) writes.set(m[1], m[2].split(" ").filter(Boolean));
  }
  return writes;
})();

/** The catalog's runs: of each `<name>#<run> <step>`, what was asked. */
export const stdRuns: Map<string, string> = (() => {
  const runs = new Map<string, string>();
  for (const line of text.split("\n")) {
    if (line.startsWith("#") || !line.includes(" |")) continue;
    const [head, ...cells] = line.split(" |");
    for (const cell of cells) {
      const at = cell.indexOf(":");
      runs.set(`${head} ${cell.slice(0, at)}`, cell.slice(at + 1).trim());
    }
  }
  return runs;
})();

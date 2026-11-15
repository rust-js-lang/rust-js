// Which rows of the iterator tables a change runs before it lands
// (scripts/iteratorRows.ts, #818): a list's entries alone decide the rows
// of those its diff changes, and of what reaches them (dev-loop, ioCF).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "list-entries-unreached",
    breaks: "the rows of what a change names are its names' alone, not of what reaches them: an entry `map` added to `HELD_FLIPPED` runs none of `chain.flatten`'s rows, whose per-item `map` their ids don't name (#1780)",
    file: "scripts/iteratorRows.ts",
    find: "      const all = reached(set);\n",
    replace: "      const all = new Set(set);\n",
    tests: ["test/iterator-rows.test.ts", "-t", "a list's entries alone decide"],
  },
  {
    name: "list-entries-the-whole-line",
    breaks: "a hunk that changes a list's entries alone names every entry on its line, each one's rows run where one was added: a family's flip runs every flipped family's rows, 32 min of its check (ioCF)",
    file: "scripts/iteratorRows.ts",
    find: "      const hunk = entries ?? given;\n",
    replace: "      const hunk = given;\n",
    tests: ["test/iterator-rows.test.ts", "-t", "a list's entries alone decide"],
  },
];

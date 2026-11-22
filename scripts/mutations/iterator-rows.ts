// Which rows of the iterator tables a change runs before it lands
// (scripts/iteratorRows.ts, #818): a list's entries alone decide the rows
// of those its diff changes, and of what reaches them (dev-loop, ioCF); code
// moved within its file removes nothing (dev-loop, ioDT).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "foreign-method-names-rows",
    breaks: "a method the compiler's Rust calls and doesn't define, std's `.map(` or rustc's `.skip_binder()`, names the rows of an adapter, so a lowering change runs each row of `map` and `skip` (#2340)",
    file: "scripts/iteratorRows.ts",
    find: "    defined.has(name) && !steps.has(name) ? call : `.${rest}`,",
    replace: "    call,",
    tests: ["test/iterator-rows.test.ts", "-t", "a method the compiler's Rust calls and doesn't define names no row"],
  },
  {
    name: "own-method-names-no-row",
    breaks: "a method the compiler defines, `.lower_map(`, names no row, so a change to how it lowers `map` runs none of its rows (#2340)",
    file: "scripts/iteratorRows.ts",
    find: "    defined.has(name) && !steps.has(name) ? call : `.${rest}`,",
    replace: "    false ? call : `.${rest}`,",
    tests: ["test/iterator-rows.test.ts", "-t", "a method the compiler's Rust calls and doesn't define names no row"],
  },
  {
    name: "own-method-named-as-a-step-names-it",
    breaks: "a method of the compiler's own named as a consumer is, `any(&self)`, names the consumer `any`, whose every row a change of its call runs (#2340)",
    file: "scripts/iteratorRows.ts",
    find: "    defined.has(name) && !steps.has(name) ? call : `.${rest}`,",
    replace: "    defined.has(name) ? call : `.${rest}`,",
    tests: ["test/iterator-rows.test.ts", "-t", "a method the compiler's Rust calls and doesn't define names no row"],
  },
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
  {
    name: "list-entries-with-their-doc",
    breaks: "a list's doc comment rewritten in the same hunk as its entries makes the hunk more than its entries, so each family on the line is named: the rev flip's check ran 140,563 rows where its one entry's are 15,865",
    file: "scripts/iteratorRows.ts",
    find: '  const code = (lines: string[]) => lines.filter((line) => line.trim() !== "" && !comment(line));\n',
    replace: '  const code = (lines: string[]) => lines.filter((line) => line.trim() !== "");\n',
    tests: ["test/iterator-rows.test.ts", "-t", "a list's entries alone decide"],
  },
  {
    name: "list-entries-line-by-line",
    breaks: "a list rustfmt lays out again, one line to one an entry, is read line by line and isn't its entries: each family on it is named, the flat_map flip's 143,575 rows where its entry's are some 15,000",
    file: "scripts/iteratorRows.ts",
    find: '  if (skeleton(added.join(" ")) === skeleton(removed.join(" "))) {\n',
    replace: '  if (added.length === removed.length && added.every((line, i) => skeleton(line) === skeleton(removed[i]))) {\n',
    tests: ["test/iterator-rows.test.ts", "-t", "a list's entries alone decide"],
  },
  {
    name: "rows-of-an-entry-by-its-words",
    breaks: "a list's entry `\"filter_map\"` names `filter` and `map` by its words, and through what calls them `flatten`: 84,036 matrix rows for one entry",
    file: "scripts/iteratorRows.ts",
    find: "  return whole.length > 0 ? whole : undefined;\n",
    replace: "  return undefined;\n",
    tests: ["test/iterator-rows.test.ts", "-t", "a list's string entry names its adapter whole"],
  },  {
    name: "rows-moved-code-read-as-removed",
    breaks: "code a change moves within its file, or folds onto fewer lines, reads as a removal and runs each row of what it names: #2199's `.filter(present)` moved into one closure ran 84,036 matrix rows, 1,768 s of its push",
    file: "scripts/iteratorRows.ts",
    find: "      hunk.removed.flatMap(identifiers).some((id) => !saidInFile.has(id) && !hunk.added.flatMap(identifiers).includes(id)),\n",
    replace: "      true,\n",
    tests: ["test/iterator-rows.test.ts", "-t", "code a change says again in its file was moved"],
  },
];

// Mutations of src/to_oxc/closing.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments-ends.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "object-end-comment-unplaced",
    breaks: "a comment after a struct literal's last field goes before the statement, `// c\\n return { name, start };`, not before the object's `}` (ADR 0708, amended)",
    file: "src/to_oxc/closing.rs",
    find: "        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))\n",
    replace: "        .filter(|&&(lo, hi)| false && lo < c.lo && c.hi < hi && closing(hi))\n",
    tests,
  },
  {
    name: "object-end-comment-of-any-code-after",
    breaks: "a comment before a struct literal's field, not after its last, is taken for one before its `}`",
    file: "src/to_oxc/closing.rs",
    find: "                    between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ','))\n",
    replace: "                    between.is_some()\n",
    tests,
  },
  {
    name: "object-end-comment-printed-twice",
    breaks: "a comment before an object's `}` is also before the statement it's in, which places what nothing else does (ADR 0708, amended)",
    file: "src/to_oxc/closing.rs",
    find: "        taken.push(c.lo);\n",
    replace: "",
    tests,
  },
  {
    name: "export-list-doc-listed",
    breaks: "a field's doc comment in a namespace thread-local's struct, its declaration's, is printed in the JS's export list too (rd610)",
    file: "src/to_oxc/closing.rs",
    find: "            .filter(|_| !c.doc)\n",
    replace: "",
    tests: ["test/namespace-values.test.ts", "-t", "of an entry.s own fields of one value"],
  },
  {
    name: "export-list-end-before-first",
    breaks: "a comment after a namespace struct's last field, `// $FlowFixMe[..]`, is before the last export, not before the list's `}` (rd610)",
    file: "src/to_oxc/closing.rs",
    find: "                None => fields.last().map(|&lo| (lo, true)),\n",
    replace: "                None => fields.last().map(|&lo| (lo, false)),\n",
    tests: ["test/namespace-values.test.ts", "-t", "of an entry.s own fields of one value"],
  },
  {
    name: "export-list-end-at-its-field",
    breaks: "a comment after a namespace struct's last field is split before the last export, not past it at the list's `}` (rd610)",
    file: "src/to_oxc/closing.rs",
    find: "                .and_then(|text| text.get(column as usize..)?.find('}').map(|i| column + i as u32)),\n",
    replace: "                .map(|_| column),\n",
    tests: ["test/namespace-values.test.ts", "-t", "of an entry.s own fields of one value"],
  },
  {
    name: "export-list-comment-of-its-own-field",
    breaks: "a comment above a namespace struct's field, `// TODO: Remove this ..`, is before the export after it, not its own (rd610)",
    file: "src/to_oxc/closing.rs",
    find: "            .and_then(|(_, fields)| match fields.iter().find(|&&lo| lo > c.hi) {\n",
    replace: "            .and_then(|(_, fields)| match fields.iter().filter(|&&lo| lo > c.hi).nth(1) {\n",
    tests: ["test/namespace-values.test.ts", "-t", "of an entry.s own fields of one value"],
  },
];

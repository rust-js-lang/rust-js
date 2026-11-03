// Mutations of src/to_oxc/closing.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments-ends.test.ts"];
const commentFields = ["test/lowering-comment-fields.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "object-end-comment-unplaced",
    breaks: "a comment after a struct literal's last field goes before the statement, `// c\\n return { name, start };`, not before the object's `}` (ADR 0708, amended)",
    file: "src/to_oxc/closing.rs",
    find: "        .map(|end| end.hi - 1)\n",
    replace: "        .filter(|_| false)\n        .map(|end| end.hi - 1)\n",
    tests,
  },
  {
    name: "object-end-comment-before-left-out-unplaced",
    breaks: "a comment before a struct literal's last fields, `None`s it leaves out, goes before the statement, `// We don't know the requesting owner nor stack.` above `return {`, not before the object's `}` (rd321)",
    file: "src/to_oxc/closing.rs",
    find: "    let written_before = |end: &ObjectEnd| end.last_written.is_some_and(|last| last < c.lo);\n",
    replace: "    let written_before = |end: &ObjectEnd| end.last_written.is_some_and(|last| last < c.lo && false);\n",
    tests: commentFields,
  },
  {
    name: "object-end-comment-of-outer-literal",
    breaks: "a comment before a nested literal's left-out `None` is before the outer object's `}`, `// Its owner's.` after `info: { .. }` (rd321)",
    file: "src/to_oxc/closing.rs",
    find: "        .or_else(|| (ends.iter().filter(braced).min_by_key(|end| end.hi - end.lo)).filter(|end| written_before(end)))\n",
    replace: "        .or_else(|| (ends.iter().filter(braced).max_by_key(|end| end.hi - end.lo)).filter(|end| written_before(end)))\n",
    tests: commentFields,
  },
  {
    name: "object-end-last-written-first",
    breaks: "where an object's last written field starts is where its first does, so a comment before a `None` and then a written key, `// We'll initialize these at the root.` before `_threadCount`, is before the `}` (rd321)",
    file: "src/to_oxc/closing.rs",
    find: "try_fold(e.span.lo, |last, at| at.map(|at| last.max(at)));\n",
    replace: "try_fold(e.span.lo, |last, at| at.map(|at| last.min(at)));\n",
    tests: commentFields,
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

// Mutations of src/to_oxc/arguments.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments.test.ts", "-t", "a comment after a call's last argument is before its parenthesis"];

export const mutations: Mutation[] = [
  {
    name: "argument-end-comment-unplaced",
    breaks: "a comment after a call's last argument goes before the statement, `// c\\n console.warn(a, b);`, not before the call's `)` (ADR 0708, amended)",
    file: "src/to_oxc/arguments.rs",
    find: "        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))\n",
    replace: "        .filter(|&&(lo, hi)| false && lo < c.lo && c.hi < hi && closing(hi))\n",
    tests,
  },
  {
    name: "argument-end-of-any-comment-inside",
    breaks: "a comment before a call's argument is taken to be after its last, before its `)` (ADR 0708, amended)",
    file: "src/to_oxc/arguments.rs",
    find: "                between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ',' || ch == ')'))\n",
    replace: "                between.is_some()\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment before an argument or a field is before it"],
  },
  {
    name: "argument-end-of-the-outermost",
    breaks: "a comment after the last argument of a call in a call's last argument is before the outer call's `)`",
    file: "src/to_oxc/arguments.rs",
    find: "        .min_by_key(|&&(lo, hi)| hi - lo)\n",
    replace: "        .max_by_key(|&&(lo, hi)| hi - lo)\n",
    tests,
  },
  {
    name: "method-field-comment-inside",
    breaks: "a comment before a field that's a method, ReactNoopUpdateQueue's `// Forces an update.`, is in the method's body, and the blank line before it lost",
    file: "src/to_oxc/arguments.rs",
    find: "                        js::Prop::Field(key, value) | js::Prop::Method(key, value) => {\n",
    replace: "                        js::Prop::Field(key, value) => {\n",
    tests: ["test/lowering-comments.test.ts", "-t", "field that's a method"],
  },
  {
    name: "inner-part-start-kept",
    breaks: "a comment before a struct literal's field whose value is a method call, `parent: x.unwrap()`, is before the `return {`, as the call's receiver's part takes its start (ADR 0708, amended)",
    file: "src/to_oxc/arguments.rs",
    find: "                    starts.entry(s.lo).or_insert((e.span.lo, field));\n",
    replace: "                    starts.insert(s.lo, (e.span.lo, field));\n",
    tests: ["test/lowering-comment-fields.test.ts", "-t", "a blank line before a method call's field"],
  },
  {
    name: "reexport-leaf-comments-unplaced",
    breaks: "a comment on lines of its own before a name in a `pub use` list, `// State` above `use_state`, isn't before its name in the `export { .. } from` (rd338)",
    file: "src/to_oxc/arguments.rs",
    find: "            starts.entry(name.lo).or_insert((reexport.span.lo, true));\n",
    replace: "            let _ = name;\n",
    tests: ["test/lowering-comments-ends.test.ts", "-t", "a pub use list.s comments are its names."],
  },
];

// Mutations of src/to_oxc/trailing.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trailing-comment-dropped",
    breaks: "a comment after code on its line, `let n = 1; // why`, is lost",
    file: "src/to_oxc/trailing.rs",
    find: "                    after.entry(last).or_default().push(&comment.text);",
    replace: "                    let _ = (last, comment);",
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "value-comment-equals-only-before",
    breaks: "a value whose statement's JS is placed where the value is, `let mut last = /* first */ mode`, or of a thread local, has its comment at its line's end or after its `let`",
    file: "src/to_oxc/trailing.rs",
    find: "                let found = (text[..at].rfind(\" = \")).or_else(|| text[at..].find(\" = \").map(|i| at + i))?;\n",
    replace: "                let found = text[..at].rfind(\" = \")?;\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment between an = and its value"],
  },
  {
    name: "line-comment-after-the-line",
    breaks: "a line comment a later line's JS follows on its JS line is after all of it, `// ellipsis // ellipsis`",
    file: "src/to_oxc/trailing.rs",
    find: "            (_, Some((line, end))) => ends.entry(line).or_default().push((end, &comment.text)),\n",
    replace: "",
    tests: ["test/lowering-comments.test.ts", "-t", "a line comment is after its code"],
  },
  {
    name: "line-comment-cut-before-operator",
    breaks: "a line comment's line is cut before the next line's code, after its `:`, `? \"…\" : // ellipsis`",
    file: "src/to_oxc/trailing.rs",
    find: "                .trim_end_matches(|c: char| c.is_whitespace() || \"([{?:,\".contains(c))\n",
    replace: "                .trim_end_matches(char::is_whitespace)\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a line comment is after its code"],
  },
];

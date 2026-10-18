// Mutations of src/comments.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "output-line-docs-carried",
    breaks: "a module's or an item's doc comment, `//!` or `///`, is taken for one the file starts with",
    file: "src/comments.rs",
    find: "            if doc(\"///\", &[\"////\"]) || trimmed.starts_with(\"//!\") {",
    replace: "            if false {",
    tests: ["test/emission.test.ts", "-t", "leading comments"],
  },
  {
    name: "output-block-docs-carried",
    breaks: "an item's block doc comment, `/** .. */`, is taken for one the file starts with",
    file: "src/comments.rs",
    find: "            if doc(\"/**\", &[\"/***\", \"/**/\"]) || trimmed.starts_with(\"/*!\") {",
    replace: "            if trimmed.starts_with(\"/*!\") {",
    tests: ["test/emission.test.ts", "-t", "leading comments"],
  },
  {
    name: "output-nested-comment-carried",
    breaks: "a block comment Rust nests is written in JS, which ends it at its first `*/`",
    file: "src/comments.rs",
    find: "        Some(inner) => !inner.contains(\"/*\"),",
    replace: "        Some(_) => true,",
    tests: ["test/emission.test.ts", "-t", "leading comments"],
  },
  {
    name: "output-line-comment-js-ends",
    breaks: "a line comment with a bare carriage return, which JS ends there, is written, and its rest is code",
    file: "src/comments.rs",
    find: "        None => !comment.contains(['\\r', '\\u{2028}', '\\u{2029}']),",
    replace: "        None => true,",
    tests: ["test/emission.test.ts", "-t", "leading comments"],
  },
  {
    name: "doc-comment-ended-early",
    breaks: "a `///` line with a `*/` is written in a JSDoc, which it ends, and its rest is code",
    file: "src/comments.rs",
    find: "    if lines.iter().any(|line| line.contains(\"*/\")) {\n",
    replace: "    if false {\n",
    tests: ["test/lowering.test.ts", "-t", "a comment is in the JS where"],
  },
];

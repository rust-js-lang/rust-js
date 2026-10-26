// Mutations of src/comments.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "head-comment-cut-inside-a-comment",
    breaks: "a file's leading block comment with a blank line in it is cut at that line, and rust-js writes an unterminated `/*` (#312)",
    file: "src/comments.rs",
    find: "        && let Some(&(before, _)) = gaps.iter().rev().find(|&&(from, to)| blank(&text[from..to]))\n",
    replace: "        && let Some(before) = text[..end].rfind(\"\\n\\n\")\n",
    tests: ["test/lowering-comments.test.ts", "-t", "blank line in it is one comment"],
  },
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
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "leading-run-above-code-kept",
    breaks: "a comment run right above the first code, a blank line after the license, stays in the header, a blank line between it and its code",
    file: "src/comments.rs",
    find: "        && let Some(&(before, _)) = gaps.iter().rev().find(|&&(from, to)| blank(&text[from..to]))\n",
    replace: "        && let Some(&(before, _)) = gaps.iter().rev().find(|&&(from, to)| blank(&text[from..to])).filter(|_| false)\n",
    tests: ["test/emission.test.ts", "-t", "a file's leading comments are its JS's"],
  },
];

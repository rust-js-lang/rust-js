// Mutations of src/to_oxc/openings.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "erased-let-block-code",
    breaks: "a `let` read in place whose value is a block, `unsafe { js::cast(arg) }`, is taken for code, so the comment before it goes before the `for` it's in (ADR 0444, amended)",
    file: "src/to_oxc/openings.rs",
    find: "            '(' | '[' | '{' => depth += 1,\n",
    replace: "            '(' | '[' => depth += 1,\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before a let read in place"],
  },
  {
    name: "type-alias-is-code",
    breaks: "a comment before a `pub type`, which writes no JS, is lost, ReactFiberConfigWithNoHydration's `// Hydration (when unsupported)` (ADR 0444, amended)",
    file: "src/to_oxc/openings.rs",
    find: "    item_after_visibility(statement).starts_with(\"type \")\n",
    replace: "    false\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "an item with no JS"],
  },
  {
    name: "type-use-is-code",
    breaks: "a comment above a `use` that makes no import, a type's, is lost, not above the next import or item (rd92)",
    file: "src/to_oxc/openings.rs",
    find: "    item_after_visibility(statement).starts_with(\"use \")\n",
    replace: "    false\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "above a use"],
  },
  {
    name: "receiver-not-only",
    breaks: "a comment before an if chain's arm whose first statement is a method call, `None => { DOWN.set(false); }`, whose span starts at `set`, is lost, not before `down = false` in the `else` (rd119, ADR 0905 amended)",
    file: "src/to_oxc/openings.rs",
    find: "        || (code.strip_suffix('.')).is_some_and(|path| {\n",
    replace: "        || (code.strip_suffix('.')).filter(|_| false).is_some_and(|path| {\n",
    tests: ["test/lowering-comments-inside.test.ts", "-t", "an arm of a block of an if chain"],
  },
  {
    name: "let-value-of-any-start",
    breaks:
      "a comment between an arm's patterns, `\"keydown\"\\n// ..\\n| \"keyup\"`, is read as above a `let` whose JS starts inside its value, and printed before the arm's body, not before `case \"keyup\":` (rd533)",
    file: "src/to_oxc/openings.rs",
    find: "    if !first.trim_start().starts_with(\"let \") {\n",
    replace: "    if first.trim().is_empty() {\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment between an arm.s patterns"],
  },
  {
    name: "let-end-past-its-indentation",
    breaks:
      "a `let` a comment is above ends where its block does, so the comment is before the first JS printed of what's after it too, a declaration hoisted from a later statement, `let cache` (rd533)",
    file: "src/to_oxc/openings.rs",
    find: "        if !code.is_empty() && (depth < indent || depth == indent && !code.starts_with(['}', ')', ']'])) {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before a let read in place"],
  },
  {
    name: "whole-arm-unseen",
    breaks: "a whole match arm between a comment and the next start isn't seen, so the comment above `Ok(h) => h,` is the `Err` arm's, in the `catch` (rd630)",
    file: "src/to_oxc/openings.rs",
    find: "                if ended {\n",
    replace: "                if false {\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "Ok arm is above its value"],
  },
];

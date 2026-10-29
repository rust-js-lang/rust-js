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
    breaks: "a comment before an arm whose first statement is a method call, `DOWN.set(true)`, whose span starts at `set`, moves above the `switch` (rd119, ADR 0905 amended)",
    file: "src/to_oxc/openings.rs",
    find: "        || (code.strip_suffix('.')).is_some_and(|path| {\n",
    replace: "        || (code.strip_suffix('.')).filter(|_| false).is_some_and(|path| {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
];

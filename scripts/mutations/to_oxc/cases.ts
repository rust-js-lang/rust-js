// Mutations of src/to_oxc/cases.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fallthrough-unsaid",
    breaks: "a case that falls through to the next has no `// Fallthrough` before the next, which ESLint's `no-fallthrough` asks",
    file: "src/to_oxc/cases.rs",
    find: "    (before.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    replace: "    (false && before.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    tests: ["test/lowering-switch-fallthrough.test.ts", "-t", "falls through to its case"],
  },
  {
    name: "default-fallthrough-unsaid",
    breaks: "a case that falls through to the `default` has no `// Fallthrough` before it, which ESLint's `no-fallthrough` asks",
    file: "src/to_oxc/cases.rs",
    find: "    (cases.last()?.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    replace: "    (false && cases.last()?.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    tests: ["test/lowering-switch-fallthrough.test.ts", "-t", "falls through to the default"],
  },
  {
    name: "arm-comment-before-every-case",
    breaks: "a comment before any arm of a `switch` is before its `case`, the scheduler's `// Times out immediately` too, not in its body (ADR 0590)",
    file: "src/to_oxc/cases.rs",
    find: "    let mut starts = fallthrough_starts(module);\n",
    replace: "    let mut starts = Vec::new();\n    each_switch(module, &mut |cases, _| starts.extend(cases.iter().filter_map(|c| c.values.first()).filter(|v| !v.span.is_none()).map(|v| v.span.lo)));\n",
    tests: ["test/lowering-matches.test.ts", "-t", "a match of constants cases each by its name, its comments in its arms"],
  },
  {
    name: "fallthrough-said-twice",
    breaks: "a case the program's `// falls through` is before gets `// Fallthrough` beside it too",
    file: "src/to_oxc/cases.rs",
    find: "        .any(|said| text.contains(said))\n",
    replace: "        .any(|said| said.is_empty() && text.contains(said))\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^option_constants_switch.rs$"],
  },
  {
    name: "pattern-comment-in-body",
    breaks: "a comment between a match arm's patterns, `\"keydown\" // .. | \"keyup\"`, is in the case's body, not before `case \"keyup\":`",
    file: "src/to_oxc/cases.rs",
    find: "        starts.extend(values.filter(|v| !v.span.is_none()).map(|v| v.span.lo));\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment between an arm"],
  },
  {
    name: "stacked-case-starts-of-one-pattern",
    breaks: "a comment before an arm of one pattern, which ADR 0590 puts in its body, is before a start no clause is printed at, and lost, `// Track the input node that has focus.` (rd143)",
    file: "src/to_oxc/cases.rs",
    find: "    case.values.len() > 1 || (!case.values.is_empty() && case.body.iter().all(|s| s.span.is_none()))\n",
    replace: "    !case.values.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
  {
    name: "block-case-start-unlisted",
    breaks:
      "a case of one pattern whose arm opens a block isn't a start a comment above its arm is before, and the comment stays in its body (rd166)",
    file: "src/to_oxc/cases.rs",
    find: "        starts.extend(single.filter_map(|case| clause_start(source, &case.body)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
  {
    name: "default-pattern-comment-unread",
    breaks:
      "a `_ /* \"{\" \"[\" */ => {` arm isn't read as its `default`'s opening, so the `// Fallthrough` above it is lost from before `default:` (rd525)",
    file: "src/to_oxc/cases.rs",
    find: "    while let Some(open) = code.find(\"/*\")\n",
    replace: "    while false\n        && let Some(open) = code.find(\"/*\")\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "whose pattern has a comment"],
  },
  {
    name: "arm-opening-before-its-if-unseen",
    breaks:
      "a `_` arm whose first statement is an `if` whose span starts at its test, `if __DEV__ && row.is_empty() {`, has no clause start, so the `// Fallthrough` above it is stacked above the `switch` (rd525)",
    file: "src/to_oxc/cases.rs",
    find: "    if code[line..].trim() == \"if\"\n        && line > 0\n",
    replace: "    if false\n        && line > 0\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "whose pattern has a comment"],
  },
  {
    name: "default-let-start-unread",
    breaks:
      "a `_` arm whose block opens with a `let`, whose span is its value's, `let mut tail_node = render_state.tail..`, has no clause start, so the comment above the arm prints above the `switch`, React's cutOffTailIfNeeded's `// Hidden is now the default.` (rd532)",
    file: "src/to_oxc/cases.rs",
    find: "    let mut end = let_start(before).unwrap_or(before.len());\n",
    replace: "    let mut end = before.len();\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "opens with a `let`"],
  },
  {
    name: "default-let-start-of-no-let",
    breaks:
      "a `_` arm's first statement after any line ending in `=` is read as a `let`'s value, so a clause starts where none does; and none of a `let` whose line lacks its `=` (rd532)",
    file: "src/to_oxc/cases.rs",
    find: "    (head.starts_with(\"let \") && head.ends_with('=')).then_some(line)\n",
    replace: "    (head.starts_with(\"let \") && head.ends_with(';')).then_some(line)\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "opens with a `let`"],
  },
  {
    name: "empty-arm-case-at-no-value",
    breaks:
      "a comment above an arm that does nothing, `// Controlled attributes are not validated.` above `\"value\" => {}`, is above the loop's `const name = names[i];`, not its `case \"value\":` (rd136)",
    file: "src/to_oxc/cases.rs",
    find: "    case.values.len() > 1 || (!case.values.is_empty() && case.body.iter().all(|s| s.span.is_none()))\n",
    replace: "    case.values.len() > 1\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guarded_or_pattern\\.rs$"],
    snapshots: true,
  },
];

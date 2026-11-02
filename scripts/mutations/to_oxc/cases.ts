// Mutations of src/to_oxc/cases.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fallthrough-unsaid",
    breaks: "a case that falls through to the next has no `// falls through` before the next, which ESLint's `no-fallthrough` asks",
    file: "src/to_oxc/cases.rs",
    find: "    (before.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    replace: "    (false && before.falls && !first.span.is_none()).then_some(first.span.lo)\n",
    tests: ["test/lowering-switch-fallthrough.test.ts", "-t", "falls through to its case"],
  },
  {
    name: "default-fallthrough-unsaid",
    breaks: "a case that falls through to the `default` has no `// falls through` before it, which ESLint's `no-fallthrough` asks",
    file: "src/to_oxc/cases.rs",
    find: "    let fallen = cases.last()?.falls && !first.span.is_none();\n",
    replace: "    let fallen = false && cases.last()?.falls && !first.span.is_none();\n",
    tests: ["test/lowering-switch-fallthrough.test.ts", "-t", "falls through to the default"],
  },
  {
    name: "default-fallen-to-past-its-open",
    breaks:
      "a `default` fallen to whose `_` arm opens a block is printed at its first statement, so the program's `// Fallthrough` above the arm is lost and rust-js's `// falls through` written in its place (rd638)",
    file: "src/to_oxc/cases.rs",
    find: "    fallen.then(|| clause_start(source, default).unwrap_or(first.span.lo))\n",
    replace: "    fallen.then(|| first.span.lo)\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "above a block's"],
  },
  {
    name: "arm-comment-before-every-case",
    breaks: "a comment before any arm of a `switch` is before its `case`, the scheduler's `// Times out immediately` too, not in its body (ADR 0590)",
    file: "src/to_oxc/cases.rs",
    find: "    let mut starts = fallthrough_starts(module, source);\n",
    replace: "    let mut starts = Vec::new();\n    each_switch(module, &mut |cases, _| starts.extend(cases.iter().filter_map(|c| c.values.first()).filter(|v| !v.span.is_none()).map(|v| v.span.lo)));\n",
    tests: ["test/lowering-matches.test.ts", "-t", "a match of constants cases each by its name, its comments in its arms"],
  },
  {
    name: "fallthrough-said-twice",
    breaks: "a case the program's `// falls through` is before gets `// falls through` beside it too",
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
    breaks: "a case of more than one value is printed where none is written, so a comment or a blank line above its arm, `\"rp\" | \"rt\" => ..`, is in its body, not above its first `case` (rd143)",
    file: "src/to_oxc/cases.rs",
    find: "    case.values.len() > 1 || written_apart(case, source)\n",
    replace: "    written_apart(case, source)\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "above an arm of patterns is above its first case"],
  },
  {
    name: "block-case-start-unlisted",
    breaks:
      "a case whose arm opens a block, or is guarded and does nothing, isn't a start a comment above its arm is before, and the comment is its body's first statement's, `if (skip)`'s on the guard's line (rd166, rd619)",
    file: "src/to_oxc/cases.rs",
    find: "        starts.extend(apart.map(|case| case.values[0].span.lo));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
  {
    name: "default-pattern-comment-unread",
    breaks:
      "a `_ /* \"{\" \"[\" */ => {` arm isn't read as its `default`'s opening, so the `// falls through` above it is lost from before `default:` (rd525)",
    file: "src/to_oxc/cases.rs",
    find: "    while let Some(open) = code.find(\"/*\")\n",
    replace: "    while false\n        && let Some(open) = code.find(\"/*\")\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "whose pattern has a comment"],
  },
  {
    name: "arm-opening-before-its-if-unseen",
    breaks:
      "a `_` arm whose first statement is an `if` whose span starts at its test, `if __DEV__ && row.is_empty() {`, has no clause start, so the `// falls through` above it is stacked above the `switch` (rd525)",
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
    name: "block-case-comment-in-body",
    breaks:
      "a comment above an arm of one pattern that opens a block, `// Don't fire the event ..` above `\"mousedown\" => {`, is in its case's body, not before its `case` (rd166, rd619)",
    file: "src/to_oxc/cases.rs",
    find: "        false => !head.contains(['{', '}', ';']) && body.starts_with('{'),\n",
    replace: "        false => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^(switch_guard_skips_its_case|switch_guarded_or_pattern).rs$"],
    snapshots: true,
  },
  {
    name: "guarded-nothing-case-comment-in-body",
    breaks:
      "a comment above a guarded arm that does nothing, `// Chrome and IE fire ..` above `\"selectionchange\" if skip => {}`, is in its case, before the guard's `if`, not above `case \"selectionchange\":` as upstream writes it (rd619)",
    file: "src/to_oxc/cases.rs",
    find: "        true => body\n",
    replace: "        true => false && body\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
  {
    name: "guarded-doing-case-comment-above",
    breaks:
      "a comment above a guarded arm that does something, `// Remove unwanted keypresses.` above `\"keypress\" if char_code(..) == 0 => return None`, is above its `case`, not before the guard's `if` in it, as upstream's SimpleEventPlugin writes it (rd619)",
    file: "src/to_oxc/cases.rs",
    find: "        true => body\n",
    replace: "        true => true || body\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_text_guard_falls.rs$"],
    snapshots: true,
  },
  {
    name: "commented-arm-written-apart",
    breaks:
      "a case whose arm is only comments, `3 => { // The last case .. }`, is printed where its value is, and its comments, the clause's own, are lost (rd619)",
    file: "src/to_oxc/cases.rs",
    find: "    !commented\n",
    replace: "    (commented || !commented)\n",
    tests: ["test/lowering-comments-ends.test.ts", "-t", "a last arm of only comments keeps them in its clause"],
  },
];

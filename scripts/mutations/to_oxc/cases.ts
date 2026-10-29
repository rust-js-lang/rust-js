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
    find: "        let stacked = cases.iter().filter(|case| case.values.len() > 1);\n",
    replace: "        let stacked = cases.iter().filter(|case| !case.values.is_empty());\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_skips_its_case.rs$"],
    snapshots: true,
  },
];

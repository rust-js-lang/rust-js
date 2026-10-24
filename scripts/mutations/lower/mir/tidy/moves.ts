// Mutations of src/lower/mir/tidy/moves.rs (ADR 0093): a temporary's
// value moved where what the program does would change.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-captured-kept",
    breaks: "what an element made before a branch reads stays a `const className = cn(…)`, not read where it's written",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !is_temporary(name) && !element {\n",
    replace: "    if !is_temporary(name) {\n",
    tests: ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-element-read-held",
    breaks: "what an element made ahead of a branch reads stays a `const` where elements before it come first, `className$1`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        true if element => {\n",
    replace: "        true if false && element => {\n",
    tests: ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-inlined-no-backtrack",
    breaks: "of temporaries one after another, only the last is read where it's read, `cn(_8, _14, x)`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        i = i.saturating_sub(1);\n",
    replace: "",
    tests: ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-inlined-not-past",
    breaks: "a temporary is read in place only by the statement right after it, `const _34 = formatted; const args$1 = args;` kept",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !past {\n",
    replace: "    if !past || at > 0 {\n",
    tests: ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"],
  },
  {
    "name": "target-root-not-inlined",
    "breaks": "`const _25 = ref.current; _25[k] = v` stays, not `ref.current[k] = v` (and no `??=` of it)",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "                (true, true) => Some(Read::Target),\n",
    "replace": "                (true, true) => None,\n",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-past-borrow-blocks",
    "breaks": "a chain stays a `const _4` ahead of a callback's borrow, `const list$1 = list;`, not the array's own `forEach`",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "    if !past {\n",
    "replace": "    if !past || (acts && at > 0) {\n",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-guarded-read-blocked",
    "breaks": "a value made where its test holds isn't read where that test holds, `const className$2 = error != null ? cn(..) : undefined` kept",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "            if guard.is_some_and(|g| same_test(a, g))",
    "replace": "            if false && guard.is_some_and(|g| same_test(a, g))",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "mir-pure-value-held-past-calls",
    "breaks": "what does nothing, of a `let` no closure sets, `showToc && toc.length !== 0`, is held ahead of the calls before its read",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "unchanged_by_calls || !e.has_effects()",
    "replace": "!e.has_effects()",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "inlined-pure-not-past-statements",
    "breaks": "what does nothing, of variables no statement between assigns, `version === \"Canary\"`, stays held before a loop",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "\n        || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)));",
    "replace": ";",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "jsx-key-read-in-written-order",
    "breaks": "a `key` that does something is read in place, made after the children by JS's JSX runtime, `[1, 3, 2]`",
    "file": "src/lower/mir/tidy/moves.rs",
    "find": "                && let Some(found) = leads(key, name, guard, passes)\n",
    "replace": "                && false\n                && let Some(found) = leads(key, name, guard, passes)\n",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    name: "closure-of-statements-called-in-place",
    breaks: "a closure of statements is moved into its call, `x ? ((l) => { .. })(x) : undefined`, not called by its name",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                left_called |= applied_in_place(value, args).is_none();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(adapter_drops|tuple_scrutinee_calls).rs$"],
    snapshots: true,
  },
  {
    name: "inlined-not-past-a-method",
    breaks: "a call's value MIR holds before a branch stays a `const value$1 = ..`, not read in `out.push(..)`'s arguments past the method",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        ExprKind::Call(f, args) if let ExprKind::Member(receiver, _) = &f.kind => {\n",
    replace: "        ExprKind::Call(f, args) if false && let ExprKind::Member(receiver, _) = &f.kind => {\n",
    tests: ["test/lowering-moves.test.ts", "-t", "past the method"],
  },
  {
    name: "inlined-not-into-blocks",
    breaks: "a test MIR holds before the blocks its arms leave stays a `const value = ..`, not read where it's tested",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        StmtKind::Labeled(_, body) if js::mentions_in(after, name) == 0 => entered(stmt, body, facts),\n",
    replace: "        StmtKind::Labeled(_, body) if false && js::mentions_in(after, name) == 0 => entered(stmt, body, facts),\n",
    tests: ["test/lowering-moves.test.ts", "-t", "block after it"],
  },
  {
    name: "binding-method-lookup-passed",
    breaks: "a call's value is read past a binding's method of a JS value, which a getter may answer, looked up before it runs",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                && f.known == js::Known::Getter =>\n",
    replace: "                && false =>\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a binding's method is looked up after its arguments run"],
  },
];

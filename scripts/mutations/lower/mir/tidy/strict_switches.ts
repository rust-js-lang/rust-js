// Mutations of src/lower/mir/tidy/strict_switches.rs (ADR 0958).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/mir/tidy/strict_switches.rs";
const switches = ["test/lowering-strict-switches.test.ts", "-t", "React's matchSelector's chain of strict_equals tests is a switch"];
const stays = ["test/lowering-strict-switches.test.ts", "-t", "a chain of strict_equals tests a switch would change stays"];

export const mutations: Mutation[] = [
  {
    name: "strict-switch-of-no-strict-equals",
    breaks: "a chain of `js::strict_equals` tests, React's matchSelector's, is an `if` chain, not upstream's `switch (selector.$$typeof)` (ADR 0958)",
    file,
    find: "    (strict && s.reads_same() && !value.has_effects()).then_some((&**s, &**value))\n",
    replace: "    (false && s.reads_same() && !value.has_effects()).then_some((&**s, &**value))\n",
    tests: switches,
  },
  {
    name: "strict-switch-of-a-subject-read-again",
    breaks: "a chain whose subject a call reads, `next() === a`, is a `switch` that calls it once, not once a test (ADR 0958)",
    file,
    find: "    (strict && s.reads_same() && !value.has_effects())",
    replace: "    (strict && !value.has_effects())",
    tests: stays,
  },
  {
    name: "strict-switch-of-a-value-that-does-something",
    breaks: "a case's value that does something, `s === next()`, is a `switch`'s, of a subject read once (ADR 0958)",
    file,
    find: "    (strict && s.reads_same() && !value.has_effects())",
    replace: "    (strict && s.reads_same())",
    tests: stays,
  },
  {
    name: "strict-switch-taking-a-loops-break",
    breaks: "a chain in a loop whose branch `break`s it is a `switch`, which the `break` leaves instead of the loop (ADR 0958)",
    file,
    find: "                && !chain.bodies().any(|b| breaks(b, None))\n",
    replace: "\n",
    tests: stays,
  },
  {
    name: "strict-switch-of-mixed-subjects-part",
    breaks: "a chain testing two subjects is a `switch` of its tail's, not the chain written (ADR 0958)",
    file,
    find: "                && !chain.tests.iter().any(|t| mixed.contains(t))\n",
    replace: "\n",
    tests: switches,
  },
  {
    name: "strict-switch-of-two-ways",
    breaks: "an `if`/`else if` of two tests is a `switch`, not the two ways a person writes (ADR 0958)",
    file,
    find: "                && chain.cases.len() + usize::from(chain.default.is_some()) >= CASES\n",
    replace: "                && chain.cases.len() + usize::from(chain.default.is_some()) >= 2\n",
    tests: switches,
  },
  {
    name: "strict-switch-guard-not-written-with-else",
    breaks: "guards MIR's structure made whose `else` isn't what's after them are a `switch` whose `default` is, not as written (ADR 0958)",
    file,
    find: "            None if ending && written().is_some_and(|(_, t, o)| of(t, then) && of(o, after)) => {\n",
    replace: "            None if ending => {\n",
    tests: stays,
  },
  {
    name: "strict-switch-dropping-what-follows-an-else",
    breaks: "an `if` chain in an `else` with statements after it is a `switch` of the chain, dropping them (ADR 0958)",
    file,
    find: "            Some(next) if first || after.is_empty() =>",
    replace: "            Some(next) =>",
    tests: switches,
  },
  {
    name: "strict-switch-of-a-returned-conditional",
    breaks: "a chain of values whose last two ways are a returned `?:` stays `if`s and the `?:` (ADR 0958)",
    file,
    find: "            && let ExprKind::Cond(test, yes, no) = &returned.kind\n",
    replace: "            && let ExprKind::Cond(test, yes, no) = &returned.kind\n            && false\n",
    tests: switches,
  },
  {
    name: "strict-switch-returned-value-where-written",
    breaks: "a chain of values, React's selectorToString's, whose one `return` MIR's structure moved into each branch, stays guards (ADR 0958)",
    file,
    find: "        StmtKind::Return(Some(value)) if !value.span.is_none() => value.span,\n",
    replace: "",
    tests: switches,
  },
];

# 0883. A closure written as its call's value is an arrow of that value

Status: Accepted. Extends [0040](0040-jsx.md)'s concise handler to any
closure written so, and amends [0865](0865-a-blank-line-is-where-the-rust-has-one.md)
(a blank line before an `if` whose test has a comment) and
[0882](0882-a-field-of-statements-is-a-method.md)'s reading of what an
arrow of a value is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N for each. An arrow of
a value gives that value back where Rust's closure gives `()`, `undefined`
in JS ([0775](0775-a-unit-given-to-js-is-undefined.md)): it's taken only
where the call's JS gives `undefined` too, so what the arrow gives is the
same. A blank line and a temporary's name change nothing that runs.

## Context

React's `act` (react-in-rust's `ReactAct.rs`) printed three differences
from upstream's `ReactAct.js`:

| Rust | Was | Upstream |
|---|---|---|
| `queue_macrotask(Box::new(move \|\| { // c\n recursively_flush(..) }))` | `queueMacrotask(() => {\n // c\n recursivelyFlush(..);\n});` | `queueMacrotask(() =>\n // c\n recursivelyFlush(..),\n);` |
| `queue_microtask(Box::new(move \|\| queue_microtask(callback)))` | `queueMicrotask(() => {\n queueMicrotask(callback);\n});` | `queueMicrotask(() => queueMicrotask(callback))` |
| `}\n\nif\n// $FlowFixMe[..]\nlet Some(result) = result && ..` | `}\nif (\n // $FlowFixMe[..]` | `}\n\nif (\n // $FlowFixMe[..]` |
| `queue.borrow_mut()[i] = callback;` | `const borrowMut = queue;\nborrowMut[$at(borrowMut, i)] = callback;` | `queue[i] = callback;` |

- A closure of one call was `() => f()` only as a JSX handler (0040); any
  other was a block, whether written `|| f()` or `|| { f(); }`.
- An arrow whose one `return` had a comment before it was a block too:
  the printer wrote statements' comments only.
- `queueMicrotask` gave `undefined` by WebIDL, but its binding didn't say
  so, `#[rust_js::returns_undefined]`.
- A blank line before a statement was looked for before the line its
  span starts on: an `if`'s test's, after the comments written between the
  `if` and its test, which aren't blank.
- `alias_variables` reads a temporary of a variable, `const _42 = queue`,
  as the variable; but this one was a `let` until `names/declarations`,
  after it, and `$at`'s check that reads it twice was made later still.

## Decision

**A closure the program wrote as its call's value, `|| f()` or `|| { f()
}` with no `;`, is `() => f()`, where that call's JS gives `undefined`**:
a function of the crate's that gives `()`, a closure of its own, or a
binding that says so (`closure_calls_for_nothing`, which says how it's
written, `OneCall::Value` or `OneCall::Statement`). Tidy's `concise
arrows` returns the call; one written `|| { f(); }` stays a block, as
written, but a JSX handler's (0040). A call whose JS gives something else,
`v.push(x)`'s array's count or a `dyn Fn`'s, keeps the block.

**The webapi crate's bindings say what WebIDL says**: `webapi/generate.ts`
marks each operation whose result is `undefined` with
`#[rust_js::returns_undefined]`. A setter, whose assignment's value is
what it's given, isn't marked.

**An arrow of a value is `=>` its value, a comment before it after the
`=>`**, on lines of its own, as Prettier writes it: the value is an arm, as
a conditional's branch is (ADR 0658), not a statement.

**A blank line before an `if` is where its keyword is**: of the comments
after the `if`, its test's (`if_keyword`), the blank line before is the
`if`'s, whether the test's statement is an `if` or the `return` of the
conditional it's folded into.

**A temporary of a variable that `names/declarations` made a `const` is
the variable** too (`declared aliases`, `alias_variables` run again):
`queue[$at(queue, i)] = callback;`. Its `$at` check stays, as the call
between the test and the write may shrink the array. It's
`alias_variables`' rule, a temporary's: 0879's clone of what a closure
captured is the program's own name, a rule of its own.

### Beside 0882

0882 makes a field of an arrow of statements a method, and keeps an arrow
of a value. A closure written `then: Box::new(move || f())` of a call of
`undefined` is an arrow of a value here, so it stays `then: () => f()`; one
written `move || { f(); }` is statements, a method, `then() { f(); }`. One
rule each: this one says when an arrow is its value, 0882 what a field of
each is.

## Why

- **It's how the program wrote it, and upstream's**: ReactAct's
  `queueMacrotask(() => ..)`, `queueMicrotask(() => queueMicrotask(callback))`,
  React's docs' `return () => clearInterval(id);` and `useEffect(() =>
  save(todos), [todos])`.
- **It gives what Rust's gives**: each call it's taken of gives
  `undefined`, so what reads the arrow's value, `useEffect`'s cleanup or
  `f.call()`, reads `undefined`, as of the block.
- **It's tested**: `test/lowering-arrows.test.ts` reads each closure's
  value with `Function.prototype.call`, of a function of the crate's, a
  statement, a `Vec`'s `push` and `console.log`; the corpus's
  `closures_of_their_value.rs` gives each to a generic function that
  formats what it gives, against native Rust.

## Alternatives

- **Every closure of one call an arrow of it**: `|| v.push(x)` would give
  the array's count, and `useEffect(() => v.push(x))` a cleanup React
  calls. Rejected.
- **`|| { f(); }` an arrow of it too**: two forms the program writes, one
  JS. Rejected, as 0879 keeps what's written.
- **A list of the web APIs that give `undefined`**: a second catalog of
  what WebIDL says. Rejected; the generator says it of each.

## Consequences

- The guide's hooks, `setInterval(() => setTicks(..), 1000)`, `useCallback((n)
  => setPicked(n), [])`, `[on, () => setOn((on) => !on)]`; the playground's
  and todomvc's `return () => controller.abort();`.
- `docs/readability.txt` blessed: `renamed x$1` 718 to 717.5 in 10,000
  lines, `const borrowMut = queue` and its kind gone.

## What holds it

- Mutations: `arrow-of-a-value-kept-a-block`, `arrow-of-a-statement-its-value`
  (`mir/tidy.ts`); `one-call-statement-unsaid` (`jsx.ts`);
  `arrow-value-a-statement-too`, `blank-before-if-test-comment-lost`,
  `blank-before-folded-if-comment-lost` (`to_oxc.ts`);
  `commented-arrow-expression` (`to_oxc.ts`);
  `declared-aliases-kept` (`mir/tidy/stages.ts`); and
  `mir-tidy-concise-handlers-kept`, of a handler of a statement
  (`test/jsx-events.test.ts`).

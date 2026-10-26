# 0277. A variable shadowed by its own value is the variable

Status: Accepted. Amended: another's variable only where nothing runs
before it's read (below); and past a `try`'s start and a binding's call
that runs nothing (below).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust shadows a variable with what's made of it, and every local of a JS
function has a name of its own, so react.dev's errors page read:

```rust
let Type = ElementType::from_unknown(Type);
jsx! { <Type key={key} {...props} /> }
```

```js
const Type$1 = Type;
return <Type$1 key={key} {...props} />;
```

and `let content = content.clone();` before a closure was `const content$1
= content;`. Its original has `<Type ..>` and `content`.

## Decision

**`const n$1 = n;`, a variable shadowed by its own value, is `n`**: each
read of `n$1` reads `n`, and the `const` is gone. Only where `n` holds that
value wherever `n$1` is read: nothing sets `n` after it, nor in a closure,
nor in a loop it's in, where a closure of each turn keeps its own. One of
another name, `let kept = text;`, keeps the name written.

## Why

- **No program can tell**: the two hold one value, the same object or the
  same string, wherever the new one is read.
- **It's the JS a person writes**, and the name the Rust has.
- **It's tested**: a compiler test shadows a captured string and a number
  set before, and keeps one of another name and one a loop sets again,
  whose closures each read their own turn's; mutations alias each.

## Since

- **A closure's own parameter shadowed in it is the parameter too**,
  `(total, item) => total + item.length` of `let item = item.as_str();`,
  as react.dev's createFileMap casts each snippet it's given. Only its
  parameters: nothing outside the closure sets them.

## Amendment: another's variable only where nothing runs before it's read

A local named as a thread-local's module variable, `let count =
COUNT.get(); bump(); count`, was `const count$1 = count; bump(); return
count$1;` until this rule took `count$1` for `count`: nothing in the
function set `count` after it, but `bump()` did, so the function returned
6 where Rust returns 5, a wrong answer. A function's own variables only it
and its closures set, which the rule sees; a module's, or an import, any
code that runs.

**A `const n$1 = n;` of a name the function doesn't declare is `n` only
where each read of `n$1`, in JS's order, is before anything that may run
code**: a call, `new`, an `await`, a JSX element, a binding's getter or
setter. A call's callee and arguments are read before it runs, `if
(holder != null) { bump(holder); }`; a branch's way is its own; a loop, a
`try` or a `switch` that reads it runs nothing. Else it stays its copy,
`const count$1 = count; bump(); return count$1;`, and so does one read
past a binding's property, which a getter may answer, `const saved$1 =
saved; const now = button.onclick;`. The function's own names, its
parameters and what its blocks declare, a `catch`'s, a loop's and a
function's too, are as before; so are a closure's.

The rule is `src/prepare/shadows.rs`'s now, out of `prepare.rs`.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)):
Rust's answer, preserved. Tested by `test/corpus/shadow_of_module_variable.rs`
against native Rust; the playground's `setTheme` and the bindings test's
`same` keep their copy past a binding's method and getter. Its mutations
are `shadow-of-outer-variable-aliased`, `outer-shadow-read-past-a-call` and
`outer-shadow-read-before-a-call-copied`.

## Amendment: past a `try`'s start, and a binding's call that runs nothing (2026-10-26)

The rule above kept React's reconciler's copies where upstream reads the
variable:

```js
// ReactFiberAsyncAction.js, startIsomorphicDefaultIndicatorIfNeeded
const isomorphicDefaultTransitionIndicator$1 = isomorphicDefaultTransitionIndicator;
if (typeof isomorphicDefaultTransitionIndicator$1 === 'function' && !pendingIsomorphicIndicator) {
  try {
    const cleanup = isomorphicDefaultTransitionIndicator$1();
// ReactFiberHotReloading.js, scheduleFibersWithFamiliesRecursively
const failedBoundaries$1 = failedBoundaries;
if (failedBoundaries$1 && (failedBoundaries$1.has(fiber) || (alternate != null && failedBoundaries$1.has(alternate)))) {
```

Between the copy and each read lies nothing that can set the variable:
in the first, a read of another module variable, then a `try` whose
first act is the call that reads it, its callee read before it runs;
in the second, a weak set's `has(..)`. A module's variable is set only by
its module's code (ADR 0270), so only code of the program's that runs
between can set it; what's in question is what each call may run.

- **A `try`'s block is read in its order, as any block is, and its
  `catch` or `finally` after all of it**: either may run after any of
  the block, which may have run code before it threw. So the indicator's
  call in its `try` is the variable's, `isomorphicDefaultTransitionIndicator()`,
  and a copy read in the `catch` after a call that set the variable, as
  Rust reads what it copied, stays a copy: `const level$1 = level; try {
  return raise(); } catch { return level$1 + 100; }`.
- **A binding's call that runs none of the program's code sets none of
  its variables**, `failedBoundaries.has(fiber)`. Which calls these are
  is the bindings' own answer, ADR 0867's and ADR 0898's, which
  `cell_borrows::binding_call_runs_nothing` gives: one marked
  `#[rust_js::runs_nothing]`, a weak collection's methods, `new
  AggregateError(..)`; or one marked `#[rust_js::runs_only_given]` given
  only numbers and text, `console.error("..")`. Lowering marks its call
  so (`Known::RunsNothing`), and the rule reads the mark; its callee and
  arguments are read as any part is, so a call among them still counts.
- **Any other binding's call may run anything**, given nothing too: JS
  reaches the program through a callback it holds, one given now or
  before, `dispatchEvent` running a listener added earlier, and through
  any function the program exports. Telling which of those may set a
  variable is a whole program's question, which the rule doesn't ask;
  only the binding's author knows it runs none, as they know a property
  is data (ADR 0301). So `before.call(); return failedBoundaries$1.has(fiber);`
  keeps its copy.

Case "results change" ([0262](0262-when-rust-and-js-disagree.md)), as
before: Rust's answer, preserved, the copy elided only where no code that
can set the variable runs before a read. What the rule assumes of a
marked binding is what the borrow check assumes of it already (ADR 0867):
a binding marked wrongly is wrong for both.

**Alternatives:**

- **Every binding's call runs nothing of the program's** unless it's given
  a callback: wrong of `el.click()` and `dispatchEvent`, which run a
  listener given before, and of a host's JS calling an export.
- **The variable's writers, by whether JS can reach them**: a whole
  program's escape analysis, every function given to JS and every export,
  where React's writers, `markFailedErrorBoundaryForHotReloading` among
  them, are exported anyway.

Tested by `test/corpus/shadow_of_module_variable_in_try.rs` against
native Rust, a copy read in a `try` before and after a call that sets the
variable and in its `catch`, whose kept copies, `indicator$1` and
`level$1`, are what it's of: `docs/readability.txt`'s `renamed x$1` counts
10 more with it, 8026 to 8036; and
`test/lowering-shadows.test.ts`, "a copy
of a module's variable read past a call that runs nothing is the
variable", React's `failedBoundaries`, and one past `before.call()`, a
callback that sets the variable, run as JS. Its mutations are
`shadow-catch-read-before-its-block`, `shadow-read-in-try-copied`,
`runs-nothing-runs-code` (`prepare/shadows.rs`),
`binding-call-runs-nothing-unmarked` and `every-binding-call-runs-nothing`
(`lower/binding_calls.rs`).

# 0942. Whether a global is a function is `js::is_function`'s

Status: Accepted. Extends [0597](0597-whether-the-host-has-a-global-is-is-defined.md),
with [0624](0624-is-defined-of-a-path-of-bindings.md)'s paths and
[0645](0645-is-defined-of-a-static-is-of-the-static.md)'s statics, to a
second question; and [0887](0887-std-items-that-are-math-are-math.md)'s
`Math`, which every host has, to ECMAScript's first edition's globals.
Amended: a chain of its tests reads as written, a comment before it kept;
and of a value of a function's type JS gives, it asks the value; and of a
path it asks the last part alone, not each before it (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): A for `is_function`:
what's asked of the host, as JS asks it. N for the globals not asked of:
every JS host has them, so asking is always `true` and nothing differs.

## Context

React asks whether a global is a function, not whether it's there:

```js
if (__DEV__ && typeof Object.preventExtensions === 'function') {
  Object.preventExtensions(this);
}
```

and its scheduler, `typeof setTimeout === 'function'`. ADR 0597 gave one
question, `js::is_defined(binding)`, `typeof X !== "undefined"`, so the
port wrote `typeof Object !== "undefined" && typeof
Object.preventExtensions !== "undefined"`: a test that's true of a
`preventExtensions` a host or a test set to a number, which React's isn't,
and asks of `Object`, which every host has. There was no way to write
upstream's test.

## Decision

The user's decision: **whether a global is a function is its own
question, `js::is_function(binding)`**, `typeof X === "function"`, given
what `js::is_defined` is given, in the same forms: a binding, a static
given as itself, or a tuple of bindings for a path:

```rust
pub fn can_prevent_extensions() -> bool {
    js::is_function(js::object::prevent_extensions::<js::Unknown>)
}
```

```js
export function canPreventExtensions() {
  return typeof Object.preventExtensions === "function";
}
```

Each part of a path before the last is asked whether it's there, as
`js::is_defined` asks it, `typeof rustJsKit !== "undefined" && typeof
rustJsKit.hook === "function"`, and only the last whether it's a
function (superseded below: only the last is asked). A refusal names the
function the program wrote.

**A path's global that's one of ECMAScript's first edition's, every
host's, isn't asked of**: `Object`, `Function`, `Array`, `String`,
`Boolean`, `Number`, `Math` and `Date`. ADR 0887 said so of `Math`; it's
one rule of which parts a host may lack, for both questions, so
`js::is_defined(js::object::prevent_extensions::<js::Unknown>)` is
`typeof Object.preventExtensions !== "undefined"` now. A later edition's
global, `Symbol` or `Reflect`, is asked of as before, and a global that's
the whole path, `js::is_defined(js::date::new)`, is asked: the program
asked of it.

`bindings::JsForm::Defined` and `JsForm::Function` are their binding forms,
and `mir_defined_parts` is the one place their tests are made.

## Why

- **It's the JS people write**, where they write it: React's test, as it
  reads, and the one that's false of a non-function.
- **One question, one function**: "is it there" and "can it be called"
  differ for a global set to a value, so neither stands for the other;
  each has one form.
- **The same forms as `is_defined`**: a person who knows one knows the
  other.
- **No `typeof` of what's always there**: a guard that's always true is
  noise, and ECMAScript's first edition's globals are the spec's, not a
  list of which hosts lack what.

## Alternatives

- **`js::is_defined` is `=== "function"` of a function binding.**
  Rejected: `typeof MessageChannel !== "undefined"`, which React writes of
  a class, would change, and one function would ask two things.
- **`js::type_of(..) == "function"`** of the binding's value. Rejected:
  reading a missing global throws before `typeof` sees it, and it's text,
  not a binding Rust checks.
- **Ask every part, `Object` too.** Rejected: always true, and not what
  React writes.
- **Leave out every ECMAScript global, the `js` crate's.** Rejected:
  `Symbol`, `Reflect`, `Iterator`, newer editions', aren't every host's.

## Consequences

- `js::is_defined` of a path through `Object`, `Array`, `Number` and the
  rest of the first edition's globals no longer asks of that global.
- The port writes `js::is_function(..)` where upstream writes `typeof X
  === 'function'`.

## What holds it

- `test/lowering-is-function.test.ts`.
- The guide's [global scope](../guide/interop/global-scope.md#a-global-some-hosts-lack).
- Mutations: `is-function-asks-defined`, `is-function-form-called`,
  `is-function-refusal-named-defined`, `every-host-has-asked`,
  `every-host-has-unasked-whole`.

## Amendment: a chain of its tests reads as written, a comment before it kept

React's ReactFiberPerformanceTrack writes

```js
const supportsUserTiming =
  enableProfilerTimer &&
  typeof console !== 'undefined' &&
  typeof console.timeStamp === 'function' &&
  (!__DEV__ ||
    // In DEV we also rely on performance.measure
    (typeof performance !== 'undefined' && ..));
```

Its port's `ENABLE_PROFILER_TIMER.with(|&enabled| enabled) &&
js::is_defined(console) // ..comment.. && js::is_function(console::time_stamp)`
printed `!!(enableProfilerTimer && typeof console !== "undefined") &&
typeof console !== "undefined" && typeof console.timeStamp === "function"`:

- **A `bool` thread-local's value a `with` lends is a `bool`**
  (`Known::Boolean`, as a `bool` constant's read is, ADR 0031): the
  value tidy makes of the initializer's branches needs no `!!`, which had
  split the chain so ADR 0964's `typeof console` already asked stayed.
- **The tests `is_defined` and `is_function` make are where the call is
  written**, each of its span, so a comment written before the call is
  before its first test; it had no place, and was lost.

Case N for both: the same values. Held by
`test/lowering-is-function.test.ts`'s "a thread-local bool of another and
is_function's chain read as written"; mutations `local-with-bool-unknown`
and `defined-test-unplaced`.

## Amendment (2026-10-28): of a value of a function's type, it asks the value

React's SchedulerMock (and Scheduler) runs a task's callback only if it's a
function, and keeps what it gives back only if that is one:

```js
const callback = currentTask.callback;
if (typeof callback === 'function') {
  ..
  const continuationCallback = callback(didUserCallbackTimeout);
  if (typeof continuationCallback === 'function') {
```

Its test "gracefully handles scheduled tasks that are not a function"
schedules `null`, `undefined`, `{}` and `42`. A port's task holds
`Option<Callback>`, a transparent struct of a `Box<dyn FnMut(bool) ->
Option<Callback>>`, whose `Some` was `callback != null`, so `{}` and `42`
were called: a `TypeError` where upstream skips them. Nothing could ask
it: `js::is_function` took a global's binding, and refused anything else.

**`js::is_function` of a value of a function's type that JS may give is
`typeof value === "function"`**: one argument, a `fn`, a `dyn Fn` lent or
boxed, a type parameter's `impl Fn`, a transparent struct of one, or an
`Option` of one, not of an `Option` (`asks_of_value`, `function_given`):

```rust
if let Some(Callback(mut callback)) = callback
    && js::is_function(&callback)
{
```

```js
if (typeof callback === "function") {
```

- **It's the one question, "can it be called", of a value now**, as
  ADR 0942 made it of a global: a person who knows `typeof X ===
  "function"` writes `js::is_function`. A global's binding, a static and
  a path are asked as before; a static of a function's type is asked as
  the global it is, unread.
- **The value is read**, where a global's argument isn't
  (`unread_defined_operands` leaves it, ADR 0647): reading a variable
  throws nothing.
- **A closure or a function item the program makes is refused**, as
  before: only Rust makes one, so it's always a function.
- **An `Option` of one is `null` or `undefined` where it's `None`**,
  which `typeof` says isn't a function, so `js::is_function(&callback)`
  asks of the `Option` as of its value; and an `if let Some(..) = x &&
  js::is_function(..)` needs no `!= null` before it, which the implied
  test drops.
- **Rust is told nothing**, as of a global: natively a value of a
  function's type is always one. Case A
  ([0262](0262-when-rust-and-js-disagree.md)): what's asked of JS, as JS
  asks it.

Alternatives:

- **`js::classify(js::unknown(&callback))`'s `Kind::Function`**: the
  `&Unknown` form, which reads as `typeof` too, but of a value cast to
  any value first, and its `Kind::Function` a `&dyn Fn() -> ..` of no
  parameters, not the callback to call.
- **Typing the task's callback `Option<&Unknown>`**, as ADR 0910's values
  are: then it's called as `function::call(f, None, (x,))`, which
  upstream's `callback(didUserCallbackTimeout)` isn't.
- **A function of its own, `js::is_callable(&value)`**: a second way to
  ask one question.

Held by `test/lowering-is-function.test.ts`, "js::is_function of a value
of a function's type is whether JS gave a function", each form, run of
`null`, `undefined`, `{}`, `42`, a string and a function; its refusals
in "js::is_function of what isn't a global's binding is refused"; the
guide's `run` (interop/global-scope.md). Mutations `is-function-of-no-value`,
`is-function-value-of-an-option-refused`,
`is-function-value-of-a-transparent-struct-refused`,
`is-function-value-of-the-crates-function` (`mir/calls.ts`) and
`is-function-value-unread` (`mir/temporaries.ts`).

## Amendment (2026-10-28): of a path, it asks the last part alone

react-dom-bindings' estimateBandwidth asks

```js
if (typeof performance.getEntriesByType === 'function') {
```

which throws a `ReferenceError` where the host has no `performance`. Its
port, `js::is_function(Performance::get_entries_by_type)`, was `typeof
performance !== "undefined" && typeof performance.getEntriesByType ===
"function"`, false there: another result than upstream's (gap rd22). And
nothing could write upstream's test, where the guarded one has a spelling
of its own, the program asking whether the global is there first.

**`js::is_function` of a path asks only its last part, whether it's a
function**: `typeof performance.getEntriesByType === "function"`, which
throws as JS's does where a part before it is missing, as reading it
would. A program whose host may lack the global asks so, as JS writes it:

```rust
js::is_defined(performance) && js::is_function(Performance::get_entries_by_type)
```

```js
typeof performance !== "undefined" && typeof performance.getEntriesByType === "function"
```

- **`js::is_defined` asks each part still** (ADR 0624, decided with the
  user): its question is whether the host has it, which a missing part
  before it answers, as React's ExecutionEnvironment writes `typeof window
  !== 'undefined' && typeof window.document !== 'undefined' && ..`.
  `js::is_function`'s is whether what's read is a function, which JS asks
  of the read alone.
- **Each test written is kept**: `js::is_defined(x) && js::is_function(x)`
  asks both, as written (ADR 0964 drops only a `typeof` an earlier test of
  the `&&` already asked).
- Case ([0262](0262-when-rust-and-js-disagree.md)): A, what's asked of
  the host as JS asks it; the result, a `ReferenceError` where the global
  is missing, is upstream's.

Held by `test/lowering-is-function.test.ts`'s "js::is_function is whether
a binding's global is a function": `typeof rustJsKit.hook === "function"`
throwing where `rustJsKit` is missing, and `is_defined` before it false
there; `typeof Symbol.for === "function"` of "js::SymbolFor is
Symbol.for"; the guide's `logRender`. Mutation `is-function-asks-each-part`
(`mir/calls.ts`).


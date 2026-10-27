# 0841. A nullable place stores what's never `undefined` as it is

Status: Accepted. Amended: a nullable function's result, kept in a
variable or a `Drop` guard's field, is stored as it is, and a trait's
method's isn't taken as one; stored again where it's read in place; an
`and_then` of one is `null` for `None` wherever it's kept. Amends [0791](0791-a-cells-storage-says-what-it-stores.md),
whose `Storage::written` kept as it is only a constant and what MIR's
`nulled` says; and [0781](0781-a-none-only-nullables-read-is-null-where-its-made.md),
whose variable only nullable fields read was of `Option`s made, now of a
call's too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A nullable place's
`None` is `null` either way; what's written is what was, without a `??
null` that changes nothing.

## Context

React's port wrote a `?? null` of what can't be `undefined`:

```js
// createContext: a Some of an object made there, written through a handle
context.Consumer = {$$typeof: REACT_CONSUMER_TYPE, _context: context} ?? null;
// lazy: a map's None, undefined, then coalesced
debugTask: (createTask ? createTask('lazy()') : undefined) ?? null,
```

Upstream writes `context.Consumer = { .. }` and `debugTask:
console.createTask ? console.createTask('lazy()') : null`.
`Storage::written(made, nulled)`, the one answer to what a nullable place
stores, coalesced whatever wasn't a constant or `nulled`, and asked
again with `nulled` false where tidy writes a handle's place in its stead
(`read_handles`), where MIR's answer was gone. A binding's nullable
parameter answered it a second time, the same rule in `mir/calls.rs`.

## Decision

**What's never `undefined` is stored as it is.** `Expr::may_be_undefined`
is the one answer of whether a JS value may be: never a constant but
`undefined`, `null` among them, nor an object, an array, a function, a
`new`, a template, a regex or JSX made there, nor what an operator gives
but an `&&`, `||` or `??` that may give an operand that may be, nor a test
of two that aren't; nor what lowering knows isn't, `Known::Defined`.
Anything else, a variable or a call, may be.

- **Lowering says what it knows**: what MIR's `nulled` says is `null` or
  a value already, set through a variable holding a cell's handle, is
  marked `Known::Defined`, so the write tidy makes of it to the place,
  as the handle's storage stores (`read_handles`), keeps it,
  `context.Consumer = consumerOf(context);`; and an `Option`'s `map`
  marks its `Some`, not boxed, its value, which can't be `undefined`
  (ADR 0051).
- **A test's branches each as they're stored**: `c ? f(c) : undefined` is
  `c ? f(c) : null` where `f(c)` is never `undefined`; else the whole is
  `?? null`.
- **A call's `Option` only nullable fields read is written where the
  call's made** (`nullable_written`, ADR 0781's `null_nones`), its `None`
  `null`: `debugTask: createTask ? createTask("lazy()") : null`. Not a
  clone, which `nulled` gives what it's a clone of.
- **One answer**: a binding's nullable parameter asks
  `Storage::written` too.

| Rust | JS |
|---|---|
| `context.consumer.set(Some(Box::leak(Box::new(Consumer { .. }))))` | `context.Consumer = { $$typeof: REACT_CONSUMER_TYPE, _context: context };` |
| `context.consumer.set(Some(consumer_of(context)))` | `context.Consumer = consumerOf(context);` |
| `context.consumer.set(consumer)`, an `Option` given | `context.Consumer = consumer ?? null;` |
| `debug_task: console::create_task().map(\|t\| t("lazy()"))` | `debugTask: createTask ? createTask("lazy()") : null` |

## Why

- **It's the JS a person writes**, and what React writes; and the same
  value: `?? null` of what's never `undefined` gives it back.
- **One owner, told what it can't see**: the JS's form says what it can;
  lowering's facts ride on the expression (`Known`), as a getter's read
  does (ADR 0672), so no later pass guesses them.

## Alternatives

- **Each caller checks its value's form**: three answers to one
  question. Rejected.
- **Tidy's handle write skips storing**: a write the cell's call didn't
  store, a closure's through the variable it captured (#192), needs it.
  Rejected.

## Consequences

- A `#[rust_js::nullable]` field given `n.map(|n| n + 1.0)` is `n != null ?
  n + 1 : null`, where it was `(..) ?? null`.

## What holds it

- `test/lowering-nullable.test.ts`, "a nullable field is written what's
  never undefined as it is": React's `createContext` and `lazy`, a
  handle's write, an `Option` given still coalesced, run with and
  without `console.createTask`.
- Mutations in `scripts/mutations/js/make.ts` (`null-for-none-objects-coalesced`,
  `null-for-none-branches-coalesced`, `defined-may-be-undefined`),
  `scripts/mutations/lower/std_types/cell.ts` (`cell-variable-nulled-unmarked`), `scripts/mutations/lower/std_types/option.ts`
  (`map-some-undefined`), `scripts/mutations/lower/mir/patterns.ts`
  (`null-nones-calls-unwritten`, `null-nones-clones-written`) and
  `scripts/mutations/lower/mir/assign.ts` (`call-option-written-as-given`).

## Amendment: a nullable function's result, kept, is stored as it is

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above; and the
trait's method's, results change: its `undefined` is `null` where it's
stored, as the place's contract says.

React's `startTransition` keeps the Transition it replaces and gives it
back to `ReactSharedInternals.T` as its scope ends, through the `Drop`
guard that's its `finally`:

```rust
#[cfg_attr(rust_js, rust_js::link_name = "get T")]
#[cfg_attr(rust_js, rust_js::nullable)]
pub fn t(this: &SharedStateClient) -> Option<&'static Transition> { .. }

let prev_transition = t(REACT_SHARED_INTERNALS);
..
let _finish = FinishTransition { prev_transition, current_transition: &current_transition };
..
impl Drop for FinishTransition<'_> {
    fn drop(&mut self) {
        ..
        set_t(REACT_SHARED_INTERNALS, self.prev_transition);
    }
}
```

rust-js wrote `ReactSharedInternals.T = prevTransition ?? null;`, where
upstream writes `ReactSharedInternals.T = prevTransition;`. A nullable
function's result is `null` or a value (ADRs 0578, 0638), so `?? null` of
it changes nothing; but MIR's `nulled` knew it only of the call's own
destination, not of the variable it's kept in, nor of a guard's field
made of that, whose `Drop` is written apart and placed where the guard's
dropped (ADR 0589).

- **`nulled` says it of a variable written once, by a call of a
  `#[rust_js::nullable]` function**, a binding's too, and so of each copy
  of it, `let again = prev;`. One set again, `prev = other`, may be
  `undefined`, and isn't.
- **What's `nulled` is `Known::Defined` where it's made into a struct**,
  so a guard's field carries it; the guard taken apart where it's
  dropped, `fields_of`, reads the field as the variable it was made of,
  `prevTransition`, and keeps what was known of it, where nothing sets
  the field again; a one-field guard's too (`one_field_of`).
- **What a `Drop`'s body stored, `self.prev ?? null`, is asked again**
  where the guard's field is read as that variable: `Expr::store_again`
  is `made` of a `made ?? null` whose `made` is now never `undefined`
  (`may_be_undefined`), as `Storage::written` would have stored it. The
  one answer stays `may_be_undefined`'s.
- **A trait's method isn't taken to give `null`** (`calls::gives_null_for_none`,
  the one answer, which ADR 0638's call asks too): its impl may give
  `undefined`, which says nothing of the trait's `#[rust_js::nullable]`.
  `{ top: PPeek.peek(p) }` stored `undefined` in a nullable field; it's
  `PPeek.peek(p) ?? null`.

| Rust | JS |
|---|---|
| `let prev = t(i); ..; set_t(i, prev);` | `i.T = prev;` |
| `let prev = t(i); let again = prev; set_t(i, again);` | `i.T = again;` |
| `let mut prev = t(i); if r { prev = other; } set_t(i, prev);` | `i.T = prev ?? null;` |
| a guard `Restore { internals, prev }` whose `drop` is `set_t(self.internals, self.prev)` | `} finally { internals.T = prev; }` |
| the guard's `restore.prev = other;` before it's dropped | `} finally { internals.T = prev ?? null; }` |
| `set_t(i, queue.peek())` of a trait's nullable `peek` | `i.T = PPeek.peek(queue) ?? null;` |

### Why

- **It's React's JS**, and the same value: what's never `undefined` is
  given back as it is.
- **The fact rides on the expression**, as this decision's `Known` does:
  the guard's `Drop` is lowered once, apart, and only where it's placed
  is the field's value known.

### Alternatives

- **A crate fact of each struct field, every value written to it
  `nulled`**: each body's `nulled` asked of every other's, before any is
  lowered, and every write to the field, through a `&mut` too, found.
  Rejected for the expression's fact, which is there where it's needed.
- **Any `x ?? null` of what's never `undefined`, folded in preparation**:
  the identity holds, but a second place answering what a nullable place
  stores. Rejected: the guard's inliner asks the owner again.

### What holds it

- `test/lowering-nullable.test.ts`, "a nullable function's result is
  written to a nullable place as it is": a `let`, a copy, a `let` set
  again, a guard's field and a one-field guard's, a guard's field set
  again, and a trait's nullable method, run.
- Mutations `nullable-result-not-nulled`, `aggregate-nulled-unmarked`
  (`scripts/mutations/lower/mir/rvalues.ts`), `trait-nullable-result-trusted`
  (`lower/calls.ts`), `guard-field-known-lost`,
  `guard-field-set-again-known`, `one-field-guard-known-lost`
  (`lower/drops/inlined.ts`), `stored-again-unasked`,
  `stored-again-of-undefined` (`js/make.ts`).

## Amendment: an `and_then` of a nullable function

Case ([0262](0262-when-rust-and-js-disagree.md)): A, as above.

React's logUncaughtError writes `componentName = errorInfo.source ?
getComponentNameFromFiber(errorInfo.source) : null;`; its port,
`COMPONENT_NAME.set(error_info.source.as_deref().and_then(get_component_name_from_fiber))`,
printed `componentName = (errorInfo.source != null ?
getComponentNameFromFiber(errorInfo.source) : undefined) ?? null;`. `map`
marks its `Some` `Known::Defined`; `and_then`'s value is the function's
own `Option`, which may be `undefined`, so the whole was coalesced.

**An `and_then` of a function that `calls::gives_null_for_none` marks
what it gives `Known::Defined`** (`calls::given_by`), as a call of one
kept in a variable is (`nulled`, above): its `None` is `null`, so the
conditional's branches are each stored as they are, `c ? f(c) : null`. Only of a function item,
`and_then(name_of)`, as clippy's `redundant_closure` asks it written; a
closure's value isn't asked. One that may give `undefined`, a function
without `#[rust_js::nullable]` or a trait's method, is still `(..) ??
null`.

Held by `test/lowering-nullable.test.ts`, "an and_then of a nullable
function is stored as it is", run, `nullable_and_then.rs` against native
Rust, and the mutations `and-then-of-nullable-coalesced` and
`and-then-of-any-function-defined` (`lower/calls.ts`).

## Amendment: stored again where it's read in place (2026-10-26)

React's `ReactProfilerTimer.js` keeps the update's task in DEV:

```js
blockingUpdateTask = __DEV__ ? fiber._debugTask : null;
```

Its port, `BLOCKING_UPDATE_TASK.set(if __DEV__ { fiber.debug_task.get() }
else { None })`, read `(__DEV__ ? fiber._debugTask : undefined) ?? null`:
MIR makes the `if`'s value in a temporary, which the cell's write coalesced,
`v ?? null`, before tidy read it in place.

**A nullable place's `made ?? null`, once `made` is read in place, is
stored again** (tidy's `in place/stored again`, `Expr::store_again`): as
`Storage::written` would have stored it, the test with `null` for each
`undefined` branch where every other is never `undefined`, `__DEV__ ?
fiber._debugTask : null`, a nullable field's read being `null` or a value
(`Known::Defined`). The one rule is `null_for_none`'s, which `written`
and `store_again` both ask.

Case A, as before. Held by `test/lowering-module-variables.test.ts`, "a
nullable thread-local of an Option is null where it's made, and what it
gives is as it is", the task in and out of DEV, and the mutation
`mir-tidy-stored-again-off`.

## Amendment: an `and_then` of a nullable function is `null` for `None`, wherever it's kept

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The program reads
`null` and `undefined` alike as `None` (ADR 0030; a map's `Option` key
too, ADR 0121, amended), so which one a `None` is shows only to JS, and
here only upstream's text tells them apart.

React's `const ownerName = owner ? getComponentNameFromOwner(owner) :
null; if (ownerName) ..`, ported `owner.filter(|owner|
js::truthy(Some(*owner))).and_then(get_component_name_from_owner)`, was
`owner ? getComponentNameFromOwner(owner) : undefined`: the `None` of
`and_then` was `null` only where a nullable place stored it (above),
and `undefined` in a variable.

**An `and_then` of a function that `calls::gives_null_for_none` gives
`null` where what it's of is `None`** (`calls::and_then_given`), so the
whole `Option` is the function's, `null` for `None` both ways, as
upstream writes it: `owner ? nameOfOwner(owner) : null`, `fiber != null
? nameOf(fiber) : null`. A filter's and an `and_then`'s of one are both
so; an optional chain, `o?.name()`, whose `None` is `undefined`, isn't
written where `null` is asked for. A function that may give `undefined`
is as before, `: undefined`.

Held by `test/lowering-nullable.test.ts`, "an and_then of a nullable
function is stored as it is", now with a variable tested for truth and
one tested `is_some()`, run; mutations `and-then-of-nullable-none-undefined`
(`lower/calls.ts`) and `filtered-and-then-none-undefined`
(`lower/combinators.ts`).

## Amendment: an `and_then` of a closure of a nullable function's call

React's claimNextHydratableActivityInstance writes `const
activityInstance = nextInstance ? tryHydrateActivity(fiber,
nextInstance) : null;`; its port, `next_instance.and_then(|next_instance|
try_hydrate_activity(fiber, next_instance))`, printed `: undefined`:
`and_then_given` gave `null` only of a function item that
`gives_null_for_none`, and a closure is none.

**An `and_then` of a closure whose value, inlined, is known never
`undefined` (`Known::Defined`), a nullable function's call, gives `null`
where what it's of is `None`**, so the whole `Option` is the call's, as
of the function itself. A closure of anything else is as before, `:
undefined`. Case N ([0262](0262-when-rust-and-js-disagree.md)), as above.

Held by `test/lowering-nullable-reads.test.ts`'s "an and_then of a
closure of a nullable function's call is null for None", run, and the
mutation `and-then-of-closure-of-nullable-none-undefined` (`calls.ts`).


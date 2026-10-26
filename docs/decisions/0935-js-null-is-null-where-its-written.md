# 0935. `js::NULL` is `None`, and `null` where it's written

Status: Accepted, the user's choice of 2026-10-26; amended: `js::null()`, of any
`Option`, replaces `js::NULL`. Extends
[0586](0586-a-nullable-field-is-an-option.md) (a nullable `const`) and
[0225](0225-unknown-values.md) (`js::Unknown`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N where Rust reads it:
`null` and `undefined` are each `None` to the program, which tests `!=
null` and compares `(a ?? null) === (b ?? null)` (ADRs 0030, 0612); C
where JS reads it, which tells `null` from `undefined` and sees a key
that's there: the program says which it means.

## Context

React's Fiber has fields typed `any` upstream, `memoizedState`,
`memoizedProps`, `updateQueue`, `stateNode` among them, which its
constructor makes `null` and which then hold exactly what JS writes,
`undefined` too:

```js
this.memoizedState = null;
..
workInProgress.memoizedState = nextState; // whatever a hook gave
```

The port's `Cell<Option<&'static Unknown>>` had two forms, neither
right:

- **A plain `Option` field made `None`** is left out of the object
  literal (ADR 0030). React's dev mode can't add the key later, its
  objects' shapes kept, and 1220 of its tests failed.
- **A `#[rust_js::nullable]` field** writes `?? null` on every store
  (ADR 0772), so a stored `undefined` reads back `null`: a wrong answer,
  where React tells them apart.

## Decision

The user's decision: **the js crate has `js::NULL: Option<&'static
Unknown>`, `None` in Rust, written `null`**, beside `js::Unknown` and
`js::cast`:

```rust
pub struct Fiber {
    pub memoized_state: Cell<Option<&'static Unknown>>,
}

Fiber { memoized_state: Cell::new(js::NULL) }
fiber.memoized_state.set(next_state);
```

```js
({ memoizedState: null });
fiber.memoizedState = nextState;
```

- **It's the js crate's `const`, `#[rust_js::nullable]`**, the meaning a
  nullable `const` has already (ADR 0586, amended): its `None` is
  `null`. A binding crate is never compiled to JS, so a use of one of its
  constants is its value, written in place (ADR 0031); a nullable one's
  is stored as a nullable place stores it, `null` (`named_const`).
- **A constant written of it is written as it's written**, `const EMPTY:
  Fiber = Fiber { memoized_state: Cell::new(js::NULL), .. }`'s `{
  memoizedState: null }`, not rustc's value of it, whose `None` would be
  left out: it's of a constant that isn't rustc's value, as one of a
  library's flags is (ADR 0031, amended).
- **The field stays a plain `Option`**: what's stored there later is
  stored as it is, `undefined` too. One meaning each: `js::NULL` says
  "this `None` is `null`", where it's written; `#[rust_js::nullable]`
  says "this place's `None` is always `null`".
- **Reading it is reading any `Option<&Unknown>`**: `is_some()` is `!=
  null`, `==` is `(a ?? null) === (b ?? null)`, `None` of either, as Rust
  reads both. `js::strict_equals(x, js::NULL)` is `x === null`, JS's.

## Why

- **It's what upstream writes**, `null` where it's made, and keeps what
  JS stores, which no attribute of the field can say: a field's attribute
  is of every write to it.
- **No dialect**: in Rust it's `None`, an `Option` std's code reads as
  one; only the JS it's written as says `null`.

## Alternatives

- **`#[rust_js::nullable]` on the field, stores passed through**: a
  nullable field would mean `null` at its making and as-is after, a
  second meaning of the attribute, where its `None` is `null` everywhere
  else.
- **A field attribute, "made null"**: an attribute per field for what
  one value says where it's written, and none for a `None` given
  elsewhere, an argument or a local.
- **`Some(js::unknown_of(js::Kind::Null))`**: a `Some` to Rust, whose
  `is_some()` is true natively and false in JS: a wrong answer.

## What holds it

- `test/lowering-nullable-constants.test.ts`, "js::NULL is null where
  it's written, and what's stored after is as it is": a struct made, a
  constant, a thread-local, a store and a reset, run as JS, an
  `undefined` stored kept so and read as `None`.
- Mutations `js-null-written-undefined` (`lower.rs`) and
  `js-null-constant-by-value` (`lower/pipeline.rs`).

## Amendment: `js::null()`, of any `Option`, replaces `js::NULL`

Status: Accepted, the user's choice of 2026-10-26.

Case ([0262](0262-when-rust-and-js-disagree.md)): as above, N where Rust
reads it, C where JS reads it; and C for a map's keys, a wrong answer
fixed (ADR 0121, amended).

**Context.** `js::NULL` was only an `Option<&'static Unknown>`. React's
fiber has typed fields that start `null` and keep what's stored after:
`_debugStack: Error | null`, `_debugHookTypes: Array<HookType> | null`,
a tracing marker's `name: string | null`. The port had to make them
`#[rust_js::nullable]`, whose every store writes `?? null`:
`workInProgress._debugStack = current._debugStack ?? null`, and
`name: pendingProps.name ?? null` even made an absent name `null` where
upstream keeps `undefined`.

**Decision.** The user's: **`js::null::<T>() -> Option<T>`, usually
inferred, is `None` to Rust and `null` where it's written**, of an
`Option` of any type. It replaces `js::NULL`: one way.

```rust
pub struct Fiber {
    pub debug_stack: Cell<Option<&'static JsError>>,
    pub name: Option<String>,
}

Fiber { debug_stack: Cell::new(js::null()), name: js::null() }
work_in_progress.debug_stack.set(current.debug_stack.get());
```

```js
({ debugStack: null, name: null });
workInProgress.debugStack = current.debugStack;
```

- **It's a `const fn` of the js crate, `link_name = "null"`**, whose
  call is written `null` (`JsForm::Null`), and `#[rust_js::nullable]`,
  whose result's `None` is `null`, so a nullable place stores it as it
  is. Natively it's `None`.
- **A constant written of it is its initializer's**, as one written of
  `js::NULL` was: its call is one written in place, which rustc's value,
  whose `None` is left out, isn't (`written_of_constants`'s `in_place`).
  `js::NULL`'s handling, a binding crate's nullable `const` written
  `null` (`named_const`, `by_name`'s `nullable_local`), is gone.
- **`js::NULL` fails to resolve**, and rustc's error names the way:
  "a function with a similar name exists: `js::null`".

**Reads.** A `None` that's `null` is `None` wherever rust-js reads one.
Every read of an `Option` was checked, by programs run natively and as
JS whose `None`s were a nullable field's `null`: `is_some()`, `match`,
`==` (`a == b`, `(a ?? null) === (b ?? null)` and `$eq`), `cmp`, `sort`,
`dedup`, `contains`, `position`, `flatten`, `collect` into an `Option`,
`sum`, `take`, `replace`, `get_or_insert`, `xor`, `zip`, `transpose`, a
`Some` of one, `$some(null)`, a tuple's, an array's and a struct's,
`Debug`, a map's values and serde's `skip_serializing_if`. One misread
it: **a map or a set keyed by an `Option` was a plain `Map`**, its
`null` key and its `undefined` key two, fixed first (ADR 0121, amended).
The `=== undefined` tests rust-js writes are of what's never a program's
`Option`: a `OnceCell`'s slot, which holds `$some` of an `Option`; an
iterator's end, whose items are boxed where they're `Option`s; a props
field's default, which is of a literal's type; and a `fmt::Result`.

**What isn't changed.** A plain `Option` field's `.d.ts` is still `?: T
| undefined`, though the program may now write `null` there. Making
every one `T | null | undefined` changed every port's props
(`onClick?: MouseEventHandler | null | undefined` where @types/react
writes `| undefined`); a field's own `| null` where `js::null()` is
written to it needs a crate-wide fact of where it's written. Left for
the user's choice.

**What holds it.** `test/lowering-nullable-constants.test.ts`, "js::null()
is null where it's written, of any Option, and what's stored after is as
it is": an `&Unknown`, a `&JsError`, an `Rc<RefCell<Vec<String>>>` and a
`String` field made, a constant, a thread-local, a copy and a reset, no
`?? null`, run as JS, an `undefined` stored kept so and read as `None`.
Mutations `js-null-written-undefined` (`binding_calls.rs`) and
`js-null-constant-by-value` (`pipeline.rs`).

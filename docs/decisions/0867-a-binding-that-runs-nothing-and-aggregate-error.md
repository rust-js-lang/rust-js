# 0867. `AggregateError` is ES2021's, and a binding that runs nothing borrows nothing

Status: Accepted. Extends [0102](0102-js-and-webapi.md) (the js crate)
and [0283](0283-js-crate-covers-typescripts-es-library.md) (its
coverage); amends [0863](0863-a-refcell-of-an-object-is-the-object.md)
and [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md) (what
a call may borrow). Extended by
[0898](0898-a-binding-that-runs-only-what-its-given.md): a binding that runs only
what it's given, `#[rust_js::runs_only_given]`, given only numbers and
text, borrows nothing. Extended by [0277](0277-shadow-aliases.md),
amended: a call of either sets none of the program's variables.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. `AggregateError`
is JS's own. A borrow left unchecked is one no other can meet: a binding
that runs nothing, and a JS object's `Deref`, which is the object, run
none of the crate's code while it's held, so its count is 0 wherever
it's checked. A binding that may run the crate's code, given a closure
or not, is counted as before, and panics `already borrowed` where Rust
does.

## Context

React's `act` throws what its scope caught, several as one:

```js
function aggregateErrors(errors: Array<mixed>): mixed {
  if (errors.length > 1 && typeof AggregateError === 'function') {
    return new AggregateError(errors);
  }
  return errors[0];
}
..
const thrownError = aggregateErrors(ReactSharedInternals.thrownErrors);
ReactSharedInternals.thrownErrors.length = 0;
```

The js crate had no `AggregateError`, so the port declared its own,
`#[link_name = "new AggregateError"] fn new_aggregate_error(..)`. And its
`aggregate_errors(r.thrown_errors.borrow().as_ref().unwrap())` holds the
`RefCell`'s borrow while it calls the binding. A binding is JS no one can
see, which may call the crate back (ADR 0863), so that borrow was counted,
and `thrownErrors`, a field the renderers' JS reads as an array, was a
`{ value }` cell everywhere: `thrownErrors: {}`, `$cellReplace($borrowMut(
..), [])`, a `$borrow`/`$unborrow` `try` at every use. Four of React's
`ReactTestUtilsAct` tests failed reading it.

`new AggregateError(errors)` runs none of the crate's code: it iterates
the array it's given and keeps what's in it, calling nothing of it. Nor
does `typeof AggregateError`, `js::is_defined`'s, nor the `Deref` of an
`&AggregateError` to the `&JsError` whose `as_unknown()` the port returns,
which is the object itself.

## Decision

**The js crate has `AggregateError`, as TypeScript's `lib.es2021`
types it**: `js::aggregate_error::new(errors, message)`, `new
AggregateError(errors: Iterable<any>, message?: string)`, `errors` a
slice of any value, `&[Option<&Unknown>]`, as `Reflect.apply`'s
`ArrayLike<any>` is, `message` an `Option<&str>`; and an
`&AggregateError`'s `errors()`, `any[]`, and, by `Deref`, a `&JsError`'s
`name()`, `message()`, `cause()`, `stack()`, `is_error()` and
`as_unknown()`. It's in the crate's coverage, all of its members bound.

```rust
fn aggregate_errors(errors: &[Option<&'static Unknown>]) -> Option<&'static Unknown> {
    if errors.len() > 1 && js::is_defined(aggregate_error::new) {
        return aggregate_error::new(errors, None).as_unknown();
    }
    errors.first().copied().flatten()
}
```

```js
function aggregateErrors(errors) {
  if (errors.length > 1 && typeof AggregateError !== "undefined") {
    return new AggregateError(errors);
  }
  return errors[0];
}
```

**A binding runs none of the crate's code where its form can't**
(`cell_borrows::binding_runs_nothing`), now whether the host has a
global, `typeof X !== "undefined"`, too; **or where it says so,
`#[rust_js::runs_nothing]`**: it runs no code but its own, not even what
it's given, as reading a data property can't throw is
`#[rust_js::cannot_throw]`'s (ADR 0301). The js crate marks
`aggregate_error::new`. **A JS object type's `Deref` to the class it
extends is the object** (`is_js_object_deref`), so it runs nothing
either. A borrow held across them asks nothing, and a cell no borrow of
which can meet another is the object, unchecked, as ADR 0863 has it:

```js
$unwrap(ReactSharedInternals.thrownErrors).push(error);
..
const thrownError = aggregateErrors($unwrap(ReactSharedInternals.thrownErrors));
$unwrap(ReactSharedInternals.thrownErrors).length = 0;
```

## Why

- **It's JS's**, and TypeScript's; a port writes it, not a stand-in.
- **The check it drops can't fail**: what runs while the borrow is held
  is the binding's own code, which never reaches the crate's.
- **A mark, not a rule of types**: a binding given nothing may still call
  the crate back, `el.click()` running a listener it was given before,
  `dispatchEvent`, a custom element's `connectedCallback`; and one given
  only values may read what's in them, `String(x)` its `toString`. Only
  the binding's author knows it reads none, as only they know a property
  is data (ADR 0301).

## Alternatives

- **A binding given no closure runs nothing**: wrong of `click()`,
  whose listener may borrow the cell mutably while the borrow is held:
  a check left out that Rust's program fails.
- **`Iterable<any>` as `impl IntoIterator`**: a JS iterator of a Rust
  one, a second form beside the array a slice is, which every caller
  here has.
- **The port clones the errors before the call**: Rust bent to fit the
  compiler, and a copy React doesn't make.

## Consequences

- `#[rust_js::runs_nothing]` is in the guide's table of a binding's
  attributes; the js crate's `AggregateError` in its error page.
- A crate's own binding may say it; saying it of one that calls back
  leaves a check out, as a wrong `cannot_throw` would.

## What holds it

- `test/js-builtins.test.ts`, "an AggregateError is ES2021's, and making
  one borrows nothing": React's `aggregateErrors` of a thread-local's
  `RefCell` field, compiled and run, the cell the array, and
  `AggregateError`'s `errors`, `name` and `message`.
- `test/lowering-cells.test.ts`, "a borrow held while a binding runs is
  counted, unless it runs nothing": a binding that calls the crate back
  panics `already borrowed`, as Rust's would; one marked doesn't count.
- `builtins/coverage.txt`: `AggregateError`'s six members.
- Mutations in `scripts/mutations/lower/analysis/cell_borrows.ts`:
  `binding-runs-nothing-ignored`, `is-defined-runs-code`,
  `js-object-deref-runs-code`.

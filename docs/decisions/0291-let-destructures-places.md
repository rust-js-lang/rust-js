# 0291. A `let` taking a variable apart is JS's destructuring

Status: Accepted. Extends [0038](0038-js-names-and-destructuring.md)'s and
[0244](0244-destructuring-through-references.md)'s destructuring `let`s
from values to places.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`let (count, set_count) = use_state(0);` is `const [count, setCount] =
useState(0);`, but a `let` of a variable, or of a field, named each part
where it is: `let State { status, register, .. } = state;` gave no
statement, and `register(*status)` was `state.register(state.status)`.

That loses the names the Rust gave, and it's not the same call: JS gives a
function called as a method its object as `this`, which a Rust call of a
function in a field never does. react.dev's Preview destructures Sandpack's
state and calls what it took:

```ts
let {error: rawError, registerBundler, unregisterBundler} = sandpack;
registerBundler(iframeElement, clientId);
```

## Decision

**A `let` that binds parts of a place is JS's destructuring of it, as one
of a value is: `let { error: rawError, registerBundler } = sandpack;`.**

- Each part is read once, where the `let` is, as Rust moves, copies or
  borrows it there. What JS's destructuring can't give is taken apart as
  before: a `Copy` part changed in place, which needs a copy of its own, a
  `ref mut` binding, a `Cell` in an object's field.
- A place owning a destructor is taken apart as before: what's moved out
  of it is cleared, so its drop skips it (ADR 0098).
- An `async fn`'s parameter pattern stays the parameter's, `async function
  swap([a, b])` (ADR 0029).
- A `let` binding nothing, `let (_, _) = pair;`, is no statement.
- A component's props with a `Rest` can be taken apart by a `let` too,
  `const { href, ...rest } = p;` (ADR 0195); a `match` still can't.

## Why

- **It's the JavaScript a person writes**, with the Rust's names:
  `const [run, page] = ..` and `}, [run]);`, not `tmp[0]`.
- **It's the same program**: a function taken out of an object is called
  without it, as Rust calls it.

## Consequences

- A `let` of a conditional, `let (point, width) = if .. { .. } else { .. };`,
  destructures it too: `const [point, width] = b < 128 ? .. : ..;`.

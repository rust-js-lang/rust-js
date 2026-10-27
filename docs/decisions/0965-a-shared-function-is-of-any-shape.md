# 0965. A shared function, an `Rc<dyn Fn>`, is of any shape, the function itself

Status: Accepted. Extends [0673](0673-a-function-is-a-value-of-any-shape.md),
whose function of any signature is of any shape, to an `Rc` of one, as
[0850](0850-an-rc-is-defined-as-what-it-points-at.md) made an `Rc` of a
`Defined` value `Defined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N of an uncounted
`Rc`, the function itself, which JS shares as Rust does; D, refused, of a
counted one, whose count JS couldn't keep (ADR 0850).

## Context

React's `mountTransition` keeps the function it returns in its hook, and
`updateTransition` reads it back:

```js
const start = startTransition.bind(null, currentlyRenderingFiber, stateHook.queue, true, false);
const hook = mountWorkInProgressHook();
hook.memoizedState = start;
return [false, start];
```

`mountRefresh` does the same with `refresh`. A port's `start` is an
`Rc<dyn Fn(..)>`, returned to the component and kept by the hook, and
`hook.memoizedState` is any value, `Option<&'static Unknown>`. The port
wrote

```rust
hook.memoized_state.set(Some(js::unknown_of(Rc::clone(&start))));
```

and rustc refused it: "the trait `js::AnyShape<_>` is not implemented for
`Rc<dyn Fn(f64)>`". A `dyn Fn` is of any shape (ADR 0673), and an `Rc` of
a `Defined` value is (ADR 0850), but an `Rc` of a function was neither:
`Rc<F>` isn't `Fn`, and a `dyn Fn` isn't `Defined`. The port stored the
closure boxed and read it back as an `Rc`, a draft-only workaround.

## Decision

**An `Rc` of a function of any signature is of any shape, the function
it points at: `js::unknown_of(Rc::clone(&start))` is `start`, and
`js::cast` gives it back as the `Rc`.**

```rust
pub fn mount_transition(hook: &Hook, pending: Rc<Cell<bool>>) -> Rc<dyn Fn(bool)> {
    let start: Rc<dyn Fn(bool)> = Rc::new(move |value| pending.set(value));
    hook.memoized_state.set(Some(js::unknown_of(Rc::clone(&start))));
    start
}
```

```js
export function mountTransition(hook, pending) {
  function start(value) {
    pending.value = value;
  }
  hook.memoizedState = start;
  return start;
}
```

- **`AnyShape`'s impls for a function**, one per arity up to 12, each
  have one for `Rc<F>` beside them, its marker `Rc<fn(..) -> R>`: a
  marker of its own, as `Rc<F>`'s impl and `F`'s may not share one
  (rustc can't rule out std's `Rc` being `Fn` one day). No program names it.
- **Its JS is the value itself**, as an uncounted `Rc` is (ADRs 0023,
  0844): `hook.memoizedState === start`, and what's read back calls the
  same closure.
- **A counted one is refused**, as every counted `Rc` given to JS is
  (ADR 0850's `check_given_to_js`): JS would hold its `{ value, strong,
  weak }`, which isn't callable, and what `js::cast` gives back is an
  owner Rust never counted.

## Why

- **The Rust a person writes**: `Rc::clone(&start)` given where a value of
  any shape goes, no box and no cast.
- **Upstream's JS**: one function, stored and returned, identical in
  both places.

## Alternatives

- **`impl AnyShape<fn(..) -> R> for Rc<F>`**, the function's own marker:
  rejected by coherence, overlapping `F`'s impl if `Rc` were ever `Fn`.
- **`Rc<T>` of any shape for every `T: AnyShape`**: a blanket impl over a
  trait with a marker parameter, `impl<T: AnyShape<M>, M> AnyShape<M> for
  Rc<T>`, overlaps the `Defined` one (`Rc<T>` is `Defined` where `T` is).
  Only functions lacked it.
- **`Arc` too**: no port has needed one; it would be one more impl line
  per arity when one does.

## Consequences

- React's `mountTransition` and `mountRefresh` store and return the one
  function, as upstream does.
- Tested by `test/js-object.test.ts`'s "a shared function is of any
  shape": stored, read back, called, and `===` in JS. The change is the
  `js` crate's impls, which no compiler mutation reaches; without them the
  test's program is E0277.

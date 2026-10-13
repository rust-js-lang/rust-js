# 0367. A state's setter is the function it is

Status: Accepted. Extends [0039](0039-generic-bindings.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`use_state` gives a setter, a `Dispatch<SetStateAction<T>>`, a `Copy`
handle called by `.set(value)` (ADR 0039). To @types/react it's a
`Dispatch`, a function, and a page gives it where a prop wants one:
react.dev's home page writes `onChange={setSearchText}` and returns
`setHoverLine` for `onLineHover`. A handle is no Rust `Fn`, so its port
wrapped it in a closure, leaked to be `'static`:
`Box::leak(Box::new(move |line| set_hover_line.set(line)))`.

## Decision

**The setter derefs to `dyn Fn(T)`, and `use_state` gives it `'static`,
as it gives the state: `&**set_text` is a `&'static dyn Fn(T)`, and
rust-js writes it as the setter itself, `setText`.**

- `impl<T> Deref for Dispatch<SetStateAction<T>>`, whose `Target` is
  `dyn Fn(T)`; rust-js compiles a JS object's `Deref` to the object
  (ADR 0024), so the body never runs.
- `use_state` gives `&'static Dispatch<SetStateAction<T>>`: React keeps
  the setter, the same one each render. A memo'd component's props are
  `'static`, and react.dev gives `setHoverLine` to a memo'd CodeBlock.
- `&**`, not a coercion: Rust tries a `&Dispatch` as a `&dyn Fn(T)` by
  unsizing first, which needs the handle to be an `Fn`, and stops there.
- `.set(value)` stays the call, as ADR 0039 has it.

## Why

- **The JS a person writes**: `onChange={setSearchText}`, no wrapper.
- **@types/react's type**: `Dispatch<A>` is `(value: A) => void`.

## Alternatives

- **`Dispatch<A>` a `&'static dyn Fn(A)`, as @types/react's alias**:
  `set_text(v)` the call, `.set` gone, and the updater another spelling;
  every port's calls change, for what a `Deref` gives.
- **An `as_fn()` method**: one call spelling, but the source reads less
  like the original.

## Consequences

- `set_text(v)` compiles too, through the `Deref`; `.set(v)` is the
  spelling the docs and ports use.
- A function naming the setter's type writes `&'static Dispatch<..>`.
- A reducer's `Dispatch<A>` has no `Deref` yet: an impl for every `A`
  would overlap the setter's.
- `test/jsx.test.ts` holds `&**set_text`, alone and in a `'static` prop's
  `Some`, as `setText`.

## Since

2026-10-13.

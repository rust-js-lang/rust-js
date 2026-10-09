# 0297. A closure a `let` names for a hook is the hook's named function

Status: Accepted. Adds `#[rust_js::named_callback]` to the react crate's
effects, `useMemo`, `useCallback` and `useEffectEvent`.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev names its effects, as React's documentation advises, so a stack
trace and a reader say what each does:

```ts
useEffect(function createBundler() { .. }, []);
```

A Rust closure has no name. The name a port can give it is a `let`'s:
`let createBundler = move || { .. }; use_effect(createBundler, ());`,
which was `const createBundler = () => { .. }; useEffect(createBundler, []);`.

## Decision

**A closure a `let` names, whose one use is as the function a hook marked
`#[rust_js::named_callback]` is given, is written there as a function named
by the `let`:**

```rust
let createBundler = move || { registerBundler(iframe, clientId); };
use_effect(createBundler, ());
// useEffect(function createBundler() { registerBundler(iframe, clientId); }, []);
```

- The react crate marks `use_effect`, `use_layout_effect`,
  `use_insertion_effect` (and their `_on_every_render` forms),
  `use_effect_event`, `use_memo` and `use_callback`.
- A closure used besides, or given to anything else, is a `const` as
  before: an event handler stays `const handleClick = () => ..`.
- One whose body reads the name it would have, a variable it shadows, is a
  `const` too: named so, it would read itself.

## Why

- **It's the JavaScript a person writes**: the original's named effects.
- **It's the same program**: making a closure has no effect, and a JS
  function reads its variables when it runs wherever it's written; nothing
  in a Rust closure reads `this` or `arguments`, where an arrow and a
  function differ.

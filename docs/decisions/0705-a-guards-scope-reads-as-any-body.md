# 0705. A guard's scope reads as any body, its fields the variables it was made of

Status: Accepted. Amends [0693](0693-a-guards-scope-is-one-finally-its-returns-in-it.md),
whose `try` kept the temporaries each statement's own `try` had held
apart, and whose guard stayed an object where a field's name was taken;
and [0589](0589-a-drop-dropped-once-is-its-body-there.md), whose guard
was its fields only where no name of the function was a field's.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same drops in the
same order, and what's read the same value; only how the JS reads changes.

## Context

React's `describeNativeComponentFrame` restores what it changed in one
`finally`, and its fallback restores `fn.prototype.props` in another:

```js
} finally {
  if (prototypeModified) {
    if (prevProps !== undefined) {
      Object.defineProperty(fn.prototype, 'props', prevProps);
    } else {
      delete fn.prototype.props;
    }
  }
}
```

Its port holds each in a guard, `Restore { previous_prepare_stack_trace,
previous_dispatcher }` and `RestoreProps { prototype, prev_props }`, whose
`Drop` restores it. ADR 0693 made each scope one `try { .. } finally
{ .. }`, and left three things the port's JS read worse for:

- **The guard an object**, `const _restoreProps = { prototype, prevProps
  };`, read `_restoreProps.prevProps` in the `finally`. ADR 0589 takes a
  guard apart into a `const` of each field, by the field's name; a field
  made of a variable of that name, `{ prototype }`, took a name already
  taken, so the guard stayed whole.
- **What each statement's `try` held apart**, `const tmp =
  controlLines[c].includes(..); if (tmp)`: lowering wraps each statement
  that may throw in its own `try` and `catch` that drops the guard, `let
  tmp; try { tmp = .. } catch ..`; tidy reads temporaries in place across
  none of them, and the scope became one `finally` after tidy ran.
- **`const result = frame; return result;`**, the function's return place
  made in the `try`, for the same reason.

## Decision

**Once a body's runs of statements a drop ends are each one `finally`,
the stages of tidy that read temporaries in place run again**, `names`,
`in place` and `chains` (`stages::UNWOUND`): what a `try` held apart reads
as tidy reads any body. The scopes are made before the temporaries are
named, so what tidy reads in place is still a temporary.

**A guard's field made of a variable reads as that variable**, where
nothing sets it once the guard's made (later in the guard's block, in a
closure, or through a handle) and the function declares it once; a
`Drop`'s copy of the field, `const prevProps = guard.prevProps;` of a
`match`'s binding, declares no other and goes. Any other field is a
`const` of its own, by its name, as ADR 0589 made it.

```js
const prevProps = Object.getOwnPropertyDescriptor(prototype, 'props');
Object.defineProperty(prototype, 'props', { .. });
try {
  Reflect.construct(fn, []);
} finally {
  if (prevProps) {
    Object.defineProperty(prototype, 'props', prevProps);
  } else {
    Reflect.deleteProperty(prototype, 'props');
  }
}
```

```js
try {
  const [sampleStack, controlStack] = DetermineComponentFrameRoot(fn, construct);
  ..
  while (c < controlLines.length) {
    if (controlLines[c].includes('DetermineComponentFrameRoot')) {
      break;
    }
    c++;
  }
  ..
} finally {
  reentry = false;
  Error.prepareStackTrace = previousPrepareStackTrace;
}
```

## Why

- **It's React's `finally`**, which reads `prevProps` and
  `previousDispatcher` themselves, and its loops' tests in place.
- **One way a body reads**: a scope of drops reads as the same statements
  without one, the tidy that made the one making the other.
- **The same values**: a variable nothing sets once the guard's made is
  what the guard holds wherever the guard is read.

## Alternatives

- **Making the scopes before tidy**: MIR's blocks and labels are still
  there, which a scope's run doesn't cover, and the drops not yet alike.
- **Tidying the whole body again**: its `loops` stage expects MIR's
  `while (true)` shapes, and broke its own contract on six corpus cases.

## Consequences

- Corpus programs read shorter: `const tmp = ..; if (tmp)` read in
  place, a tuple `let` a destructuring, `const [a, b] = f();`, a `&&` one
  test, `if (fn.displayName && frame.includes(..))`, and a
  `$catchUnwind(() => ..)`'s closure where it's given. A temporary a `let`
  set, `tmp`, is a `const` now, named `value`, as in a body without drops.
- **An arrow is never moved into a call of it that `calls::apply` can't
  make a value of** (`values::applied_in_place`, the one answer `applied
  arrows` asks too): a closure of statements stays held by its name,
  `const value = (l) => { .. }; x ? value(x) : undefined`, not `x ? ((l)
  => { .. })(x) : undefined`, here and where nothing's guarded, which
  four corpus programs wrote before.
- **A body's temporaries are named in the order they're first read**, not
  outer statement lists first: one made in a `try` takes `next` before a
  `let` declared at the top and set later.
- A guard whose field's variable is set later, `previous += 1;`, stays an
  object, `_restore.previous`.

## What holds it

- `test/lowering-unwinds.test.ts`, "a guard's fields are its variables,
  its scope read in place", run in Node: both guards taken apart, the
  loop's test read in place, a variable set later and one shadowed by the
  `Drop` kept apart.
- Every corpus case against native Rust.
- `test/lowering.test.ts`, "a tuple let in a guard's try is a
  destructuring" and "a && in a guard's try is one test".
- Mutations `closure-of-statements-called-in-place` (`mir/tidy/moves.rs`),
  `temporaries-named-by-nesting` (`mir/tidy/bindings.rs`),
  `scoped-drops-unread-in-place` (`mir/tidy/unwinds.rs`),
  `unwound-stages-none` (`mir/tidy/stages.rs`), `guard-field-variable-kept`,
  `guard-field-variable-set-later`, `guard-field-copy-declares`,
  `guard-field-self-copy-kept`, `guard-field-shadowed` (`drops/inlined.rs`).

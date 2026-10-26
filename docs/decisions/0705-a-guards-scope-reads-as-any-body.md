# 0705. A guard's scope reads as any body, its fields the variables it was made of

Status: Accepted. Amends [0693](0693-a-guards-scope-is-one-finally-its-returns-in-it.md),
whose `try` kept the temporaries each statement's own `try` had held
apart, and whose guard stayed an object where a field's name was taken;
and [0589](0589-a-drop-dropped-once-is-its-body-there.md), whose guard
was its fields only where no name of the function was a field's. Amended
below: a `while`'s test the scope held apart joins the loop's, and a loop
that ends a block MIR made breaks as a loop; a one-field tuple guard's
field of a variable is that variable too. Extended by
[0859](0859-a-loop-that-ends-a-branch-breaks-as-a-loop.md): each of the
loop's `break`s of the block, as a `loop` that ends an `if`'s branch has.

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
  while (c < controlLines.length && !controlLines[c].includes('DetermineComponentFrameRoot')) {
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

## Amendment: a `while`'s test is whole, and a loop that ends a block breaks as a loop

Case ([0262](0262-when-rust-and-js-disagree.md)): N, how the JS reads;
nothing that runs changes.

React's ported `describeNativeComponentFrame` still read, in its guard's
`finally`:

```js
bb166: {
  if (sampleStack != null && controlStack != null) {
    ..
    while (s < sampleLines.length) {
      if (!sampleLines[s].includes('DetermineComponentFrameRoot')) {
        s++;
      } else {
        break;
      }
    }
    ..
    while (true) {
      if (s >= 1 && c >= 0) {
        ..
      }
      break bb166;
    }
  }
}
```

where upstream writes `while (s < sampleLines.length &&
!sampleLines[s].includes('DetermineComponentFrameRoot')) { s++; }` and
`while (s >= 1 && c >= 0) { .. }`, with no label. Neither was new between
ADR 0705's commit and ADR 0710's, as the port reads now: each was there
before ADR 0709 took `$index` out of the first.

- **The `while`'s test.** Its first tidy left `contains` apart, `let
  contains; try { contains = .. } catch ..`, so `while_tests` took only
  `s < sampleLines.length` for the loop's test. Read in place once the
  scope is one `finally`, the rest is `if (t) { s++; } else { break; }`,
  which `while_tests` took only of a `while (true)`. **`while (a) { if (t)
  { rest } else { break; } }` is now `while (a && t) { rest }`**, as
  `while (true)`'s is `while (t) { rest }`, one rule for the `if` first in
  a `while`'s body whose `else` is its `break` alone, wherever its test
  came from (ADR 0702).
- **The label.** MIR puts each loop in the way out of the one before;
  `exits after` puts each after it, so the last ends the `if`, and the
  block MIR made round it, whose end its `break bb166` went to. **A loop
  that ends a labeled block MIR made, through the `if`s and blocks that
  end with it, and whose body ends `break bb166;`, ends with `break;`**
  (`blocks_loops_end`, in `chains`), which goes where the block ends too,
  and which `while_tests` reads the loop's test by; `unlabeled` then
  leaves the block where nothing else names it. Only the body's last
  jump: a `break bb45` in an `if` of the body is what later passes read
  a loop's way out by, `while (t) { i++; } rest` of a `rest` that leaves
  the loop. This isn't a guard's alone: `loop_value_after_loop.rs`,
  `calculator.rs` and `returns_in_place.rs` lose their `bb22:`, `bb69:`
  and `bb61:` too, and the last two a `while (true)`, `while
  (Parser.peek(parser) === "Comma") { .. }`; `tail continues` runs after
  `early returns` now, not before `chained checks`: `early returns` makes
  `if (r.TAG === "Err") { return r; } args.push(r._0); continue;` of a
  loop this leaves `while (true)` no more, whose `continue` goes round
  anyway, and nothing between the two places reads one.

The corpus reads shorter: three labeled blocks and two `while (true)`
fewer, fourteen lines; `renamed x$1` is as many as it was, its share of
fewer lines a tenth higher (`docs/readability.txt`).

Both loops now read as upstream's, and the third is `while (s >= 1 && c
>= 0) { if (..) { ..; break; } s--; c--; }` (ADR 0702).

### Alternatives

- **Running `loops` and `settled` again after the scopes are made**:
  ADR 0705 rejected the whole of tidy for `loops`'s MIR shapes; `settled`
  alone left both, as neither is a jump it settles.
- **Taking an `if` into the test only where the program wrote it in the
  `while`'s condition**: the `while (true)` rule takes any, and a `while`
  whose body is an `if` and an `else` of a `break` is already written so
  where nothing's guarded. One rule, not one per where the `if` came from.

### What holds it

- `test/lowering-written-loops.test.ts`: "a while in a guard's scope is
  its test", "a loop that ends a block breaks as a loop", run in JS, and
  "React's describeNativeComponentFrame's loops are upstream's", of the
  port as react-in-rust writes it (`test/ports/component-stack-frame`).
- Corpus `loop_value_after_loop.rs`, `calculator.rs`, `returns_in_place.rs`,
  against native Rust.
- Mutations `while-test-of-true-only`, `while-test-joined-dropped`
  (`mir/tidy/loops.rs`), `loop-ending-block-kept`,
  `loop-ending-block-not-last` (`mir/tidy/jumps/continues.rs`); and
  `tail-continues-unwritten-kept` (`mir/tidy/stages.rs`), of the pass
  where it runs now, held by "a loop that ends a block breaks as a loop"
  too.

## Amendment: a one-field tuple guard's field of a variable is that variable

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value
read where the guard's field was; only how the JS reads changes.

React's `runWithFiberInDEV` restores `setCurrentFiber(previousFiber)` in
its `finally`. Its port's guard is a tuple struct, `struct
Restore(Option<Fiber>)`, made `Restore(previous_fiber)`, which ADR 0608
takes apart into a `const` of its own, named after the guard, `const
restore = previousFiber;`, read `setCurrentFiber(restore)`; a named
field's guard, `Restore { previous }`, read `previous` itself, by this
ADR's rule.

**A one-field tuple guard made of a variable that nothing sets once the
guard's made, and the function declares once, reads as that variable**,
its `const` gone, as a named field's does: the same `kept_variables`
answers both (`drops/inlined.rs::one_field_of`). Any other field is a
`const` of its own, as ADR 0608 made it: `let mut previous = n; let
_restore = Restore(previous); previous += 1;` reads `restore`, what the
guard was made of.

Six corpus programs read their guard's variable where they read a copy
of it, `console.log(`drop ${n}`)`, and a shadow of it named apart goes,
`noisy$1` (`shadow_set_in_cases`): five fewer renamed names
(`docs/readability.txt`).

Held by `test/lowering-drop-guards.test.ts`, "a one-field guard of a
variable reads as that variable", run in JS, and its variable set later;
the corpus case `drop_guard_early_return` against native Rust; and the
mutations `one-field-variable-copied`, `one-field-variable-set-later` and
`one-field-variable-guard-kept` (`drops/inlined.rs`).

# 0706. `panic!` is the one way a port throws an `Error`, and what's caught is read as it was thrown

Status: Accepted, by the user's decision of 2026-10-24. Amends
[0012](0012-panics-and-runtime-helpers.md), whose panic of an empty
message was `new Error("")`, and [0660](0660-catch-unwind-catches-what-js-throws.md)
and [0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md), whose
`catch` read what it caught through `$unwound`, and whose `resume_unwind`
of it was `$unwinding(payload)`. Amended by
[0710](0710-every-unwinds-payload-is-the-js-error-thrown.md): a downcast to
a `&JsError` is what was thrown of any `try`, not only one of panics;
below: a `catch_unwind` that heads an `if let` chain is a `try`; one
tested after what an `if` asks first is a `try` inside it; and by
[0830](0830-what-follows-a-let-err-of-a-closure-that-never-returns-is-its-catch.md):
the `catch` of a `let Err(x)` is what follows that reads `x` or what that
declares, with no `let` before the `try`; and below (rd550): the payload
is made where the `catch` first reads it, and resumed as what was thrown
where nothing writes through it, as lowering says, not where it's only
read as JS shows; and below (rd551): the `catch`'s binding is given apart
from a `thrown` of the program's, which no longer keeps the helper; and
below (rd534): a closure's value a `try` keeps in a variable declared
just before it sets the variable where the value is made, where nothing
could see it set early; and below (rd60): a closure that returns early
sets its value in each branch, and a value never named is the call where
it's read first; and below (rd60): an `Ok` arm of a literal the closure
returns is where the closure returns it.

Case ([0262](0262-when-rust-and-js-disagree.md)): A where what's read is
the same value, `$unwinding($unwound(e))` being `e`; for `new Error()`,
what the program can't observe: its `message` is `""`, as `new
Error("")`'s, which JS keeps as its own property and `new Error()` as its
prototype's, and Rust reads only the text.

## Context

React's `DetermineComponentFrameRoot` makes its control error by throwing
one, so the engine writes its `stack` where it's thrown:

```js
try {
  throw Error();
} catch (x) {
  control = x;
}
```

A port had three ways to write it, none reading as React's:
`Some(js::js_error::new(""))`, `new Error('')` made, not thrown;
`resume_unwind(Box::new(error))` or `panic_any(error)`, which rustc
refuses, as a `&JsError` isn't `Send`; and `catch_unwind(|| panic!(""))`,
whose JS was `throw new Error("")` and a `catch (thrown) { const x =
$unwound(thrown); control = x.impl.typeId() === "&js::JsError" ? x.value :
x.error; }`.

## Decision

**`panic!` is the one way a port throws a JS `Error`.** `panic!("msg")` is
`throw new Error("msg")`, as before, and `panic!("")` is `throw new
Error()`: `Expr::panic`, the one place rust-js makes a panic's `Error`.
`resume_unwind` and `panic_any` of a `&JsError` stay rustc's refusal.

**What a `catch` reads of what it caught is what was thrown**, where that
is what it reads:

- **`resume_unwind(payload)` of a payload only read**, its fields and
  nothing written through it, is `throw thrown`: the same object, as
  `$unwinding($unwound(e))` is `e`.
- **A downcast to a `&JsError` where the `try` is only panics**, `throw
  new Error(..)` of literals and variables, is the `Error` thrown: an
  operator or a read in it may run JS that throws any value, `"" +
  value` a `TypeError` of a symbol.
- **A `catch` that reads the payload no other way is of it**, by the
  `Err`'s binding: `catch (x)`.
- **`let Err(x) = catch_unwind(f)` of an `f` that never returns**, whose
  `Ok` is none, `!`'s, is the `try` too: its `catch` what follows, up to
  its last read of `x`, as what follows runs only once `f` threw; what
  that declares and the rest reads a `let` before the `try`. (Amended by
  0830: what follows that reads `x` or what that declares, transitively,
  with no `let` before the `try`.)

```rust
let Err(x) = catch_unwind(|| panic!(""));
control = x.downcast_ref::<&'static JsError>().copied();
```

```js
try {
  throw new Error();
} catch (x) {
  control = x;
}
```

```rust
match catch_unwind(render) {
    Ok(n) => n,
    Err(payload) => resume_unwind(payload),
}
```

```js
try {
  return render();
} catch (payload) {
  throw payload;
}
```

## Why

- **It's React's JS**: the `throw`, the `catch (x)` and `control = x`.
- **One way**: a port writes a throw as Rust writes one, a panic; the
  `Error` it makes is the panic's, a `&JsError` to a `catch_unwind`.
- **The same object**: Rust's `resume_unwind` gives on the box it was
  given, and JS's `throw` of what was caught the object it caught; a
  payload of the program's own is the same `$Unwinding`, where
  `$unwinding` made another of it.

## Alternatives

- **`js::throw(error) -> !`**, a binding for JS's `throw`: a second way to
  unwind beside a panic, and the user decided against it.
- **`panic_any(error)` of a `&JsError`**: rustc's `Send` bound refuses it,
  as it should; a payload that crosses threads in Rust isn't JS's.

## Consequences

- `panic!("")` reads `new Error()` everywhere: in a `catch_unwind`, a
  `Drop`, a function's body.
- A `catch` whose payload is read otherwise, `downcast_ref::<String>()`,
  keeps its `$unwound(thrown)`; one whose `try` calls what may throw
  anything keeps its downcast's test, as what's thrown may be any value.

- A `let Err(x)` in a closure that sets a variable it captures reads
  through a temporary, `catch (x) { const value = x; control = value; }`
  (0830): a capture's assignment isn't read in place yet.

## What holds it

- `test/lowering-unwinds.test.ts`, "a port throws an Error by panic! and
  catches it as thrown", run in Node: the control error's `stack`, a
  resume rethrowing the same object, and a payload downcast to `String`.
- Corpus case `catch_unwind_rethrown` against native Rust; the
  architecture test "a panic's Error is made in one place".
- Mutations `resumed-made-again`,
  `resumed-after-write`, `thrown-error-of-any-try` (replaced by 0710's
  `thrown-error-read-as-payload`),
  `catch-unnamed`, `unwound-let-kept`, `unwound-let-declared-inside`
  (`prepare/catches.rs`), `panic-of-nothing-written` (`js/make.rs`).

## Amendment: a `catch_unwind` that heads an `if let` chain is a `try`

React's `DetermineComponentFrameRoot` catches its sample, then tests the
control its `try` set:

```js
try { .. } catch (sample) {
  // This is inlined manually because closure doesn't do it for us.
  if (sample && control && typeof sample.stack === 'string') {
    return [sample.stack, control.stack];
  }
}
return [null, null];
```

Its port writes `if let Err(sample) = catch_unwind(AssertUnwindSafe(|| {
.. })) && let Some(sample) = sample.downcast_ref::<&'static JsError>() &&
let Some(control) = control && let Some(sample_stack) = sample.stack() {
return ..; }`, and rust-js wrote `const value = $catchUnwind(() => {..});
if (value.TAG === "Err" && control) { let sample = value._0; sample =
sample.error; .. }`: the downcast is always `Some` (0710), so its test
left the chain, and the `if`'s test was the `Result`'s `TAG` and the
rest, which `try_catch` didn't know.

**An `if let Err(e) = catch_unwind(f) && c` chain is `try { f's body }
catch (e) { if (c) { .. } }`**: the `if` of its `Err` alone, `c`'s inside
it (`unchained`, `prepare/catches.rs`), which is then the `match` of a
`Result` this ADR and 0660 make a `try`.

- **`c` is tested in the `catch`**, once the body threw, as Rust tests it
  once `catch_unwind` returned: what the closure set, `control`, is what
  `c` reads, the closure's body in the `try`.
- **What the arm sets the payload's binding to first, `e = e.error;`, is
  before `c`**, which reads neither, so the `catch` is of what was thrown,
  `catch (sample)`; `c`'s `if` is where `c` is written, after a comment
  before it.
- **An `else`, where the arm leaves, is after the `try`**, as it's what
  runs where the arm doesn't: `try { f(); } catch { if (wanted) { return
  "caught"; } } return "not";`. One where the arm goes on keeps
  `$catchUnwind`, as its `else` runs where `c` is false and not where the
  arm ran.
- **A `Result` tested where it's made**, `if ($catchUnwind(f).TAG ===
  "Err" && c)`, is one made, then tested (`made_where_tested`).
- **A closure's body whose value nothing reads, `(() => { ..; return
  step(n); })();`, is its body**, `..; step(n);`, the `return`'s value a
  statement where it does something (`js::applied_tried`).
- A chain an `Ok` heads, and one whose `Ok` is read after it, keep
  `$catchUnwind`, as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same tests in
the same order. Held by `test/lowering-unwinds.test.ts`, "a catch_unwind
that heads an if let chain is a try", run in Node; corpus case
`catch_unwind_chained` against native Rust; and mutations
`chain-not-unchained`, `chain-tested-where-made-kept`,
`chain-else-of-going-on-after`, `chain-else-dropped`,
`chain-shadow-left-in-if`, `chain-if-before-its-comment`
(`prepare/catches.rs`), `tried-last-return-dropped` (`js/applied.rs`).

## Amendment: a `catch_unwind` tested after what an `if` asks first is a `try` inside it

React's checkPassiveEvents asks whether it has a DOM, then tries:

```js
if (canUseDOM) {
  try {
    const options = {};
    Object.defineProperty(options, 'passive', {get: function () { passiveBrowserEventsSupported = true; }});
    window.addEventListener('test', options, options);
    window.removeEventListener('test', options, options);
  } catch (e) {
    passiveBrowserEventsSupported = false;
  }
}
```

Its port writes clippy's form, `if can_use_dom &&
catch_unwind(AssertUnwindSafe(|| { .. })).is_err() { .. }` (clippy's
`collapsible_if` asks for it over an `if` in an `if`), and rust-js wrote
`if (canUseDOM && $catchUnwind(() => { .. }).TAG === "Err") { .. }`
(rd222): `made_where_tested` took a `Result`'s test only at the chain's
head.

**`if (c && $catchUnwind(f).TAG === "Err" && d) { .. }`, with no `else`,
is `if (c) { try { .. } catch { if (d) { .. } } }`** (`asked_first`,
`prepare/catches.rs`): what's before the `Result`'s test is the outer
`if`'s, and the rest is the `if` this ADR already makes a `try`, inside
it. A `$try` of a JS call (ADR 0637) is split the same way.

- **It's the same program**: `if (a && b) S` with no `else` is `if (a) {
  if (b) S }` in JS, `b` run only where `a` is true, as `&&` runs it, so
  the closure runs only where `c` is true, as Rust's `&&` calls
  `catch_unwind` only then.
- **Only where the rest is a `try`**: where it isn't (an `Ok` read after
  it), the `if` stays one test, as before, not two `if`s around
  `$catchUnwind`.
- **An `else` keeps `$catchUnwind`**: it runs both where `c` is false and
  where the closure didn't throw, so splitting the test would write it
  twice.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, the same tests in
the same order. Held by corpus case `catch_unwind_tested_last` against
native Rust (one asked first, two, one after, an `Ok`, an `else`);
`test/lowering-unwind-arms.test.ts`, "a JS call's Result tested after what
an if asks first is a try inside it", run in Node; the guide's `passive` (`interop/exceptions.md`); and mutations
`asked-first-kept-in-test`, `asked-first-only-the-first`,
`asked-first-else-dropped` (`prepare/catches.rs`).

## Amendment (2026-11-02): the payload made where it's first read, and resumed as thrown where nothing writes through it (rd550)

React's Fizz `replayElement` pops the replay it pushed before anything
else in its `catch`, and rethrows what it caught:

```js
} catch (x) {
  task.replay = replay;
  if (typeof x === 'object' && x !== null && (x === SuspenseException || typeof x.then === 'function')) {
    ..
    throw x;
  }
  ..
}
```

It runs where JS's stack is spent, rendering very deep trees ("can
recover from very deep trees during resume"): there a call throws a
`RangeError` again, and a property write doesn't. The port's `if let
Err(x) = catch_unwind(..) { task.replay.replace(..); let thrown_value =
thrown_value_of(&*x); .. resume_unwind(x); }` printed `catch (thrown) {
const x = $unwound(thrown); task.replay = replay; .. throw
$unwinding(x); }`: two calls of the runtime's, `$unwound` and the
`$isPanic` it calls, before the program's first statement, so the write
never happened, and a call again at the rethrow. The rule above made a
resume `throw thrown` only where every read of the payload was a field's,
`x.impl`, as the `catch`'s JS showed; a payload given to a function,
`thrown_value_of(&*x)`, kept `$unwinding(x)`.

**The `catch` makes the payload where it first reads it**, `const x =
$unwound(thrown);` before the first of its statements that mentions `x`
(`prepare::catches::unwound_where_read`), of that statement's span, so a
comment before it is still before it. What comes before reads nothing of
the payload, so it runs first, as it would in Rust: the payload is what
`catch_unwind` gave, which nothing between changes.

**`resume_unwind(x)` is `throw thrown` where nothing writes through `x`,
as lowering says**: a variable of the body's own, in a body that lends no
`&mut` to what holds a `dyn Any` (`BodyFacts::resumed_as_given`, of the
THIR, carried as `Known::AsGiven` on its `$unwinding(x)`), and `x` the
`catch`'s `const`, never set again. Only through a `&mut` is what a
`Box<dyn Any>` holds written, `downcast_mut`'s, a function's given `&mut
*x`, a `ref mut` binding's, a closure's capture by `&mut`; a payload moved
to another variable first, `let again = e;`, is the same payload, and any
`&mut` of it, before or after, is in the same body. A closure's resume of
a payload it captured, React's error loggers' `setTimeout(() => { throw
e; })`, is where no body of its family, the function it's in and each
closure in that, lends one (`body_queries::resumes_as_given`, of the
crate's bodies), as its maker may change it first. This replaces the
rule's reading of the `catch`'s JS, which a write through an alias, `const
m = .. ? e : undefined; m.value += "?"`, kept right only by the alias
being a mention that isn't a field's.

```rust
if let Err(e) = catch_unwind(AssertUnwindSafe(|| deep(d - 1, wrote))) {
    wrote.set(wrote.get() + 1);
    read.set(message_of(&*e));
    resume_unwind(e);
}
```

```js
} catch (thrown) {
  wrote.value = (wrote.value + 1) >>> 0;
  const e = $unwound(thrown);
  read.value = messageOf(e);
  throw thrown;
}
```

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `throw thrown` is
`throw $unwinding($unwound(thrown))` of every value thrown: a
`$Unwinding` is its payload's `error`; a panic's `Error` the one whose
message its payload's is, unchanged; what JS threw itself. Making the
payload later reads the same `Error`'s `message`, `constructor` and
prototype later, which only JS holding that `Error` could change between,
and nothing Rust does; it's what JS's own `throw x` of a `catch (x)`
reads. What changes is that the helpers' calls, which could throw where
the stack is spent, or where a thrown Proxy's trap throws, aren't made
where the program makes none.

Held by `test/lowering-unwind-arms.test.ts`, "a catch makes its payload
where it's first read, and resumes it as thrown", run in Node: the
issue's repro, a JS `overflow` that recurses to a `RangeError`; a
recursion of a million levels whose every `catch` that entered its `try`
writes first, where the old JS's left some unwritten (5,423 to 5,655
of 5,658 in a run on the Mac); a `TypeError` read as a `&JsError` and resumed as the same object; a
panic's message read and resumed; one changed by `downcast_mut`, resumed
as what it is now. Corpus case `catch_unwind_payload_where_read` against
native Rust: a message read after a write and resumed; one changed by a
function given `&mut`, or by `downcast_mut` and moved to another variable,
or then resumed by a closure that took it; one resumed by a closure that
took it unchanged, `throw e` in the closure; another payload resumed by a
closure whose variable has the payload's name; guards dropped as each
scope unwinds; a typed payload; a `catch_unwind` in another's closure.
`catch_unwind_irrefutable`'s snapshot makes its payloads after a `const
guard` and after a `continue`. Mutations `unwound-made-first`,
`resumed-of-another-variable` (`prepare/catches.rs`, and
`resumed-made-again`, `resumed-after-write`, `thrown-error-read-as-payload`
moved to the new rule), `as-given-lent-unseen`, `as-given-of-a-shared-borrow`,
`as-given-of-a-captured-variable`, `as-given-captured-unseen`,
`as-given-coerced-unseen`
(`body_queries.rs`), `resumed-as-given-unmarked` (`std_types/unwind.rs`),
`holds-dyn-any-never` (`recognition/types.rs`).

## Amendment (2026-11-02): a name of the program's never keeps a `catch_unwind` a `$catchUnwind` (rd551)

React's Fizz `renderNode` keeps what each arm caught in a `let thrown;`
of its own, which it throws after it restores its task:

```rust
let thrown;
match segment {
    None => {
        let Err(thrown_value) = catch_unwind(AssertUnwindSafe(|| {
            render_node_destructive(request, task, node, child_index)
        })) else {
            return;
        };
        reset_hooks_state();
        ..
        thrown = thrown_value;
        ..
    }
    ..
}
```

It printed `const value = $catchUnwind(() => renderNodeDestructive(..));
if (value.TAG !== "Err") { return; } const thrownValue = value._0; ..`,
where upstream writes `try { renderNodeDestructive(..) } catch (x) { ..
}`. A `catch` of an unwind was always `catch (thrown)`, so one whose arm
read or declared a `thrown` of the program's own, which it would hide,
stayed the helper: renderNode's two arms, React's createChildReconciler
(`const thrown = x.error`), the Flight client's `thrown(error)`. Rendering
very deep trees ("can recover from very deep trees during resume"), the
helper's call and its closure are two frames more where JS's stack is
spent, and its `catch` runs `$unwound` before the arm's first statement,
so a restore the arm makes first is lost when that call throws a
`RangeError` again.

**The `catch`'s binding is `thrown`, or a name given apart, `thrown$1`,
from what its statements read or declare and from the payload's binding**
(`prepare::catches::thrown_name`, `names::fresh_apart`'s), in a `match`'s
`try` and a `let Err(x)`'s alike. Where the `catch` is of the payload,
`catch (x)`, the name is never written.

Two forms the `try` now meets often read as JS's:

- **A `try` that ends its function has no `return;` last**, the `return;`
  a `let Err(x) = catch_unwind(..) else { return; }` put there: it falls
  off the end, as the function would (`tail_tries`).
- **A closure of statements whose value a variable is set to, `result =
  (() => { ..; return v; })();`, is its statements, then `result = v;`**,
  as `const value = (() => ..)()` already was (`js::applied_tried`), the
  `result = v;` where the `return` was written, after a comment before it.
  Its names are apart from the function's, as lowering gives them, so one
  of the same name is `name$1` where reclaiming can't take it back, the
  variable set beside it: React's throwAndUnwindWorkLoop's `const
  didFatal$1 = throwException(..); .. didFatal = didFatal$1;`, where it was
  an arrow called.

`$catchUnwind` stays where the `Result` is a value: held past what's
done between, `let r = catch_unwind(f); restore(); match r { .. }`, whose
restore runs on both paths; given or stored; or tested where its `Ok` arm
may throw and its `Err` goes on, ReactDOMComponent's `if
(catchUnwind(..).TAG === "Ok") { trackHostMutation(); return; }`, which
a `try` could write only with a label or a flag.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same program:
the `catch`'s binding is a name nothing else in it reads, and the JS's
`try` is the `match` ADR 0660 makes one; it calls nothing where the
helper called `$catchUnwind`, its closure and `$unwound` first.
The readability baseline's `x$1` names grow (8,688 to 8,713, 612.6 to
613.5 in 10,000 lines), the `catch`'s `thrown$1` where the JS was the
helper, most of them the new corpus case's, which names `thrown` on
purpose.

Held by `test/lowering-unwind-arms.test.ts`, "a catch_unwind whose arm
reads a thrown of its own is a try", run in Node: renderNode's two arms,
a `Result` named then matched and a `let Err(..) else` at a function's
top, each reading a `thrown`, recursing until the stack is spent; every
level that entered its `try` writes in its `catch`, where the helper's
form, main's before, left levels unwritten in each of the four, on each
of three runs on the Mac. Corpus case `catch_unwind_thrown_named` against
native Rust: renderNode's shape with a guard dropped as its arm unwinds,
a parameter, a closure and a payload named `thrown`, a `let Err(..)
else` ending its function, a closure's value kept by an arm with a guard
after; `let_else_returns_in_arms` and
`shadow_beside_closure_own_of_its_name`, which printed `$catchUnwind`,
are `try`s. Mutations `thrown-shadowed`, `tail-try-return-kept`
(`prepare/catches.rs`), `tried-assigned-kept-applied`,
`tried-kept-where-made` (`js/applied.rs`).

## Amendment (2026-11-02): a closure's value a `try` keeps sets its variable where it's made (rd534)

Since rd551 a closure of statements whose value an arm keeps in a
variable is the closure's statements in the `try`, then `v = value;`.
React's throwAndUnwindWorkLoop made its value a `let` of its own:

```rust
let did_fatal = match catch_unwind(AssertUnwindSafe(|| {
    let did_fatal = throw_exception(root, ..);
    if did_fatal {
        panic_on_root_error(root, thrown_value);
    }
    did_fatal
})) {
    Ok(did_fatal) => did_fatal,
    Err(error) => { .. resume_unwind(error) .. return; }
};
```

which printed `let didFatal; try { const didFatal$1 = throwException(..);
if (didFatal$1) { panicOnRootError(..); } didFatal = didFatal$1; }`, a copy
of its own, where upstream writes `didFatal = throwException(..)`.

**The variable is set where the value is made**, `didFatal =
throwException(..); if (didFatal) { .. }`, the value's reads its reads,
where that's the same program (`js::tried_set_where_made`, run by
`prepare`'s blocks after their `try`s are made), each of these holding:

- **The variable is declared just before the `try`**, a `let` with
  nothing but `let`s after it: it's of the `try`'s block, and nothing
  made before, a closure, sees it.
- **What follows the value in the `try` never names the variable**, so
  reads none of it set early, and **reads the value in no closure**,
  `() => next`, which may run once the variable changes after.
- **Where what follows may throw, nothing sees the variable set early**:
  the `catch` never names it and never goes on, so leaves the block it's
  of, by a `return`, a `throw` or a `break`; or it sets it before it
  names it, `catch { value = 100; }`. A `catch` that goes on to what
  reads it, `catch { note(6); } return done;`, reads it before it sets
  it, or reads it as it leaves, `return done + 20;`, keeps the copy, so
  sees the variable as it was. So does a variable declared outside a
  loop whose `catch` breaks to what reads it.
- **No function the block declares names it**, which runs where it's
  called.

An arm's own binding of the value, `const found = found$1; done =
found;`, is a copy too, set where it's made in turn. A comment before the
closure's value, `// What it found.`, stays where it was, after what
follows the value, a `StmtKind::Comments` where the `v = value;` was.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same program:
the value is the variable's from where it's made, where nothing reads
either apart from the other, and nothing sees the variable before the
`try` would have set it.

Held by `test/lowering-unwind-arms.test.ts`, "a closure's value a try
keeps sets its variable where it's made", run in Node:
throwAndUnwindWorkLoop's shape, its `panicOnRootError` throwing, and a
variable with a value whose `catch` goes on, which keeps the copy.
Corpus case `catch_unwind_value_set_where_made` against native Rust:
throwAndUnwindWorkLoop's shape, its `catch` resuming or returning, with
a comment before the value; a `catch` that sets the variable; and the
shapes that keep the copy, each printing another answer where it's set
early: a `catch` that reads the variable, as it leaves or before it sets
it, or goes on to what reads it, a variable outside a loop whose `catch`
breaks, a closure that reads the value, and a read of the variable after
the value. Mutations `set-where-made-read-leaving`,
`set-where-made-goes-on`, `set-where-made-read-before-set`,
`set-where-made-declared-anywhere`, `set-where-made-read-between`,
`set-where-made-closure-reads`, `set-where-made-throw-unasked`,
`set-where-made-comment-lost` (`js/applied.rs`) and
`set-where-made-never` (`prepare.rs`).

## Amendment (2026-11-02): a closure that returns early sets its value in each branch, and a value never named is the call where it's read (rd60)

Two shapes of a `try`'s value still printed what no one writes.

A closure that returns early, its value a `let`'s in an outer
`catch_unwind`'s closure, stayed a closure called, as only a body that
returns at its end was its statements:

```js
let found;
try {
  found = (() => {
    if (n === 0) {
      return 0;
    }
    const a = step(n);
    return (a * 2) >>> 0;
  })();
} catch {
  return 3;
}
```

**Each `return v;` sets the value, what follows a guard its `else`**
(`returns_set`, of `applied_tried`'s): `if (n === 0) { found = 0; } else {
const a = step(n); found = (a * 2) >>> 0; }`, of a `const` a `let` before
it. Where each `return` ends what runs of the body, last, or last of an
`if`'s branch, the other branch, or what follows the `if`, going on to
what follows it, of one branch at most, so nothing's written twice. One in
a loop, a `switch` or a `try`, whose `return` leaves more than its branch,
stays a closure called, as does a body that declares the variable's name.

And a value the program never named, tested by the arms, `Ok(true) =>
{} Ok(false) => return false`, was named after the `Result` it was read
from, a temporary MIR named after `catch_unwind`: React's
isRenderConsistentWithExternalStores printed `const catchUnwind =
objectIs(getSnapshot(), renderedValue); if (!catchUnwind) { .. }`, where
upstream writes `if (!objectIs(getSnapshot(), renderedValue))`. **Read
once, by the statement after it before anything else that statement
evaluates, it's the call there** (`js::read_where_made`, the same
evaluated-first rule `value_of` inlines a closure by): nothing runs
between. A comment before the closure's value is before that statement.
A value the program names, `Ok(found) => if found { .. }`, keeps its
name.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same program:
each `return` set the value and ended the closure, as each branch sets
it and ends; and the call runs where its value was read first, with
nothing between.

Held by `test/lowering-unwind-arms.test.ts`, "a closure that returns
early sets the value a try keeps in each branch" and "a try's value read
once where it's evaluated first is the call there", run in Node; corpus
case `catch_unwind_value_in_place` against native Rust: the stores loop,
a value tested where it's returned after a comment, values the program
names read once and twice, a guard in an outer `catch_unwind`, an `else
if` chain with a panic, and a `return` in a loop. Mutations
`value-in-place-never`, `value-in-place-of-named`
(`prepare/catches.rs`), `value-in-place-comment-moved`,
`returns-set-never` and `returns-set-rest-dropped` (`js/applied.rs`).

## Amendment (2026-11-02): an `Ok` arm of a literal the closure returns is where it returns it (rd60)

React's throwAndUnwindWorkLoop returns from its function where the
closure finds the error fatal, which a closure can't do itself, so its
port gives out a flag and the arms act on it:

```rust
match catch_unwind(AssertUnwindSafe(|| {
    let did_fatal = throw_exception(x);
    if did_fatal {
        panic_on_root_error(x);
        return true;
    }
    false
})) {
    Ok(true) => return 0,
    Ok(false) => {}
    Err(error) => { .. }
}
x + 1
```

It printed `let catchUnwind; const didFatal = throwException(x); if
(didFatal) { panicOnRootError(x); catchUnwind = true; } else {
catchUnwind = false; } if (catchUnwind) { return 0; }`, a flag named
after the `Result`, where upstream writes `try { const didFatal =
throwException(x); if (didFatal) { panicOnRootError(x); return; } }
catch (error) { .. }`.

**Each `return v;` of a literal is what the `Ok` arms do where the value
is `v`** (`catches::arms_where_returned`): `return 0;` where the closure
returns `true`, and nothing where it returns `false`, which falls out of
the `try` to what follows it; the rest of the arm, which never reads the
value, `return x + 1;`, after the `try` where the `catch` leaves, else in
it where it can't throw. The arms are taken by their tests, each `if` of
the value, `!` of it or `===` and `!==` of it and a literal, a number, a
string, a char's or a `bool`, the branch that runs. Each `return` is
replaced where it is (`js::returns_replaced`, what `returns_set` was,
the one walk of a closure's `return`s that both use): an arm that ends,
`return 0;`, where the `return` was, what follows its `if` after it;
one that goes on only where the `return` ends what runs of the body,
what follows its `if` the other branch's, `if (x !== 0) { .. }` of a
guard's `return false;` that does nothing. In a loop or a `switch` of
the closure's, an arm is placed only where it returns from the function,
as the closure's `return` did, ending with no `break` or
`continue` in it, which would be of the closure's loop: the Flight
client's fulfillReference returns from inside its loops, `Ok(false) =>
return`, as upstream's `try` does. A `return` in a `try` of the closure's
keeps the flag, as its `finally` would run after the arm, not before.
React's dispatchSetStateInternal, `if let Ok(true) = r { return v; }`
(`flagged_try`), is the same rule, its `if` the arm, where `flag_returns`
was a walk of its own.

Moving an arm into the `try` changes what its `catch` catches, so only
what can't throw moves: **an arm that may throw, `Ok(true) => { fatal(x);
return 2; }` or `Ok(true) => panic!(..)`, keeps the flag**, the arm after
the `try`, where what it throws goes on past the `catch`, as Rust's goes
on past the `catch_unwind`. An arm placed where the closure returns its
value in more places than one is one jump, `return 0;` or `break;`, never
statements written twice; else the flag stays. A value the program names,
`Ok(found) => ..`, keeps its name; a closure that returns what isn't a
literal keeps the flag.
What's said before a `return` the arm replaces is before what the arm
does there. What's said in an arm, and before it, `// Where it's thrown.`
above `Ok(true) =>`, is before its statements where it's placed at the
closure's last statement; placed before more of the closure, an arm's
statements are of no place in the source, as a comment the closure
writes after that `return`, `// TODO: Mark downstreams as halted too.`
of the Flight client's next branch, is before what follows it, not
taken into the arm.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same program:
where the closure returned `v`, the arm of `v` ran next, nothing between,
and it runs there now; it throws nothing the `catch` could catch, and
what does nothing goes on past the `try`, as past the `match`.

Held by `test/lowering-unwind-arms.test.ts`, "an Ok arm of a literal the
closure returns is where it's returned", run in Node:
throwAndUnwindWorkLoop's shape, its `catch` resuming or returning, and an
arm that throws, which its `catch` never catches. Corpus case
`catch_unwind_arms_where_returned` against native Rust: that shape, an
arm that panics and one that calls what panics, numbers whose `catch`
goes on, chars, a guard's `return` of what does nothing with comments, a
value returned twice whose arm is more than a jump, a loop's `break`
beside a name the closure declares that its arm reads of its own, and
`return`s in the closure's loop whose arm returns, may `break`, or does
nothing; `catch_unwind_flag_return` holds `flagged_try`'s. Mutations
`arms-returned-never`, `arms-returned-throwing`,
`arms-returned-rest-in-try`, `arms-returned-negation-unseen`,
`arms-returned-ne-as-eq`, `arms-returned-written-twice`,
`arms-returned-comment-lost`, `arms-returned-spans-kept` (`prepare/catches.rs`),
`returns-replaced-goes-on-anywhere`, `returns-replaced-nothing-unnegated`,
`returned-in-loop-jumps` and `returned-in-loop-goes-on`
(`js/applied.rs`); `returns-set-never`
and `returns-set-rest-dropped` find the walk where it is now.

## Amendment: a value the program names is read in place too (rd16)

React DOM's getActiveElement, `try { return doc.activeElement || doc.body; }
catch (e) { return doc.body; }`, is ported as a `#[rust_js::nullable]`
function whose `Ok(active_element) => active_element` the result's
conversion makes `return activeElement ?? null;`. It printed `const
activeElement = doc.activeElement ?? doc.body; return activeElement ??
null;`, because a value the program names kept its name in the `try`
(the amendment above). Out of a `try`, tidy reads any `const` that's
read once where it's read, a name the program gave it too: the same arm
without `catch_unwind` prints `return doc.activeElement ?? doc.body;`.

**A value the program names is read where it's read, as one it never
named is** (`js::read_where_made`, asked of every `Ok` value the `try`
keeps): read once, by what follows, before anything else that evaluates.
The result is `return doc.activeElement ?? doc.body ?? null;`. The
`?? null` is bindings run 18's. This replaces "a value the program names
keeps its name", which was a second rule beside tidy's for one place.

m6-wife cleared the change on one condition: what goes into the `try`
must not throw. That holds, since `read_where_made` is asked only of what
the `try` holds already. An `Ok` arm whose rest can throw is after the
`try` (ADR 0637), because Rust's `catch_unwind` doesn't catch a panic in
the arm. A value read twice, `word != null ? word : "-"`, or after
another read, `before + (word ?? "")`, keeps its name.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, none. The call runs
where its value was read first, with nothing between, in the same `try`.

Held by the corpus case `catch_unwind_value_read_first` against native
Rust. It covers the fold, an `Ok` arm whose `expect` panics past the
`catch_unwind`, a sum whose other operand panics, both caught outside as
Rust catches them, and both negatives. `catch_unwind_value_in_place`'s
`namedOnce` and `shadow_of_module_variable_in_try`'s `raised` lose their
`const`. The mutation `value-in-place-of-named`, which this rule now
is, becomes `value-named-kept` (`prepare/catches.ts`), which puts the
old rule back.

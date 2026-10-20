# `catch_unwind` catches a Rust panic

Status: Draft; blocked on the abort gate below. Would amend
[0012](../0012-panics-and-runtime-helpers.md),
[0026](../0026-testing.md), and [0098](../0098-destructors.md); extend
[0331](../0331-dyn-any.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)) for panic identity,
typed payloads, ownership and cleanup; D for unsupported behavior.
Changing a payload's type is not a harmless B difference.

## Context

Panics currently throw ordinary `Error`s. `$shouldPanic` recognizes an
`Error` with `constructor === Error`, but a host dependency can throw
exactly that. Its predicate cannot establish that a thrown value is a
Rust panic. Runtime refusals also throw errors, and must never become
successful `Err` results from `catch_unwind`.

An error message is not a panic payload. A `String` and an `&'static str`
can have identical text but different `Any` downcasts. Catching a panic
also transfers ownership of its payload to the caller, which may inspect,
drop or resume it.

## Decision

**A shared panic runtime owns panic construction, recognition and unwind
state. `catch_unwind(f)` calls `f` once and catches only a Rust panic
identified by that runtime, returning its owned, typed payload.**

```text
f()
 |-- returns value --------> Ok(value)
 |-- Rust panic -----------> cleanup --> Err(owned Any payload)
 |-- host error/refusal ---> propagate unchanged
 |-- abort ---------------> never Err
```

### One identity and one payload representation

- All generated panic sites and runtime helpers use the same constructor
  and predicate. Keep useful JS error messages and stacks, but do not
  recognize panics by message text, constructor name or ordinary
  `instanceof Error`. A private runtime brand belongs to the compatible
  shared runtime instance; packaging must keep that identity consistent
  across compiled libraries and chunks.
- Migrate `$shouldPanic`, assertion helpers and the differential oracle
  to that predicate too. Classify existing throws deliberately: bounds
  checks and division panics are Rust panics; compiler/runtime refusals
  and implementation failures are not. No fallback accepts unbranded
  errors. Test duplicate/incompatible runtime installation explicitly.
- The payload uses the existing `Any` representation, type identity and
  drop evidence, not a second panic-only type tag. `panic_any(String)`
  stays a `String`; `panic_any(&'static str)` stays that type. A failed
  downcast remains failed. A non-string payload is supported only if its
  `Any` value and ownership are representable; otherwise refuse where it
  is introduced. A printable message alone is never sufficient.
- Determine `panic!` payloads from the pinned frontend/std behavior,
  including formatted and literal forms. Do not infer their types from
  whether the final text looks constant.

### Catch, resume and drop

`catch_unwind` returns the existing `Result` representation. It invokes
its `FnOnce` and drops the closure's remaining captures as Rust does,
including when the call panics. The payload moves from the panic envelope
into `Err`; it is not cloned, formatted into a replacement value or
dropped by both owners.

`resume_unwind(payload)` takes ownership of the supplied `Box<dyn Any +
Send>` and starts unwinding without invoking a panic hook. The argument
need not have come from an earlier catch, so an implementation cannot
require a saved original `Error`. A new internal envelope can carry the
same owned payload.

`AssertUnwindSafe` remains representation-transparent while rustc checks
its use. `UnwindSafe`, `Send` and `'static` bounds are never bypassed.
This is synchronous catch behavior: do not turn it into a Promise
rejection handler.

`set_hook` and `take_hook` remain refused until hook timing and ownership
are implemented. Acceptance must also state the default-hook contract:
native Rust invokes it before unwinding even if a panic is caught.
Existing differences in host diagnostics must be named, not mistaken for
equivalent output.

### Abort gate: unresolved

The current destructor contract explicitly permits a second panic during
unwinding to replace the first, where native Rust aborts. Making that
replacement catchable would let Rust code continue where native Rust
cannot. Plain nested `try`/`finally` does not fix this.

**Keep `catch_unwind` refused until a separate, explicit abort contract
resolves this gate.** It must cover the following:

- The runtime distinguishes active unwinding from an ordinary destructor
  call; nested catches restore unwind state correctly.
- A second panic while unwinding cannot become an ordinary catchable
  panic, and further cleanup cannot continue as if Rust had recovered.
- Each supported host has a specified termination/refusal boundary.
  Throwing another JS error does not terminate a browser realm or prevent
  arbitrary JS from catching it. Do not claim process-abort equivalence.
- Abort-mode behavior and unsupported host configurations are explicit.
  The compiler rejects configurations it cannot support; a runtime-only
  refusal follows ADR 0262 and remains distinct from a Rust panic.

This draft chooses no silent new divergence. Resolving the gate may
require a dedicated ADR before this one can be accepted.

#### A proposed resolution, for the user's acceptance

Written 2026-10-21 while accepting the other drafts; nothing of it is
implemented, and it isn't accepted until the user accepts the browser
difference it names.

- **An abort is the runtime's one state, `$aborting`**, set where a panic
  starts while another unwinds: the panic runtime already knows when it's
  unwinding, as the destructor contract's drop helpers run in `finally`.
- **On Node, an abort is the process's end**, `process.abort()`, after
  writing what native Rust writes: the same exit as native Rust's, no
  further JS. Equivalent, as far as an observer outside the process can
  tell.
- **Where a host can't end its realm, a browser's**, an abort throws a
  branded `$Abort`, which no `catch_unwind` catches and no Rust frame
  recovers from, and leaves the runtime poisoned: every later call that
  enters the panic runtime, a panic, `catch_unwind`, a drop helper, throws
  the same `$Abort`. JS `finally` blocks between the abort and the host
  still run, which native Rust's abort never would: that's the difference
  to accept by name, of Case B under [0262](../0262-when-rust-and-js-disagree.md)
  only if no Rust code can observe it, which the poisoning is for.
- **`panic = "abort"`** is the same state from the first panic.

Without that acceptance, `catch_unwind` stays refused.

## Acceptance

Differential cases cover normal returns, literal and formatted panics,
typed `panic_any` payloads, matching and mismatching downcasts, payload
mutation, successful ownership-taking downcasts, and payload drops.
Include resuming a caught payload and resuming a newly boxed one.

Record destructor order through nested catches and partial moves, closure
capture cleanup, and payload destruction that itself panics. Isolate
native abort cases in subprocesses; use host-specific isolation for JS.
Plain host `Error`, `TypeError`, thrown primitives and runtime refusals
must propagate unchanged. A panic crossing compiled-library and chunk
boundaries must still be recognized.

Mutations remove the brand check, accept every error, erase the payload's
type, duplicate or lose its drop, invoke a closure twice, or treat a fatal
unwind as a recoverable panic.

## Consequences

This is a migration of the panic runtime and its users, not one small
`try` helper. Existing `Any`, `Result` and drop owners keep their jobs.
No limitations entry is removed until both the runtime migration and
abort gate are complete. The semantic reference and earlier ADRs must
then describe the actual supported panic contract together.

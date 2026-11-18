# 1173. A property read runs nothing only of what a builtin made

Status: Accepted (2026-11-18, the user's decision: fix, not accept; #2106).
Amended 2026-11-18 (#2110): a variable is at hand only where every write
of it is. Amended 2026-11-18 (#2109): of the program's own structs, as
`fn_pointers.rs` says JS may make or give them, and of a type parameter's
value, as each caller's instantiation says.
Amends [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
whose Consequence it narrows to ADR 0262's difference 60, and [0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md);
adds `#[rust_js::creates]`.

## Context

A binding's property read, `this.name` (`JsForm::Get`) or `this[key]`
(`JsForm::GetIndex`), was taken to run none of the program's code, of any
object. ADR 0580 recorded what that missed: a getter of an object a JS
caller handed over may call back into the program and borrow a cell the
read is held across, where Rust's borrow would panic `already borrowed`
and the JS's passes. It was documented, in ADR 0580 and the guide's
differences, but never an exception the user decided (ADR 0262): a wrong
answer, which the user decided to fix.

## Decision

A property read runs nothing only of a value at hand, one a builtin made,
whose getters are none of the program's (`cell_borrows/runs_nothing.rs`,
`read_of_what_is_at_hand`); of any other, a parameter's, a field's, what a
collection kept, it's counted, as any call that may run the program's code
is.

Which values may meet a getter of the program's is `read_may_run`'s,
of the type read (`read_of` traces the receiver through borrows, derefs
and casts to a JS value, then `Box`, `Rc` and `Arc`):

- A JS object's type (`Recognition::is_js_object`).
- The program's own struct where JS may make or give a value of it, as
  `fn_pointers.rs`, its one owner, says (`js_may_make`, `js_may_give`,
  ADR 1022): `js::get(js::unknown(chunk), "reason")` of the `&Chunk` an
  exported function takes, which the crate never builds, is counted
  (#2109). One only the crate constructs, which JS gives back as the crate
  gave it, is plain data, ADR 0262's difference 60 (Tu, 2026-11-15): its
  read runs nothing, as of the Flight server's `ReactPromise`, whose
  `then` JS calls, or the scheduler's `Task`, which `cancelCallback`
  takes. A getter JS adds to one, or an object of its own in its place,
  isn't seen; difference 60 says JS mustn't.
- A type parameter's, the scheduler heap's `get sortIndex` of `T: Node`:
  its caller's to settle, as a drop of one is. `called` gives it as
  `Called::Read(ty)`, the body's summary keeps it in `reads`, and each
  instantiation (`instantiated`) settles it by `read_may_run`: a heap of
  `Rc<Task>`s reads plainly, one of a JS caller's tasks is counted. Where
  no instantiation says, it's counted.

Nothing else, a `String`'s or a number's, has a getter of the program's.
#2108, a collection's elements a builtin made, isn't needed for this; it's
its own (P2).

What a builtin made is `AtHand`'s, one crate-wide set of variables built
to a fixed point over every body's THIR:

- What a constructor makes, `new X` (`JsForm::New`), or a binding that says
  it `creates` its result, `regExp.exec(s)`'s match, `JSON.parse(text)`'s
  values, `Intl.Locale`'s `maximize()` and `minimize()`.
- Through `unwrap`, `expect`, `clone` and `?`; a `let`, an `if let` or a
  `match` of one, `let Ok(tag) = Intl.Locale(..) else`; the parameter of an
  `Option`'s or a `Result`'s combinator's closure given one's payload,
  `.and_then(|matched| matched.get(1))`; a closure's capture of one, by its
  root variable.
- Never what a builtin hands back from what a JS caller stored: `Map.get`'s,
  an element's, `Reflect.get`'s, a promise's value. Nor a DOM query's
  element, which may be a custom element whose class has getters.
- A variable is at hand only where every write of it is: its binding's
  and each assignment's, to it or to a closure's capture of it, `locale =
  tag`. One lent `&mut` or `&raw mut`, `std::mem::swap(&mut locale, ..)`,
  or bound `ref mut` in a pattern of it, isn't: what's written through the
  loan isn't seen. As first landed, a variable was at hand by its binding
  alone, so a JS caller's locale assigned over one `maximize()` made was
  read plainly under a held cell, where Rust's re-borrow panics (#2110,
  P0). The fixed point stays the least: each condition holds of more as
  more is at hand, so a variable only a cycle of writes gives is never at
  hand, as is sound.

`#[rust_js::creates]` is a binding fact: generated builtins have it from
`builtins/policy.ts`' `CREATES`, which the generator writes through the one
`attributesOf`, beside `RUNS`; handwritten ones by hand, each with why. A
library author marks a binding `creates` only where its result is an object
it makes itself, every time; a wrong mark is unsound, a getter of the
program's then unseen. `binding_call_runs_nothing` is the one rule: the
borrow analysis passes the call's THIR arguments and the crate's `AtHand`;
lowering, which has none, takes no property read to run nothing.

## Why

ADR 0262's third case: where a getter calls back, the results change. A
property read of a value the program made itself, the vercel-ui SDK's
locale, its regex's match, still reads plainly: its memo of a locale's first
day, `firstDayOfWeek`, has no borrow check.

## Consequences

- The corpus doesn't move. test/snapshots' `playground/compiler.js` holds
  its `stderr` cell's borrow, checked, while it reads `memory.byteLength`,
  of the wasm instance's exports, which no builtin is known to have made:
  correctness-forced, its `stderr$1` and `memory$1` growing
  docs/readability.txt's `renamed x$1` share (re-blessed).
- React's port, react-in-rust 2026-11-18: `collectClientRects` (react-dom)
  checks `rects`' borrow while it reads a `DOMRectList`, and the Flight
  webpack plugin `compilation.warnings`' while it reads
  `this.clientManifestFilename`: correctness-forced. Its
  `getClassNameByType` iterates a copy of the transition's types, `const
  types = activeTypes.slice()`, the user's decision: a props getter may
  call `addTransitionType`, which borrows that cell mutably, and with the
  cell borrowed across the loop rust-js refuses it (a library's cell
  borrowed unchecked while code it can't see runs); upstream sees a type
  added mid-loop, the copy doesn't (the port's PORTING.md).
- A borrow of a cell any such read is held across is counted in every body
  that borrows that cell, as the getter may re-enter any of them.
- ADR 0580's Consequence and the guide's difference now name what's left,
  difference 60's.
- #2109 moves nothing: not the corpus, the snapshots nor React's port (the
  Flight server's `ReactPromise` and the scheduler's `Task` are difference
  60's), so neither its port change for `_serverReferenceCache` nor any
  provenance of parameters or collections was needed.

## What holds it

test/builtins-runs-nothing.test.ts: a JS caller's locale read in a memo's
borrow, and under a `RefCell` guard, keeps the check; the match `exec`
made, through `and_then`, reads plainly; a value a `Map` kept is counted.
A JS caller's locale assigned over one a builtin made, by the function or a
`move` closure, or a variable lent `&mut` or bound `ref mut`, keeps the check
(#2110).
Mutations `at-hand-of-any-variable`, `at-hand-of-any-capture`,
`at-hand-of-any-binding`, `creates-unread`,
`at-hand-combinator-parameter-lost`, `regexp-exec-not-creating`,
`library-methods-create-unmarked`, and #2110's `at-hand-assignment-unread`,
`at-hand-loan-unread`, `at-hand-mut-borrow-not-lending`,
`at-hand-ref-mut-not-lending` and `at-hand-capture-assignment-unseen`.
#2109's rows: the program's own `Chunk` a JS caller gives, cast to a JS
value, keeps the check, its getter that notes again throwing as Rust's
panic; one the crate constructs reads plainly; a generic heap of tasks the
crate constructs reads `sortIndex` plainly under its queue's borrow, one of
a JS caller's tasks keeps the check, its re-entering getter throwing.
Mutations `struct-js-reach-ignored`, `struct-crate-built-counted`,
`rc-not-peeled`, `generic-read-not-deferred`, `generic-read-unkept` and
`generic-read-settled-plain`; `cast-to-js-value-not-traced` moved to the
crate-built row; `impls-ask-nothing`, `unseen-never`,
`class-trait-call-asks-nothing` and `any-call-asks-nothing-seen`
re-anchored, caught.

# 1173. A property read runs nothing only of what a builtin made

Status: Accepted (2026-11-18, the user's decision: fix, not accept; #2106).
Amends [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md),
whose Consequence it narrows to #2109's case, and [0867](0867-a-binding-that-runs-nothing-and-aggregate-error.md);
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

Of a JS object's type (`Recognition::is_js_object`), that is: a read of
what isn't one, the program's own Rust struct cast to a JS value,
`js::get(js::unknown(&**chunk), "reason")`, or a type parameter's, the
scheduler heap's `get sortIndex` of `T: Node`, still runs nothing, as
before (`read_of` traces the receiver through borrows, derefs and casts).
A JS caller may forge such a struct with a getter where an exported
function takes one: #2109 (P1) is that case, and #2108 a collection's
elements the program made, which the first needs.

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
  #2109's.

## What holds it

test/builtins-runs-nothing.test.ts: a JS caller's locale read in a memo's
borrow, and under a `RefCell` guard, keeps the check; the match `exec`
made, through `and_then`, reads plainly; a value a `Map` kept is counted.
Mutations `at-hand-of-any-variable`, `at-hand-of-any-capture`,
`at-hand-of-any-binding`, `creates-unread`,
`at-hand-combinator-parameter-lost`, `regexp-exec-not-creating` and
`library-methods-create-unmarked`.

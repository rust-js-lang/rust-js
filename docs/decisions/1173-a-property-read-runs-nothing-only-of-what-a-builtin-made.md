# 1173. A property read runs nothing only of what a builtin made

Status: Accepted (2026-11-18, the user's decision: fix, not accept; #2106).
Amended 2026-11-18 (#2110): a variable is at hand only where every write
of it is. Amended 2026-11-18 (#2109): of the program's own structs, as
`fn_pointers.rs` says JS may make or give them, and of a type parameter's
value, as each caller's instantiation says.
Amended 2026-11-18 (#2111, #2113): a binding's method is its own only
of a receiver at hand, and a Rust field read is JS's property read.
Amended 2026-11-19 (#2118): lowering's `Known::RunsNothing` asks it too.
Amended 2026-11-19 (#2114, its first step): what a thread-local the crate
alone reaches holds, every write of it at hand, is at hand read back.
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

### A method's receiver, and a field read (#2111, #2113)

A `runs_nothing` or `runs_only_given` binding's method call, `this.m(..)`,
`this[k](..)` or `this.m?.(..)`, runs the binding's own method only of a
receiver no JS caller handed over: a JS caller's object may be a
subclass whose override, or a getter of the name, calls back into the
program (#2111, the user's decision: fix, not accept). So
`receiver_runs_its_own` (`runs_nothing.rs`), in `binding_call_runs_nothing`
and `call_runs_nothing`, takes the word only of a receiver `read_may_run`
says no JS caller may hand over, or one at hand; where the call's
arguments aren't known, by the method's declared `this` alone; a type
parameter's, by each caller's instance (`Called::Read`). Who asks says so
(`Receiver`): the borrow analysis, and lowering's `Known::RunsNothing`,
which finds the THIR call its MIR call is (`thir_call`) and asks of its
arguments (`CellBorrows::lowered_call_runs_nothing`, #2118): a copy of a
module's variable read past a JS caller's set's `has` stays its own, as
the override may set the variable. `may_run`'s walk takes the word
(`Receiver::Unasked`): its model has no binding let JS call the crate's
exports back, a premise for the user to decide (#2119). What isn't a method call, React DOM's
`writeChunk(destination, chunk)`, has no receiver to ask (ADR 0262's
difference 38, which stays, with what's left of this case: an own
property JS adds to an object a builtin made is patching it).

A Rust field read or write of a struct JS may make or give, `chunk.reason`
of a `js::cast`, is JS's own property access, which a getter or a setter
may answer (#2113): `field_access_may_run` (`runs_nothing.rs`), asked in
a body's summary (`direct`) and while a guard is held (`Quiet`), counts it
as `read_may_run` does a binding's read, unless what it's of is at hand.
A struct only the crate constructs stays plain data (difference 60). The
place a borrow is of, `form.action.borrow()`'s `form.action`, runs before
its guard is made, so `Quiet` doesn't walk a borrow's own arguments.

`AtHand` follows what's passed on (#2111): a cast of a value at hand,
`js::cast(Some(js::unknown(tag)))`, and `Some(x)`; a parameter of a
function only calls reach, `only_called`'s (ADR 0577), the one owner of
whose callers are all known, every call of which gives one (#2110's writes
rule holds of parameters too); a block's value; and what
`KEY.with(|value| ..)` gives of a thread-local whose initializer makes a
value at hand, which no code replaces, as what's at hand is never a
`Cell`'s, the crate's own, or a library's whose contract says so,
`ItemFact::AtHand` (library ABI 45), which its own `AtHand` derives. The
eight Intl formatters' `resolvedOptions()` create their result (ECMA-402's
`OrdinaryObjectCreate`), `CREATES` in `builtins/policy.ts`, which
`builtins/intl.ts` now writes through `attributesOf`, as `library.ts` does.

### What a thread-local holds (#2114)

A thread-local the crate alone reaches, `closed_keys`' (a private key used
only by `get`, `set`, `take`, `replace`, `with_borrow` and
`with_borrow_mut`, never its cell lent nor the key handed on), is a slot
(`at_hand/slots.rs`): what it holds is at hand where every write of it is.
Its writes are a closed list: its initializer, its cell's `new`'s value
(a `const { .. }` one's makes no JS object, a binding being no `const fn`);
`set` and `replace`; and in a `with_borrow_mut`'s closure, `*p = v`, an
`Option`'s `insert`, `get_or_insert`, `replace` and `get_or_insert_with` of
a closure at hand or a builtin's constructor, `js::weak_set::new`. Every
use of what the closure is lent must be one of those or a read, its `*p`,
`take`, `is_none`, `is_some`, `as_ref`: any other, the reference handed to
a function, captured by a closure, lent mutably, disqualifies the slot, as
an initializer not recognized does, so the rule fails closed. Read back,
`get`, `take`'s or `replace`'s old value, what a `with_borrow` or a
`with_borrow_mut` closure gives, and what either is lent, is at hand;
`None`, `js::null()`, and a `match` or an `if` whose every arm is, too.
React's Fizz hooks' `renderPhaseUpdates` and hot reloading's
`failedBoundaries` are back to their forms before #2111. A local `Rc` of a
cell its closures share, the noop renderer's, is the second step; a
struct's field, the lanes' `transitionLanes`, isn't a slot (#2108's).

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

- #2111 and #2113 move 11 corpus cases and no snapshot; React's port 12
  files and the vercel-ui SDK's week memo regain borrow checks, correct
  by the rule, much of it noise the follow-ups remove (P1s, filed as
  "output noise caused by a P0 fix"): a collection a builtin made kept in
  a `RefCell` and read back, `failedBoundaries ??= new WeakSet()` (cell
  contents' provenance); a struct only the program builds that its
  formatter's runtime hands back (`fn_pointers`' runtime giving); a
  `WeekInfo` returned through a private function (return values'
  provenance, `getWeekInfo`'s `creates`).
- Two port changes, correctness-forced (PORTING.md): Fizz hooks take the
  `$ACTION_KEY` out of its cell before `formData.append`, and the Flight
  server's `cleanupTaintQueue` takes its queue's values out before its
  loop, where upstream clears it after.

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
#2111's and #2113's rows: a JS caller's locale's method, plain and optional,
keeps the check, a subclass's re-entering override throwing; a locale the
memo made, through a private function, and optionally, reads plainly; a
private function also used as a value keeps the check; a thread-local map
`js::map::new` made reads plainly; a field read of a JS caller's value cast
to the program's struct keeps the check, in the body and in a helper, its
getter's re-entry throwing; of a struct the crate constructs, and of a
locale the memo made cast to one, plain; a borrow of a JS caller's struct's
cell field holds its guard plainly. Mutations `receiver-taken-at-its-word`,
`receiver-at-hand-unread`, `field-read-at-hand-unread`,
`at-hand-parameters-unread`, `at-hand-callers-unasked`,
`at-hand-thread-local-unread`, `at-hand-cast-not-passed-on`,
`field-access-not-summarized`, `quiet-field-unread` and
`quiet-own-place-walked`, caught; `optional-call-counted`,
`intl-constructors-run-unmarked`, `untagged-from-runs-code` and
`at-load-binding-runs-nothing-followed` re-anchored, caught.
#2118's row: a copy of a module's variable read past a JS caller's set's
`has`, whose override sets the variable, stays its own, run; past a set the
function made, it's the variable. Mutations `lowered-call-takes-the-word`
and `lowered-call-arguments-lost`, caught; `every-binding-call-runs-nothing`,
`binding-call-runs-nothing-unmarked` and `runs-nothing-runs-code` moved to
that row, caught. React's port: Fizz's `renderPhaseUpdates$1` copy returns,
its `Map` read back out of a `Cell` (#2114's noise, as the set of
lowering-shadows' `Cell` row is); the corpus, the snapshots and the
vercel-ui SDK don't move.
#2114's rows (test/at-hand-slots.test.ts): a private thread-local's set made
by `get_or_insert_with`, and a `Cell`'s map set where it's none, read
plainly under a memo's borrow; one whose cell is lent by `with`, one a JS
caller's set is written to (its override's re-entry throwing), and one whose
lent value is handed to a function keep the check. Mutations
`slot-writes-unread`, `slot-reads-unknown`, `slot-lent-value-unbound`,
`slot-unaccounted-use-admitted` and `slot-initializer-unread`, caught;
lowering-shadows' rows back to their forms before #2111.

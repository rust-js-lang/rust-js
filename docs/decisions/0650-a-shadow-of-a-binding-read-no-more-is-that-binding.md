# 0650. A `let` that shadows a binding read no more is that binding, assigned

Status: Accepted. Extends [0609](0609-a-shadowed-parameter-is-assigned.md),
whose parameter at a function's top is now any binding of the shadow's
block; 0609's pass in preparation is gone, this one is tidy's. Extended by
[0853](0853-a-variable-set-again-is-read-and-given-where-it-is.md): a
binding the variable it shadows is given last, the variable given it
where it's made.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: the same values are
read where they were; only names change.

## Context

React's `formatOwnerStack` reads an error's stack, then cuts it, as a
string slice in its port:

```rust
let stack = error.stack().unwrap_or_default();
js_error::set_prepare_stack_trace(prev_prepare_stack_trace);
let mut stack = stack.as_str();
if let Some(rest) = stack.strip_prefix("Error: react-stack-top-frame\n") {
    stack = rest;
}
if let Some(idx) = stack.find('\n') { .. }
let idx = stack.find("react_stack_bottom_frame").and_then(|idx| stack[..idx].rfind('\n'));
```

A Rust `let` of a name is a new binding, and JS can't declare one name
twice in a block, so rust-js named each apart:

```js
const stack = error.stack ?? '';
Error.prepareStackTrace = prevPrepareStackTrace;
let stack$1 = stack;
if (stack$1.startsWith('Error: react-stack-top-frame\n')) {
  stack$1 = stack$1.slice(29);
}
const idx = stack$1.indexOf('\n');
..
const idx$1 = at !== -1 ? stack$1.slice(0, at).lastIndexOf('\n') : -1;
```

where React has one `let stack` and one `let idx`, each set again. ADR 0609
assigned a parameter its shadow's value, but only a parameter, at its
function's top; ADR 0277 reads a shadow that's the old one's own value, a
`const n$1 = n;` nothing changes after, as the old one.

## Decision

**A `let` that shadows a binding of its own block, or a parameter of its
body or of the `for` whose block it is, both named by the program, where
nothing reads the old one after it, is that binding, assigned**: `stack =
e;`, read by its name after, the binding's `const` a `let` now, a `for`'s
pattern `let` once it's set; of the old one's own value, `let mut stack =
stack.as_str();`, nothing at all.

```js
let stack = error.stack ?? "";
Error.prepareStackTrace = prevPrepareStackTrace;
if (stack.startsWith("Error: react-stack-top-frame\n")) {
  stack = stack.slice(29);
}
let idx = stack.indexOf("\n");
..
idx = at !== -1 ? stack.slice(0, at).lastIndexOf("\n") : -1;
```

The old one is read no more where:

- **No read of it follows in its block**: a drop at the block's end, `try
  { .. } finally { noisyDrop_drop(g); }`, a shared borrow read after, a
  read in a later branch, each keeps the new one apart.
- **Nothing that runs later names it**: a closure, a function the body
  declares, or a `&mut`'s handle on it, `{ get value() { return t; }, set
  value(v) { t = v; } }`, may run after the shadow, wherever it's written,
  and would see the new value. Any of them anywhere in the body keeps the
  new one apart.
- **Only its own block**: a loop's body is a block of its own, run again
  each turn, so a `let` there never takes a binding from outside it, which
  the next turn reads anew: `let x = x + i;` in a loop is `const x$1`. A
  branch's block is too, whose `const` of the name ADR 0352 gives where each
  read keeps its own.
- **Both named by the program** (`Locals::user`): a value rust-js holds,
  named after its type or the call that made it, `value`, `tmp`, is no
  binding of the program's; one reused for the next, `let tmp = ([a, b]) =>
  ..; tmp = tmp(next);`, reads worse than a name of its own. So the pass
  is lowering's, which knows which locals the program names, after a body
  is tidied and its temporaries named (`tidy::shadowed`).
- **Not `let x$1;`**, which is `undefined` till it's set, where `x` isn't.
- **Its type may differ**: a JS variable has none, so `text` of `let text
  = format!(..); let text: u32 = text.parse().unwrap();` is a string, then
  a number.

## Why

- **No program can tell**: after the shadow nothing reads the old binding,
  by its name or by what runs later, so one variable holds what the two
  held, each value where it's read.
- **It's the JS React writes**, `let stack` set again, and what a person
  writes of `let s = s.trim();`.
- **One owner**: whether a shadow is the binding it shadows is
  `tidy::shadowed`'s. 0609's `shadowed_params`, in preparation, went: it
  asked the same of a parameter, and named a temporary its parameter's
  shadow, which this doesn't.

## Consequences

- The readability gate's `renamed x$1` is 7,171 lines where it was 7,626,
  800.5 in 10,000 where it was 851.2; 64 corpus cases and 6 snapshots read
  so, each run as native Rust is.
- Two temporaries named after a parameter, `iter` and `other`, which 0609
  assigned to it, are named apart again, `const iter$1 =
  $cycleOf(iter, ..)`: they're no binding of the program's.
- The `lowering-names` test's `early` is `n = 2; return n * 3;`, and the
  `lowering-string-places` test's `formatOwnerStack` sets its `idx`.

## What holds it

- `test/lowering-shadows.test.ts`: React's `formatOwnerStack`, its JS and
  its run in Node; and a shadow in a loop, after a closure, a `&mut`'s place
  and its handle, before a drop, of another type, of a `for`'s pattern and
  a chain, each run.
- `test/lowering-names.test.ts`, 0609's parameter.
- Mutations in `scripts/mutations/lower/mir/tidy/bindings.ts`: the shadow
  kept apart, its own value assigned, its binding left `const` and a `for`'s
  pattern too, the binding read after, captured or set through a handle
  assigned, a loop's shadow taking a parameter, and a temporary reused.

## Amendment: a `let x$1;` its `switch`'s cases set

ADR 0609 was made for React's scheduler, whose port shadows its
parameter, `let priority_level = match priority_level { IMMEDIATE_PRIORITY
| .. | IDLE_PRIORITY => priority_level, _ => NORMAL_PRIORITY };`, and
printed upstream's assignment, `priorityLevel = .. ? priorityLevel :
NormalPriority`. Since `e4be2ab50` (ADR 0629: a `switch` is of three cases,
not three arms) that `match` is a `switch`, whose value is a `let` its cases
set, and this decision took no `let x$1;`, "`undefined` till it's set,
where `x` isn't":

```js
let priorityLevel$1;
switch (priorityLevel) {
  case ImmediatePriority:
  ..
  case IdlePriority:
    priorityLevel$1 = priorityLevel;
    break;
  default:
    priorityLevel$1 = NormalPriority;
}
const restore = currentPriorityLevel;
currentPriorityLevel = priorityLevel$1;
```

**A `let x$1;` is the binding it shadows too where, each way through what
follows it, nothing names that binding once the shadow may be set**
(`set_after_last_read`): its last read before the shadow's first write, as
a case's `x$1 = x` reads `x`, then sets it; a case is entered from the test
or from the case before that falls through to it (ADR 0905), and a branch
that leaves goes on nowhere after it. A loop or a `try` that sets the
shadow takes it only where nothing after names the binding. Each case of
the shadow's own value, `x$1 = x`, is then nothing, its case's `break;`
alone; a branch of an `if` chain of it, a range's `match`, keeps the
shadow apart, as before, where it would leave `if (a) {} else ..`. What reads `x$1` read the shadow's value, which Rust has it hold by
then, and reads it in `x`; nothing reads `x`'s after. So the scheduler is
upstream's `switch` again:

```js
switch (priorityLevel) {
  case ImmediatePriority:
  ..
  case IdlePriority:
    break;
  default:
    priorityLevel = NormalPriority;
}
```

Case A, as before: the same values are read where they were. A shadow
whose old binding a closure made before reads, or a drop after it, stays
apart, `let noisy$1;`.

- `test/corpus/shadow_set_in_cases.rs`, against native Rust: the
  scheduler's `match`, a guarded case that falls through, a case that
  declares a binding of the name, an `if` chain of a range's, a shadow set
  again in a loop after, one a closure made before reads the old value of,
  and one whose old value is dropped after; `test/port-scheduler.test.ts`'s
  snapshot. Six corpus cases' `let x$1;` a `try` sets, and std's
  `FlattenCompat::size_hint`'s, are their bindings now.
- Mutations `shadow-set-in-cases-kept-apart`, `-own-value-kept`,
  `-read-after`, `-fallen-unset`, `-left-branch-goes-on`,
  `-unnamed-rest-followed`, `shadow-set-in-branches-own-value-emptied`.

## Amendment: an `if` branch that does more than give the binding is set in place (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same values are
read where they were: the branch's own value, `x$1 = x`, set nothing new.

React's createWorkInProgress, `let workInProgress = current.alternate; if
(workInProgress === null) { workInProgress = createFiber(..); .. } else {
workInProgress.flags = NoFlags; .. }` (r06 of the port's gaps). Its port
shadows, `let work_in_progress = current.alternate.borrow().clone(); let
work_in_progress = match work_in_progress { None => { .. }, Some(work_in_progress)
=> { work_in_progress.flags.set(NO_FLAGS); work_in_progress } };`, an
`if` whose `else` ends `workInProgress$1 = workInProgress;`. A shadow set
to the binding's own value was the binding only in a `switch`'s cases,
lest an `if`'s branch be left empty, `if (a) {} else ..`; so the JS was
`const workInProgress = ..; let workInProgress$1; if (..) { .. } else {
..; workInProgress$1 = workInProgress; }`.

**An own value in a list of statements that does more is the binding
too** (`shadows::shadow_of`), its `x$1 = x` gone: `let workInProgress =
current.alternate; if (workInProgress == null) { workInProgress = {..};
current.alternate = workInProgress; } else { workInProgress.flags = 0; }`,
as upstream. A branch that is the own value alone stays apart, as before.
A port names the result as the variable it takes apart, shadowing it, as
upstream assigns one variable; with another name, `let alternate = ..;
let work_in_progress = match alternate { .. }`, both names stay, as
written.

Held by `test/corpus/shadow_set_in_cases.rs`'s `work_in_progress`, both
arms, against native Rust; the mutations
`shadow-set-in-a-branch-that-does-more-apart` and
`shadow-set-in-a-branch-of-it-alone` (`mir/tidy/shadows.ts`).

## Amendment: a branch that's the binding's own value, of a test that it's there (2026-10-28, React port gap rd65)

ReactDOMComponentTree's addEventHandleToTarget takes a value from where
it's kept, or makes and keeps one where there's none: `let handles =
match handles { Some(handles) => handles, None => { let handles =
Set::new(); js::set(target, key, handles); handles } };`. The shadow
stayed apart, `let handles$1; if (handles != null) { handles$1 =
handles; } else { handles$1 = new Set(); .. }`, as a branch that's the
binding's own value alone would be left empty. **Where that branch's
test is that the binding is there, `handles != null`, and its other
branch does more, the shadow is the binding, and the `if` its other
way**: `let handles = target[key]; if (handles == null) { handles = new
Set(); target[key] = handles; }`, as upstream writes it. Any other test,
a range's `if` chain, keeps the shadow apart, as before. Held by
`test/corpus/shadow_made_where_none.rs` against native Rust and the
mutations `shadow-of-an-if-branch-kept` and
`shadow-emptied-if-unflipped` (`lower/mir/tidy/shadows.ts`).

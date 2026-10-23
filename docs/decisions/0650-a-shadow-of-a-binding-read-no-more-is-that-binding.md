# 0650. A `let` that shadows a binding read no more is that binding, assigned

Status: Accepted. Extends [0609](0609-a-shadowed-parameter-is-assigned.md),
whose parameter at a function's top is now any binding of the shadow's
block; 0609's pass in preparation is gone, this one is tidy's.

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

- The readability gate's `renamed x$1` is 7,125 lines where it was 7,580,
  797.5 in 10,000 where it was 848.4; 64 corpus cases and 6 snapshots read
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

# 0262. When Rust and JS disagree, what the program can observe decides

Status: Accepted. Spells out the rule at the top of the [design docs](../README.md).

## Context

The design docs open with one rule, borrowed from ReScript:

> Where the language and JS disagree, pick the JS behavior that keeps the
> output small and readable, and write that choice down.

Taken alone, it would allow any disagreement for smaller output. The
decisions since haven't read it that way: each asked whether the program
could tell the difference, and some kept Rust's meaning at a cost in output,
`>>> 0` after a `u32`'s sum, `$byteLen(s)` for a `len()`. That question was
never written down, so each decision asked it anew.

## Decision

**What the program can observe decides, in four cases:**

```
Can the program observe the difference?
│
├─ No ─────────────────────────────► A: JS's, and the ADR says why
│
├─ Yes, but only where it's rare ──► B: JS's, and the ADR's Consequences
│  and does no harm                     say where
│
├─ Yes, and results would change ──► C: Rust's, paid for in output
│
└─ Yes, and Rust's can't be kept ──► D: a compile error, never silent drift

No disagreement at all, tooling, bindings' shape, the output's layout: N
```

- **A, not observable: JS's.** JS runs a module on one thread, so a
  `thread_local!` is a module's `const` (ADR 0037). A fresh object of
  constants is the same made before or after what follows (ADR 0261).
- **B, observable where it's rare and harmless: JS's, written down.** A
  thread-local is made when its module loads, not at first use: only an
  initializer with effects can tell (ADR 0037's Consequences).
- **C, results would change: Rust's.** A `u32`'s arithmetic wraps, `x >>> 0`
  (ADR 0011). A string's `len()` counts UTF-8 bytes, `$byteLen(s)`, where
  JS's `length` would agree for ASCII and quietly disagree otherwise
  (ADR 0138).
- **D, Rust's can't be kept: an error.** `rust-js does not support .. yet`,
  and no output (ADR 0006). A later decision may move a case up, as ADR
  0138 did string byte counts, an error under ADR 0034. Where only running
  can tell, as an init that uses its own `LazyLock`, which deadlocks in
  Rust (ADR 0318), the runtime throws `rust-js does not support ..` there:
  never a panic that looks like std's but isn't what Rust does.

The program's meaning is rustc's: rust-js never makes Rust that's wrong
work in JS. Two closures, `Box::new(move |_| close())` given to
`addEventListener` and again to `removeEventListener`, are two functions,
as in Rust, though printing both as `close` would remove the listener.
react.dev's TopNav passes the same closure instead.

## Why

- **Every decision since has asked this**; written down, the next one
  starts from it.
- **It keeps both halves of the north star**: JS that looks hand-written
  where Rust can't tell, and Rust's answers where it can.

## Alternatives

- **The rule alone**: smaller output wherever JS and Rust disagree. A
  program's answers would change silently, `"héllo".len()` 5, not 6.
- **Rust's meaning everywhere**: every thread-local lazy, every object made
  in Rust's order. The output would be std's machinery, not the JS a person
  writes.

## Consequences

- **Each decision says which case it's in**, on a `Case:` line under its
  status, every case its parts are in, the first the main one; N where it
  decides no disagreement, tooling, bindings' shape or the output's
  layout. An unobservable difference says why the program can't tell; a
  rare one lists where it can, under Consequences.

## Amendment: the stack's depth

A recursion deeper than the JS engine's stack, about 10,000 calls in
Node and browsers against Rust's 8 MB main thread, throws `RangeError`
where Rust's returns (#155). Results change and Rust's can't be kept:
no translation of a direct call gives JS a deeper stack, and how deep a
call goes is its input's, so no compile error can say where. It's the
fourth case's, as near as it gets: listed among the guide's differences
(`semantics.md`, 16), with what to write instead, an explicit stack.

## Amendment: the crate's iterator's own `fold` and `nth`

std routes `sum`, `count`, `for_each` and `last` through an iterator's
`fold`, and `skip` and `step_by` through its `nth`, so an impl that
writes its own runs it; rust-js steps the crate's iterator by its `next`
alone (#162). Results change only where the impl's own `fold` or `nth`
does something its `next` doesn't, a side effect or another answer, which
std's contract doesn't forbid but no iterator is written for: listed
among the guide's differences (`semantics.md`, 18), the issue open for
whether it's kept.

## Amendment: a string's `repeat` past the engine's length

A `str`'s `repeat(n)` is JS's own, unchecked
([0774](0774-room-past-isize-max-bytes-panics.md), amended), as the user
decided: a repeat past `isize::MAX` bytes, where std panics `capacity
overflow`, throws the engine's `RangeError`, in V8 before any string std
would refuse is made; in JavaScriptCore, whose strings are longer, a
string of text not all ASCII past a billion units may be made where std
panics. Results change only for a program that asks a string that long,
which runs out either way but in that corner: the second case, listed
among the guide's differences (`semantics.md`, 24).

## Amendment: a thread-local of std's or a binding's calls, made as its module loads

Rust makes a thread-local at its first use; rust-js makes one whose
initializer runs the crate's code there too (ADR 0037, amended). One whose
initializer calls only std's and bindings' functions is made as its module
loads, as JS writes it, React's `export const TocContext =
createContext([])`, which the user chose (2026-10-25) over making each
lazy, `TocContext.value` read through a holder. Results change only where
such a call does something a program sees before the thread-local's first
use, or a std call in it panics: the second case's, listed among the
guide's differences (`semantics.md`, 25).

## Amendment: room std's growth asks, and a collect of smaller items

Room for a length that's asked is checked as std checks it, `resize`'s,
a range's `collect()`'s and a hash table's (#190,
[0774](0774-room-past-isize-max-bytes-panics.md), amended). Room std's
own growth asks isn't: a `push`, `extend` or `insert` past the capacity
its doubling picks, which rust-js doesn't model; nor a `collect()` whose
length is a collection's of smaller items, `ids.iter().map(|x|
x.to_string())`, which the user chose (2026-10-25) over a `$roomFor`
before each such `collect()`. Results change only past `isize::MAX`
bytes, of hundreds of millions of items, or of items each a mebibyte
made of little: the second case, listed among the guide's differences
(`semantics.md`, 26), with a count collected through another adapter
(#246) until it's checked.


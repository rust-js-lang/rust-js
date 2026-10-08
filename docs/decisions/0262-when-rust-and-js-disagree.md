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
├─ No ─────────────────────────────► JS's, and the ADR says why
│
├─ Yes, but only where it's rare ──► JS's, and the ADR's Consequences
│  and does no harm                  say where
│
├─ Yes, and results would change ──► Rust's, paid for in output
│
└─ Yes, and Rust's can't be kept ──► a compile error, never silent drift
```

- **Not observable: JS's.** JS runs a module on one thread, so a
  `thread_local!` is a module's `const` (ADR 0037). A fresh object of
  constants is the same made before or after what follows (ADR 0261).
- **Observable where it's rare and harmless: JS's, written down.** A
  thread-local is made when its module loads, not at first use: only an
  initializer with effects can tell (ADR 0037's Consequences).
- **Results would change: Rust's.** A `u32`'s arithmetic wraps, `x >>> 0`
  (ADR 0011). A string's `len()` counts UTF-8 bytes, `$byteLen(s)`, where
  JS's `length` would agree for ASCII and quietly disagree otherwise
  (ADR 0138).
- **Rust's can't be kept: an error.** `rust-js does not support .. yet`,
  and no output (ADR 0006). A later decision may move a case up, as ADR
  0138 did string byte counts, an error under ADR 0034.

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

- **Each decision says which case it's in**: an unobservable difference
  says why the program can't tell; a rare one lists where it can, under
  Consequences.

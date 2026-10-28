# 0996. `eq()` of `TrustedLen` iterators rust-js can't count is refused where it shows

Status: Accepted. Narrows #386 after
[0992](0992-eq-of-trusted-len-iterators-asks-their-lengths.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't be
preserved here, so compile error.

## Context

ADR 0992 gives `$iterEq` both sides' lengths where it can count them
before stepping. A map's value is often held in a MIR temporary, a JS
helper's `$iter(v).map(f)` that knows nothing of its length:

```rust
v.iter().map(|x| { println!("a {x}"); *x }).eq(w.iter().map(|x| { println!("b {x}"); *x }))
```

Of unequal lengths, Rust prints nothing; the JS ran both maps until one
ended.

## Decision

Where both sides are `TrustedLen` but either can't be counted, the call
is refused if what stepping runs can be seen:
- either type holds a function, a closure or a `fn`, a map's for one;
- or either side's items have a destructor.

Others, an array's items or a range's, step nothing that shows, and
compile as before.

## Consequences

The diagnostics test "eq of two maps of unequal lengths" holds it, with
the mutation `uncounted-eq-compared`.

Generic code's `eq()` still compares in pairs: whether `I` is `TrustedLen`
only its caller knows, and a JS value given for it can't give its length.
#386 stays open for that.

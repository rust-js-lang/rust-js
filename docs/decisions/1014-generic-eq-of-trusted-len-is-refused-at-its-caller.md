# 1014. Generic code's `eq` of `TrustedLen` iterators is refused at its caller

Status: Accepted. Extends [0998](0998-eq-of-uncounted-trusted-len-iterators-is-refused.md).
Part of #386.

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't be
preserved here, so it's a compile error. It was a wrong answer.

## Context

```rust
fn same<I: Iterator<Item = u32>>(a: I, b: I) -> bool { a.eq(b) }
let f = |x: &u32| { println!("a {x}"); *x };
same(vec![1, 2, 3].iter().map(f), vec![1, 2].iter().map(f))
```

std's `eq` of two `TrustedLen` iterators, `SpecIterEq`, compares their
lengths first, and steps neither of unequal ones: Rust prints `false`
alone. The generic function's JS is one for every caller and compares item
by item, as the values it's given can't report their lengths (ADR 0170).
So it printed `a 1`, `a 1`, `a 2`, `a 2`, `a 3` first. ADR 0998 refuses
such an `eq` where it's written of concrete types. Inside generic code
`I` isn't known to be `TrustedLen`, so that rule never applied there.

## Decision

An analysis, `generic_eq`, records what each body compares by `eq` or
`ne` of types holding its type parameters. It carries them to each
caller's types through generic callers, as ADR 1001 carries what's
stepped from the back. A call that makes a pair ADR 0998 would refuse
is refused at the call: both `TrustedLen`, and stepping them runs code,
a closure or a function they hold, or items with a destructor. Their
lengths are never counted before stepping in generic code.

Making it correct, the generic code given its callers' lengths, needs
iterators that carry their length, the representation change ADR 1000
decided against for now.

## Consequences

- A pair whose stepping runs nothing, `v.iter().copied()`'s, still
  compiles: comparing it item by item is unseen.
- `test/diagnostics.test.ts`' "eq in generic code of two maps" checks the
  refusal through a generic caller. The guide's limitations list it.
- The mutation is `generic-trusted-eq-allowed`.

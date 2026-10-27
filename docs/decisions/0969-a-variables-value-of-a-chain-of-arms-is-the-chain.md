# 0969. A variable's value of a chain of arms is the chain, every arm its own

Status: Accepted. Records and extends the rule fd6e76df1 made (MIR keeps a
variable's value of three arms a chain) to chains of any length.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: an `if` chain and a conditional give the same value, each test
evaluated in the same order.

## Context

`let x = if a { p } else if b { q } else { r };`, from MIR, is three
branches each assigning `x`. Tidy folds two branches that only give a
value into a conditional, bottom-up, so a chain had its last two folded,
`else { x = b ? q : r; }`, half a chain and half a conditional; fd6e76df1
restored the chain of three, `else if (b) { x = q; } else { x = r; }`, as
a person writes one of more branches.

React's ReactFiberPerformanceTrack picks colors by chains of four arms:

```rust
let color = if yield_duration < 5.0 {
    "primary-light"
} else if yield_duration < 10.0 {
    "primary"
} else if yield_duration < 100.0 {
    "primary-dark"
} else {
    "error"
};
```

It printed `if (yieldDuration < 5) { color = "primary-light"; } else {
color = yieldDuration < 10 ? "primary" : yieldDuration < 100 ? .. : ..;
}`: the restoration took only a conditional of two values, MIR gave the
inner arms through temporaries, `let _7; _7 = ..; color = _7;`, and an
arm MIR wrote the other way round came out negated, `!(selfTime < 100) ?
"error" : ..`.

## Decision

**A variable's value of a chain of arms is the chain, every arm its own**
(`tidy/branches.rs`'s `chained_arms`): the rest folded, `x = b ? q : c ? r
: s`, is `else if (b) { x = q; } else if (c) { x = r; } else { x = s; }`,
the chain going on in each conditional's second value; one of a negated
test, MIR's arms the other way round, `!t ? e : r`, is `if (t) { x = r; }
else { x = e; }`; and one MIR gives through a temporary declared and read
there, `let _7; _7 = v; x = _7;` (`assigned_through`), is `x = v`'s. An
arm's value is a value alone or the conditional of an `if` in that arm,
`wasHydrated ? "tertiary" : "primary"`.

## Alternatives

- **Every arm a conditional, `const color = a ? .. : b ? .. : ..`**, as
  upstream React writes these. The rule for three arms chose the chain;
  which form a chain of values takes is written up for the user.
- **The half-folded form, as before**: two forms in one value.

## Consequences

- Held by `test/lowering-written-branches.test.ts`'s "a variable's value of
  four arms from MIR is their chain" and `test/lowering-branches.test.ts`'s
  three arms; mutations `chain-through-temporary-unseen`,
  `chain-negated-arm-kept`, `chain-rest-one-arm` and `mir-chain-tail-folded`.

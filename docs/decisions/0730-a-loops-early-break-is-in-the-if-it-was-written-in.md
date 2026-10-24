# 0730. A loop's early `break` is in the `if` it was written in

Status: Accepted. Amends [0684](0684-a-continue-the-program-wrote-is-where-it-wrote-it.md):
a loop body's last `if` is an early `continue` only where the program
wrote one.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
which `if` a jump is in is how the JS reads, not what it does.

## Context

React's `addObjectDiffToProperties` (shared/ReactPerformanceTrackProperties.js)
stops at a width limit:

```rust
for (checked, key) in keys.iter().enumerate() {
    if checked > LIMIT {
        log.push("too many".to_string());
        equal = false;
        break;
    }

    if !next.contains(key) {
        log.push(key.clone());
        equal = false;
    }
}
```

MIR has the rest of the body as the `if`'s `else`, going round, and the
`break` after it, so the JS read:

```js
for (const [checked, key] of keys.entries()) {
  if (checked > LIMIT) {
    log.push("too many");
    equal = false;
  } else {
    if (!next.includes(key)) { .. }
    continue;
  }
  break;
}
```

`early_breaks` turns `if (c) { a; continue; } break;` into `if (!c) {
break; } a`, a `while let`'s test, but not this shape, whose `then`
breaks and whose `else` goes on.

Once the rest was the body's own, a guess in `continued` made it worse:
a body's last `if (!c) { a; b }`, its test a `!`, was read as MIR's of a
`continue` the program wrote, `if (c) { continue; } a; b`. The program's
own `if !next.contains(key)` became `if (next.includes(key)) { continue;
}`. The guess predates the hints that say where a program writes a
`continue` (ADR 0684), which cover every case it was for.

## Decision

**A loop body that ends `if (c) { a } else { b; continue; } break;`, `a`
going on to the `break`, is `if (c) { a; break; } b`, as the program
wrote it**, where `b` declares nothing beside the body's own (`early_breaks`,
`broken_in_then`).

**A loop body's last `if` is an early `continue` only where the program
wrote one**, by `Written::continues`; the guess by a `!` test is gone.

## Consequences

- The port's two width-limit loops read as upstream's.
- `drop_scopes`'s loop whose `else` dropped and went round now drops after
  the `if` that breaks.
- Nothing else in the corpus needed the guess.

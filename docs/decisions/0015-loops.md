# 0015. Loops: put `while` back, label only when needed

Status: Accepted

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

THIR has one loop construct, `Loop`. Rust's `while` is desugared before we see it:

```
while c { body }   ══►   loop { if c { body } else { break } }
```

A loop is also an expression: `loop { .. break v; }` produces `v`. And
`break`/`continue` can target outer loops by label (`'outer`).

## Decision

1. **Re-sugar `while`.** `as_while()` recognizes exactly that desugared
   shape (a loop whose body is only `if c { .. } else { break }`, with that
   `break` targeting this loop and `c` simple) and prints `while (c) { .. }`.
   Every other loop is `while (true) { .. }`.
2. **Break values use the loop's destination.** Each loop remembers the
   `dest` it was lowered with ([0008](0008-expression-and-statement-modes.md)).
   `break v` delivers `v` there, then breaks. If that destination is
   `Return`, the delivery *is* a `return`, and the `break` is skipped:

   ```rust
   loop { if i == n { break a; } ... }      // loop value is returned
   ```
   ```js
   while (true) { if (i === n) { return a; } ... }
   ```
3. **Labels only when needed.** A JS label is added only when a
   `break`/`continue` jumps past the innermost loop. Its name comes from the
   Rust label (`'outer` → `outer`), or `loop` if unlabeled, made unique per
   function.
4. `continue` maps to `continue`. In a re-sugared `while`, JS re-checks the
   condition, just like Rust.

## Why

- `while (i < n)` is what a human writes, and what the Rust source said.
- Passing `dest` into breaks avoids the temporary-plus-break-plus-return
  dance for the most common "search loop" pattern.
- Unused labels are noise. Labels are created lazily, at first use, so they
  never appear otherwise.

## Alternatives

- **Always `while (true)` with `if (!c) break;`**: correct, less readable.
- **`for (;;)`**: same meaning as `while (true)`. Chose `while (true)` for plainness.

## Consequences

- `as_while` depends on the exact desugaring shape in this rustc version. If
  rustc changes it, loops still compile correctly as `while (true)`, just
  less prettily. That's a safe failure.
- Labeled *blocks* (`'a: { .. break 'a v; }`) aren't supported yet. JS has
  labeled blocks, so this is straightforward future work.
- `for` loops need iterators, which need traits and methods. That's future work.

## Consequence: an accepted port difference, fo1 (2026-10-28)

React's commitMutationEffectsOnFiber walks `for (let ii = 0; ii <
eventPayloads.length; ii++) { const {ref, nextImpl} = eventPayloads[ii];
.. }`; its port's `for EventFunctionPayload { r#ref, next_impl } in
event_payloads` is `for (const { ref, nextImpl } of eventPayloads)`. The
user accepted the difference by name (fo1): an index loop is clippy's
`needless_range_loop`, and AGENTS.md keeps iterators as written. ADR
0262 case: None.

## Amendment: a label the Rust writes is kept, one MIR makes never is (D1, the user's decision, 2026-11-04, #523)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a label's name. The
standing rule is that a JS label is never an accepted difference. The user
decided (D1) **a narrow exception: a label the Rust program writes is
kept**, `'outer:` as `outer:`, as React DOM writes `outer: while (true)`
(packages/react-dom-bindings/src/client/ReactDOMSelection.js:75). **A label
the compiler makes, `bb5:`, is still never accepted.**

`'outer: loop { for (r, row) in .. { for .. { if .. { break 'outer; } } }
break; }` runs its `loop` once, so MIR has no loop of it: the `for` the
`loop` holds is labeled by MIR's block, `bb5: for (..)`, each `break bb5`
a `break 'outer`, as leaving the `for` ends the `loop`. Now that label is
the program's, `outer: for (..)` (`jumps::written_loop_breaks`, tidy's
last pass): a loop MIR labeled whose own loop the program didn't label,
inside the innermost loop the program labeled (`Written::loops`), where
no label in the body has the name. A loop the program labeled that's a JS
loop has it already (`written labels`), so the one left is no JS loop.

`test/readability.test.ts`, "a label in the generated JS is one its Rust
wrote", holds the rule: no committed JS but those it lists, each with its
issue, has a `bbN:` label, a list that only shrinks. Corpus
`loop_exit_moves.rs`'s `grid_find`, against native Rust; mutation
`written-loop-label-unnamed` (`mir/tidy/jumps/labels.ts`).


# 0879. An `if` reads as written, and a closure's captures are read in place

Status: Accepted. Amends [0237](0237-no-else-after-leaving.md)'s scope (an
`else` that leaves a loop the program wrote stays),
[0364](0364-mir-lowering.md)'s fold of a branch of `false` (an `if`'s value
is a conditional), [0607](0607-a-field-is-given-a-call-in-place.md) (a
variable a closure captured is given a call's value in place) and extends
[0846](0846-a-parameters-clone-is-read-by-its-name.md) (a clone of what a
`move` closure captured is read by its name).

Case ([0262](0262-when-rust-and-js-disagree.md)): N for each, which runs
the same; each is a form the program wrote, which MIR's structure lost.

## Context

React's `act` (`packages/react/src/ReactAct.rs`, react-in-rust) printed
five differences from upstream's `ReactAct.js`:

| Rust | Was | Upstream |
|---|---|---|
| `if !queue.borrow().is_empty() { flush.. } else { act_queue = None }` | `if (queue.length === 0) { actQueue = null; } else { flush.. }` | `if (queue.length !== 0) { flush.. } else { .. = null; }` |
| `if let Some(c) = continuation { .. } else { break }` | `if (continuation == null) { break; } ..` | `if (continuation !== null) { .. } else { break; }` |
| `if !DISABLE_LEGACY_MODE { x } else { false }` | `!disableLegacyMode && x` | `!disableLegacyMode ? x : false` |
| `result = callback();` in `catch_unwind`'s closure | `const value = callback(); result = value;` | `result = callback();` |
| `let prev = prev_act_queue.clone(); let queue = Rc::clone(&queue);` in a `move` closure | `const prevActQueue$1 = prevActQueue; const queue$1 = queue;` | `prevActQueue`, `queue` |

- **rustc turns `if !c`'s branches**: MIR building tests `c` and swaps the
  targets, so `switchInt(c)` goes to the `else` first.
- **A loop's early exit was any lone exit**: `early_return` made `if (c) { a
  } else { break; }` the guard `if (!c) { break; } a` whoever wrote it.
- **`conditional` folded any branch of `true` or `false`** into `&&` or
  `||`, the program's `&&` and its `if` alike, MIR making the same branches
  of each.
- **A place through a `Deref` kept its temporary** (0607), the JS place
  `a[i]` reached before the call; a closure's capture is `*(_1.0)`, whose
  JS is the variable itself.
- **A clone was read by its name only of a parameter** (0846).

## Decision

**An `if` the program wrote with an `else` (`Written::elses`) reads as it
wrote it:**

- **Its branches in their order**: `if (c) { b } else { a }` whose `a` is
  written in its `then` and `b` in its `else` is `if (!c) { a } else { b }`
  (`written_elses`).
- **Its `else` that leaves a loop stays an `else`**: `early_return` takes no
  `if (c) { a } else { e }` whose `a` is the `then` written and whose `e`
  none of it (`written_else`); not a `while`'s, whose desugared `if c { body
  } else { break }` has the whole loop for its `else`'s span, which stays
  the loop's test. 0237 still leaves out an `else` after a `then` that
  leaves; a `then` that goes on, with an `else` that leaves, is both
  branches.
- **Its value is a conditional**: `if c { x } else { false }` is `c ? x :
  false`, `if c { true } else { x }` `c ? true : x`; a value the program
  wrote with `&&` or `||` stays one, and `if c { true } else { false }` is
  `c`, as before (`written_branches`, `conditional`'s `written_if`).

**A variable a closure captured is given a call's value in place**, `result
= callback();`: a place JS reaches as a variable, with nothing to reach
first, is set once the value is made, as Rust sets it. One JS reaches
through an index keeps its temporary (0607).

**A clone of what a closure captured by value is read by its name**, as a
parameter's is (0846): the capture is a variable its maker never assigns
again from where it makes the closure (`mir_closure_fn` copies one it does,
`const captured = a`), so the clone, made in a call, is that value wherever
it's read, its closures' too. Not of one it borrowed, which its maker may
give another value once the borrow ends: `let b = a.clone(); move || *b` of
a borrowed `a` stays `const b = a`.

**`while c { ..; i += 1 }` stays a `while`** (not changed): the program
wrote a `while`, and `for i in a..b` is the `for`. A `for (; c; i++)` would
need a `for` of any test and step (the JS tree's `For` is a range's), and
would hold only where the body has no `continue`, which runs the step
before the test: two forms of one construct, picked by what the body holds.
Its `i = (i + 1) >>> 0` is a `usize`'s wrap where rust-js can't bound `i`,
`i < q.borrow().len()`, which `i < v.len()` bounds to `i++`.

## Why

- **It's the JS the program says**, and upstream's: `ReactAct.js` reads
  `queue.length !== 0` first, `else { break; }`, `!disableLegacyMode ? x :
  false`, `result = callback();` and `prevActQueue` in its callbacks.
- **It runs the same**: a test negated with its branches turned takes the
  branch it took; an `else` kept is the code that ran after the guard; `c ?
  x : false` of a `bool` is `c && x`; JS sets a variable after it makes the
  value; a clone that is the same JS value, of a variable nothing gives
  another, is that variable.
- **It's tested**: `test/corpus/branches_as_written.rs`, with native Rust,
  holds each, with a capture borrowed and given another value after its
  clone, and an index a call changes; `interpreter.rs`'s `if peek ==
  Comma { next } else { break }` and `generic_mut_ref_kept.rs`'s `if let
  .. else { return }` read as written.

## Alternatives

- **`if c { x } else { false }` as `c && x`**, 0364's: shorter, but two
  forms of one `if`, picked by a literal in a branch. Rejected.
- **Every clone of a `const` read by its name**: a `const` the function
  declares is fixed from where it's made too, but the program named the
  clone, `let gas = n;`'s choice of a second name (0846); left as it is.

## Consequences

- The guide's `is_adult`, `if let Some(age) = p.age { age >= 18 } else {
  false }`, is `p.age != null ? p.age >= 18 : false`; `p.age.is_some_and(|a|
  a >= 18)` is the `&&`.
- The playground's `ResultFrame` reads `onOutcome` in its handlers, where it
  read `told` and `silent`.
- `docs/readability.txt`'s `while (true)` share grows by one loop, 10.2 to
  10.3 in 10,000 lines: `branches_as_written.rs` writes a `loop` to hold
  its `if let .. else { break }`, and a `loop` is a `while (true)`.

## What holds it

- `test/corpus/branches_as_written.rs`, against native Rust;
  `test/lowering-branches.test.ts` ("branches from MIR read as a person
  writes them") and `test/lowering-written-branches.test.ts` ("a value's
  truth or else a bool is a bool part by part", whose `&&`, `||` and
  `match` keep the folds 0364 makes of what the program didn't write as
  an `if`).
- Mutations: `written-else-branches-as-mir-turned`,
  `written-else-branches-turned-untested`, `written-if-value-of-mir-branches`,
  `written-else-of-a-while-too` (`tidy/jumps/tails.ts`),
  `written-else-made-an-early-exit` (`tidy/jumps.ts`),
  `written-if-value-folded`, `written-chain-value-folded`
  (`tidy/branches.ts`),
  `variable-target-value-spilled` (`assign.ts`),
  `kept-captures-of-borrows-too` (`hints.ts`),
  `clone-alias-of-a-capture-kept` (`tidy/bindings.ts`).

# 0978. A `loop` that advances, then tests, at each way's end is a `do … while`

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference, under the conditions below: each turn runs what's before its
advance, the next value, then the test, in that order in both forms.

## Context

React's updateReducerImpl walks its circular queue of updates:

```js
do {
  if (shouldSkip) { ..; update = update.next; continue; }
  ..;
  update = update.next;
} while (update !== null && update !== first);
```

Rust has no `do … while` whose `continue` goes on to the test. The port
writes each way's advance and test itself, `match next { Some(n) if
!Rc::ptr_eq(&n, first) => update = n, _ => break }`, which printed `while
(true) { if (..) { if (update.next != null && update.next !== first) {
update = update.next; continue; } } else { ..; const next = update.next;
if (next != null && next !== first) { update = next; continue; } } break;
}` (h09). Correct, in another shape.

## Decision

**A `while (true)` whose body ends `break;`, each way through the rest
ending in an advance `if (T(e)) { x = e; continue; }`, with `T` the same
of each, is `do { .. x = e; .. } while (T(x));`**: preparation's
`prepare/do_whiles.rs`, its last pass, as `js::StmtKind::DoWhile` is only
preparation's, read by nothing that reasons about a loop's test.

- **Each way ends in an advance**: an `if`'s branches each do, the last
  statement of each list. One that ends otherwise, `if (total <= cap) {
  .. }` with nothing after, keeps the `while (true)`.
- **`T` is the same of each, given the variable**: each advance's test
  with its value written as the variable, `next != null && next !==
  first` of `next`, alike part by part (`js::stmts_alike`), and reading the
  variable nowhere else: `next.lane > update.lane` would read the
  variable's next value where the advance read the one before.
- **Each value reads the same twice** (`reads_same`), as the advance read
  it in its test and again where it gave it, or is a `const` made right
  before the advance, `const next = update.next;`, given in its place.
- **No other `continue` of the loop**, which in a `do` goes on to its
  test, not its start. A `break` leaves either alike.
- **The variable is a `let` of the loop's block, before it, which nothing
  after the loop names, nor a function in it**: the `do`'s last advance
  gives it the value its test then refuses, which Rust's never gives it.

## Alternatives

- **Keep the `while (true)`**: correct, and upstream's `do` is lost.
- **A `do` from MIR**: the lowering would have to find the shape in MIR's
  blocks, where tidy already found it in the JS; a pass on the finished
  tree is one owner of one shape.

## Consequences

- updateReducerImpl's walk is upstream's `do`, but for its arms' `continue`,
  which the `if`/`else` the Rust wrote leaves out.
- `docs/readability.txt`: `while (true)` grows, 10 to 10.6 in 10,000
  lines, of `test/corpus/do_while_advances.rs`'s own cases that stay one
  as they must (a variable read after, tests apart, a test reading the
  variable, a way ending in a `break`).
- Held by `test/corpus/do_while_advances.rs`, run against native Rust,
  with mutations in `scripts/mutations/prepare/do_whiles.ts`; the guide
  shows the walk (Loops).

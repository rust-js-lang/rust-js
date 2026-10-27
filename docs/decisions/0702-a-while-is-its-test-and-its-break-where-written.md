# 0702. A `while` is its test, and its `break` is where it was written

Status: Accepted. Extends tidy's `while_tests` and
[0683](0683-an-if-is-as-written-and-an-else-if-stays-a-chain.md)'s `if`
as written, which left a loop's test in its body.
Amended: a `while` that ends an `if`'s branch with code after the `if`
is its test too; and an `if` of a `break` the program wrote first in a
`while`'s body is that `if`, not the loop's test (#298); and what's written
after a loop that `break`s is after it.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. How the JS reads;
nothing that runs changes.

## Context

React's `describeNativeComponentFrame` (shared/ReactComponentStackFrame.js)
has two loops its port writes as Rust does:

```rust
while s < sample_lines.len() && !sample_lines[s].contains("DetermineComponentFrameRoot") {
    s += 1;
}
while s >= 1 && c >= 0 {
    if sample_lines[s as usize] != control_lines[c as usize] {
        // ..
        break;
    }
    s -= 1;
    c -= 1;
}
```

rust-js wrote neither as a `while` of its test:

```js
while (true) {
  if (s >= sampleLines.length || sampleLines[s].includes("Root")) {
    break;
  }
  s++;
}
while (true) {
  if (s >= 1 && c >= 0) {
    if (sampleLines[s] !== controlLines[c]) {
      // ..
    } else {
      s--;
      c--;
      continue;
    }
  }
  break;
}
```

A `while` is `loop { if test { body } else { break } }` to rustc. Where
its test calls, a `Vec`'s index here, ADR 0683's `written elses` keeps
that `if` as written, its `else` the `break`; `while_tests` took a loop's
test only from `if (t) { break; } rest`. In the port, inside its guard's
`try`s, the second also took a labeled block, `bb163: { .. break bb163;
}`, round the `while (true)`.

## Decision

**`while (true) { if (t) { rest } else { break; } }` is `while (t) {
rest }`**, and **`while (true) { if (t) { rest } break; }` is `while (t) {
rest; break; }`**, of a loop the program wrote as a `while`
(`while_tests`). The `break`, and the `continue` below, are of no label:
in the loop's body and no loop in it, that's the loop's own, so no label
is compared. Of that `break` after the `if`, where `rest` ends `if (u) {
a } else { b; continue; }`, the `break` is `a`'s and `b` the turn's rest
after it, `if (u) { a; break; } b`, and of `if (u) { b; continue; } else
{ a }`, `if (u) { b } else { a; break; }`, as written (`broken_after`):

```js
while (s < sampleLines.length && !sampleLines[s].includes("Root")) {
  s++;
}
while (s >= 1 && c >= 0) {
  if (sampleLines[s] !== controlLines[c]) {
    // ..
    break;
  }
  s--;
  c--;
}
```

A `loop` the program wrote stays `while (true)`, its `break` where it's
written: upstream's `do { .. } while (s >= 1 && c >= 0)` is the port's
`loop { ..; if !(s >= 1 && c >= 0) { break; } }`, which JS's tree has no
`do … while` for yet.

## Why

- **The loop the program wrote**: its test is the `while`'s, as Rust and
  upstream have it, not an `if` and a `break` the reader puts back
  together.
- **One rule for a `while`'s test**, wherever its `if` came from:
  `while_tests` takes it of either form.

## Alternatives

- **Undo `written elses` in a loop's head**: the `if` the program wrote
  with an `else` would lose it elsewhere too. Rejected.
- **A `for (; t; s--, c--)`**: upstream's, but the port writes `s -= 1;
  c -= 1;` as statements after the `if`, which a `for`'s update would
  move. Left for when a `for` the program can't write in Rust is asked for.

## Amendment: a `while` that ends an `if`'s branch

React's ReactFiberLane walks a root's lanes in an `if` that code follows:

```rust
if all_entangled_lanes != NO_LANES {
    let entanglements = root.entanglements.borrow();
    let mut lanes = entangled_lanes & all_entangled_lanes;
    while lanes > 0 {
        ..
    }
}
entangled_lanes
```

MIR's exit of the `while` is the `if`'s end, a block of its own, so the
`while (true) { if (!(lanes > 0)) { break bb7; } .. }` `while_tests`
reads in the chains stage leaves the block, not the loop, and stayed.
The settled stage makes that `break` the loop's own (ADR 0859); then
`while_tests` runs again, `tests of loops ending blocks`, and reads its
test: `while (lanes > 0) { .. }`, as upstream's. Before, a borrow counted
across the loop ended the block with its `finally`, which hid it.

## What holds it

- `test/lowering-loops.test.ts`, "a while that ends an if's branch keeps
  its test", run in JS; mutation `settled-while-tests-off`.
- `test/lowering-written-loops.test.ts`, "a while is its test, its break
  where it was written", run in JS.
- `test/corpus/signed_index.rs`, React's loops, run in both.
- Mutations `while-of-else-break-unread`, `while-of-tail-break-unread`,
  `broken-after-continued-branch`, `broken-after-continued-then`,
  `broken-after-then-unbroken`, `broken-after-else-unbroken`.

## Amendment: an exit that goes on with the loop stays in it

React's `findFirstSuspended` walks up while a fiber has no sibling:

```rust
while node.sibling.borrow().is_none() {
    let return_fiber = node.r#return.borrow().clone();
    match return_fiber {
        Some(return_fiber) if !Rc::ptr_eq(&return_fiber, row) => node = return_fiber,
        _ => return None,
    }
}
```

MIR tests it as `while (true) { if (node.sibling == null) { .. continue;
.. break; } rest }`, and tidy's `while_tests` wrote `while (!t) { rest }
exit` of an exit that leaves: the exit after the loop, its `continue`,
which went on with the inner loop, then the outer one's. The JS climbed
back to a fiber's child it had left, and never ended: a wrong answer.

**An exit that goes on with the loop, a `continue` of it, stays in it**
(`js::continues`): the loop is `while (true) { if (t) { .. } .. }`, as
MIR has it. Case N: what runs is Rust's.

- `test/corpus/while_exit_continues.rs`, beside native Rust; mutation
  `while-exit-continue-moved`. Its loops, `bb2: while (true)` around
  `while (true)`, count one more of `docs/readability.txt`'s labeled
  blocks and `while (true)`s, blessed: the JS that runs as Rust does. No
  other corpus case's or snapshot's JS changed.

## Amendment: no `break` after what leaves itself (#290)

`while_tests` makes `while (t) { rest }` of a loop its function ends
with, and appends the loop's `break` to `rest`, leaving it for
`unreachable` to drop. When `rest` already ends with what leaves, such
as a panic's `throw`, the `break` stayed: the `chains` stage runs no
`unreachable` after it, so tidy's contract "nothing after a jump" broke
and the compiler panicked (fuzz seed 61706). Now `broken_after` appends
no `break` after a `break`, `continue`, `return` or `throw`.
`while_test_ends_in_throw` compiles such a loop and runs it against
native Rust, and `broken-after-a-throw` is caught.

## Amendment: the inner `while` is upstream's, with no label

The amendment above left `findFirstSuspended` right, but not as upstream
writes it:

```js
bb2: while (true) {
  let result;
  if (node.tag === 13) {
    result = node;
  } else {
    // ..
    while (true) {
      if (node.sibling == null) {
        // ..
        result = undefined;
        break;
      }
      // ..
      continue bb2;
    }
  }
  return result;
}
```

The inner loop's `return None` reached the function's `return` through
the inner loop's `break`, and what followed the loop, MIR's exit of its
test, through a `continue` of the outer one. `returned_in_branches` made
a branch's value a `return` only where it ends in an assignment or an
`if`'s, not a `while (true)` left by a `break` once it's made, so the
result stayed, and with it the loop's `break`; `while_heads` made a
`while (c)` only of `if (c) { body; continue; } rest`, a body with no
`continue` or `return` of its own inside, not of an `if` whose `else` is
`rest`; and the outer loop's `continue`, past the inner loop now, stayed
where its body ended.

**A `while (true)` each of whose `break`s follows what makes the
function's value makes it** (`made_at_ends`): each `break` then the
`return` of that value (`returned_at_ends`). It's the one rule for a
loop's `break`s to the function's end: `threaded_jumps`, which made each
of a loop's own `break`s a block's `break` for `returned_where_made` to
return, only ever did so of such a loop, and no longer does. **Of a
`while` the program wrote, `while_heads` takes `while (true) { if (c) {
body } else { rest } }`, and a `body` that goes round by a `continue` or
leaves by a `return` inside it**, never by a `break`, which would run
`rest` once it's outside: `while (c) { body } rest`. A `loop` the program
wrote is left as it was. **`tail continues` runs once, after
`while_heads`**, in the `written` stage, not in `chains`: what
`while_heads` puts after a loop ends the body around it with a
`continue` of it, which goes, and with it the loop's label, where that
was its last jump. No other corpus case's or snapshot's JS changed with
it in either place. Case N: what runs is Rust's; the JS is upstream's:

```js
while (true) {
  if (node.tag === 13) {
    return node;
  }
  // ..
  while (node.sibling == null) {
    const returnFiber = node.return;
    if (returnFiber == null || returnFiber === row) {
      return;
    }
    node = returnFiber;
  }
  // ..
}
```

The port's own `findFirstSuspended`, whose first arm is an `if` of an
`else if` chain, already printed so; the corpus case, whose first arm
returns alone, didn't. A `while (true)` with a `break` that makes no
value, or a `while`'s body with a `break` of its own, are guarded and
left as they are, though no corpus case makes either.

- `test/lowering-written-loops.test.ts`, "React's findFirstSuspended's
  loops are upstream's, with no label", both shapes, run in JS; and
  `test/corpus/while_exit_continues.rs` beside native Rust, its JS with
  no label. Mutations `loop-breaks-not-returned`,
  `broken-to-inner-loops` (now of `returned_breaks`), `while-heads-else-kept`,
  `tail-continued-label-kept` and `tail-continues-unwritten-kept`.
- `docs/readability.txt`: one labeled block and one `while (true)` fewer;
  two `return;`s more, the program's `return None`s, where it wrote them.

Note (#290): since ce4dbdce5 reordered the `names` stage's loop passes,
no known program reaches `broken_after` with a `rest` that already
leaves. The seed, its reductions and four other shapes compile without
the guard, and `broken-after-a-throw` survived main's check. So the
guard and its mutation are gone. `while_test_ends_in_throw` stays, and
still holds the contract against the crash.

## Amendment: an `if` of a `break` the program wrote is not the loop's test (#298)

```rust
while i < n {
    if c && i == 3 {
        // Found it.
        break;
    }
    i += 1;
}
```

was `// Found it.` above `while (i < n && (!c || i !== 3)) { i++; }`: MIR
tests the loop's test and then the program's, each going to the loop's
end, and tidy joins tests that go to one place, so `while_tests` took both
as the loop's. The `break` the program wrote, and its `if`, were gone, and
the comment before it had no place left in the loop.

**A `while`'s test is the loop's own: of the tests joined in it, those
that are an `if` of a `break` the program wrote (`Written::breaks`, ADR
0684, amended) are that `if` again, first in the loop's body, each where
it's written** (`loops::broken_apart`):

```js
while (i < n) {
  if (c && i === 3) {
    // Found it.
    break;
  }
  i++;
}
```

- Of the loop's test `t && u`, `u` the program's, or `while (true) { if
  (!t || !u) { break; } .. }`'s `||`, the `while` tests `t` and its body's
  first statement is `if (!u) { break; }`, the same tests in the same
  order, each leaving where it did. A `break` in the body runs what's
  after the loop as the test's failing does, so an exit after it is the
  same.
- The tests of one `if` are one `if`, `if (c || d)`, and two `if`s two,
  `if (c) { break; } if (d) { break; }`: each test is of the `if` its
  place is in.
- A test of the loop's own, `i < n`, and one MIR made, stay the `while`'s.

Case: N. The same tests run in the same order, each leaving the loop
where it did; how the JS reads, and where a comment is.

No corpus case or snapshot changed; the JS of each `while` whose first
`if` the program wrote of a `break` is that `if` now.

- `test/lowering-written-loops.test.ts`, "a while's test is its own, an
  if of a break the program wrote its if": an `&&`, an `||`, two `if`s
  and a `!`, run; and `test/lowering-comments.test.ts`, "a comment before
  a loop's break is before the break", its `while`.
- Mutations `while-written-breaks-joined` and
  `while-written-breaks-ungrouped`.

## Amendment: what's written after a loop that `break`s is after it (2026-10-26)

React's enqueueCapturedUpdate clones each update in a `loop` that ends
`match next { Some(next) => update = next, None => break }`, then appends
the captured update, under `// Append the captured update the end of the
cloned list.`, and goes on past the `if let` it's in. MIR reaches that
code through the `break` alone, and lowering wrote it there, `if (next
!= null) { update = next; } else { ..; break; }`: `exits_after`, which
moves what's written after a loop out of its way out, took only a loop
nothing `break`s, leaving by a `return`. And where it did move it, the
`break` it left had the moved code's place, so a comment before that
code, `// Append the captured update.`, was the `break`'s, in the loop.

**What the program wrote after a loop is after it, its way out the loop's
own `break` too**: of a loop with one `break`, the code before it there
that's written after the loop moves after it, and the `break` stays as
it is, `if (next == null) { break; } update = next;`. Not where another
`break` leaves the loop, which would then run it. The `break` left where
the code was, of a `return`'s way out, is where the moved code ends, so
what's written before that code, a comment, is the code's, after the
loop. Case A: the code runs where the loop ends, as it did where the loop
left.

- calculator's `tokenize`, interpreter's lexer and a match arm that ends
  in a loop (`test/lowering-loops.test.ts`) write what follows the inner
  loop after it, `while ((c = $nextIf(..)) != null) { name += c; }
  tokens.push(..)`; `adapter_source_drops.rs`'s comment and
  `loop_drops.rs`'s and `scope_temporaries.rs`'s blank lines are after
  their loops. `docs/readability.txt` is blessed: the case below adds two
  `while (true)`s, its `loop`s, as written.
- Held by `test/lowering-comments.test.ts`, "a comment before a match an
  if is made of is the if's", run in JS; the corpus case
  `loop_exit_code_after.rs`, against native Rust; and the mutations
  `exit-after-of-a-break-kept` and `exit-after-break-at-the-moved-code`
  (`lower/mir/tidy/jumps/labels.ts`). That a second `break` keeps the
  code in place is a guard no program here reaches: MIR makes one block
  of what's after a loop, which each `break` reaches.

## Amendment: a `continue` of the loop around is where it's written

React's getHostSibling (ReactFiberCommitHostEffects.js) walks siblings in
`siblings: while (true) { .. }`, and in it a `while (node.tag !==
HostComponent && node.tag !== HostText)` whose arms `continue siblings`:

```js
while (node.tag !== HostComponent && node.tag !== HostText) {
  if (isScope(node)) continue siblings;
  if (node.flags & Placement) continue siblings;
  ..
}
if (!(node.flags & Placement)) return node.stateNode;
```

Its port, the same `'siblings: loop { .. while .. { if is_scope(&node) {
continue 'siblings; } .. } if (node.flags.get() & PLACEMENT) == 0 { return
..; } }`, printed the inner `while` as `while (true) { if (t) { .. }
.. }`, each `continue 'siblings` a negated test nesting the rest, and
the found `return` outside the outer loop. Three things kept it so:

- **MIR joins the three `continue`s into one**, a block whose `break`s
  go to the outer loop's `continue`, `bb86: { if (isScope(node)) { break
  bb86; } .. } continue bb2;`, and `tail jumps` nested the rest of the
  block in each test's negation. **A `break` past a block to a
  `continue`, in an `if` of a `continue` the program wrote
  (`Written::continues`), is that `continue`** (`continues_written`,
  `jumps`' stage, before `tail jumps`): the `break` goes there and
  nowhere else, so the two run the same. Not inside a loop in the block,
  whose own an unlabeled `continue` would be.
- **`while_heads` took no `rest` with a `break` of the loop**, the `if
  (!(node.flags & PLACEMENT)) { break; } continue siblings;` MIR left
  after the inner loop's test: once `rest` is after the loop, the `break`
  would leave another. Now **the `rest`'s own `break`s fall out to what
  follows it** (`fallen`): a last one goes, and `if (c) { break; } r` is
  `if (!c) { r }`, where it went on to what's after the loop, as it does
  there. Any other jump of the loop's in `rest` keeps it a `while (true)`.
- **The `if` after it was a guard**, `if (node.flags & PLACEMENT) {
  continue; } return ..`: `written thens` (ADR 0879, amended) ran before
  the `while` was made. It runs again after `while heads`.

Case N ([0262](0262-when-rust-and-js-disagree.md)): each rewrite moves a
jump to where it goes, or runs the statements a jump went to.

```js
while (node.tag !== HOST_COMPONENT && node.tag !== HOST_TEXT) {
  if (isScope(node)) {
    continue siblings;
  }
  if (node.flags & PLACEMENT) {
    continue siblings;
  }
  const child = node.child;
  if (child != null && node.tag !== HOST_PORTAL) {
    child.return = node;
    node = child;
    continue;
  }
  continue siblings;
}
if (!(node.flags & PLACEMENT)) {
  return node.stateNode;
}
```

Left: the `else { continue 'siblings; }` the program wrote after its `if
let` is `continue siblings;` after the `if`, whose `then` ends `continue;`:
`Written::continues` has only an `if`'s `then`'s jumps, so nothing says
the last `continue siblings` is the `else`'s.

Held by `test/lowering-early-exits.test.ts`, "a while's continues of the
loop around are where the program wrote them", run in JS, with a `loop`
that stays `while (true)`; and the mutations
`continues-written-left-breaks`, `continues-written-of-any-break`
(`tidy/jumps/continues.ts`), `mir-tidy-while-head-rest-jumps`,
`fallen-test-not-negated` (`tidy/jumps/tried.ts`) and
`written-thens-not-after-while-heads` (`tidy/stages.ts`).


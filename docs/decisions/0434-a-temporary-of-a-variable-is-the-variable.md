# 0434. A temporary of a variable is the variable, until it changes

Status: Accepted. Extends [0364](0364-mir-lowering.md). Amended
(2026-10-26): a copy, a temporary's or a pattern's binding's, is the
variable where each read of it is before the variable is set, on every way.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a read of a variable
where it holds what its temporary holds gives the same value.

## Context

MIR copies a place into a temporary before it reads it: `match
name.as_str()`, `args.push(self.expr()?)`'s two-phase borrow of `args`,
`point.x += 3` of a `&mut`. `alias_variables` read the variable for such
a temporary only where nothing ever set the variable again, so of a `let
mut name` or a `let` array it wrote both:

```js
const value = name;
out.push(value === "let" ? "Let" : value === "fn" ? "Fn" : { TAG: "Ident", _0: name });
const value = args;
const result = Parser.expr(parser);
// ..
value.push(result._0);
```

## Decision

**`const _5 = x;` is `x` where it's read, where nothing sets `x` from it to
its last read; one read nowhere goes.** One rule for a variable that
changes and one that doesn't, which never is.

```js
out.push(name === "let" ? "Let" : name === "fn" ? "Fn" : { TAG: "Ident", _0: name });
args.push(result._0);
```

- **Not where it changes before:** `pair(n, { n += 1; n })` copies `n`
  first, `const value = n; n = (n + 1) | 0; pair(value, n)`, and stays so.
- **Closures:** none reads a temporary, a closure's captures being its own
  `const`s (`const captured = count`), and none sets the variable between,
  which Rust's borrows forbid.

## Why

- **It's the JS a person writes**, `args.push(x)`, `v.sort(f)`, `point.x =
  point.x + 3`, across 12 of the corpus's programs and the `collections`
  example.
- **It's right by construction:** where nothing sets `x` between, `x` and
  its copy are the same value. The `alias_reads` case reads a variable
  before it changes, after it changes in a later argument, in a loop, and
  in a closure made before it changes, against native Rust.

## Amendment: until it changes on the way to each read (2026-10-26)

React's enqueueCapturedUpdate links each clone:

```js
if (newLast === null) {
  newFirst = newLast = clone;
} else {
  newLast.next = clone;
  newLast = clone;
}
```

Its port matches `&new_last`, `Some(last) => { last.next.replace(..);
new_last = Some(clone); }`, which MIR reads through a temporary, `const _15
= newLast;`, and binds, `const last = _15;`. Both were kept, as `newLast` is
set again before the temporary's last read, `if (_15 == null)`'s, the
statement that reads it last setting it too, and the binding's block sets
it: `const last = newLast; if (last == null) { .. } else { last.next =
clone; newLast = clone; }`.

**A copy of a variable is the variable where, on every way to each read of
the copy, nothing has set the variable yet**: an `if`'s branches each a way
of its own, after its test; `x = e` once `e` is read; any other statement
that reads the copy and sets the variable both, not. One that's read after,
`const old = current; current = next; sum += old.value`, or in an argument
after a later one sets it, `pair(value, x)` of `pair(x, { x += 1; x })`, is
its own `const`, as before. That's the temporary's (`alias_variables`) and
a pattern's binding's (`copied_bindings`, ADR 0598), which now takes an
`else`'s too, where the test says the variable isn't there, `match
&new_last`'s `Some(last)` arm after `None`'s:

```js
if (newLast == null) {
  newLast = clone;
} else {
  newLast.next = clone;
  newLast = clone;
}
```

- A temporary's read the variable takes is where the read is written,
  else where the temporary was: `if (!words[i]) { i++; continue; }`, read
  so sooner, is still the `if` of a `break` the program wrote, not joined
  to its `while`'s test (ADR 0702, amended).
- Case A still: where nothing set the variable on the way, it and its copy
  are the same value. A closure, which may run once the variable is set,
  reads no temporary (above), and a binding a closure reads stays its own.
- Held by `test/lowering-shadows.test.ts`, "a binding of a variable set
  again past its reads is the variable", run in JS; the corpus case
  `binding_read_before_set.rs`, against native Rust, of a list linked so,
  a binding read before and after its variable is set in one `if`'s two
  branches, a copy read in an argument after a later one sets it, in a
  branch and not, and a binding read in a test after its variable is set;
  and the mutations `alias-kept-where-set-after`, `alias-read-after-set`,
  `alias-read-in-a-branch-after-set` (`lower/mir/tidy/bindings.ts`),
  `else-binding-copy-kept` and `binding-copy-kept-where-set-after`
  (`lower/mir/tidy/copied.ts`); `alias-read-placeless` by
  `test/lowering-written-loops.test.ts`.

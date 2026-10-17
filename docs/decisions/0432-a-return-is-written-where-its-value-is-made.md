# 0432. A `return` is written where its value is made

Status: Accepted. Extends [0370](0370-loop-exits-first.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): where a `return` is
written changes nothing a program can observe, only how its JS reads.

## Context

MIR has one return block. A function whose `match`, `?` or `return Err(..)`
sits inside a loop or another `match` jumps there from each place it makes
its value, and rust-js wrote that as a variable, a labeled block and a
`return` after it:

```js
atom(parser) {
  const t = Parser.next(parser);
  let result;
  bb108: {
    bb2: {
      if (t != null) {
        bb125: {
          if (t.TAG === "Num") {
            result = { TAG: "Ok", _0: { TAG: "Num", _0: t._0 } };
            break bb108;
          }
          // ...
        }
        return result;
      }
    }
    result = { TAG: "Err", _0: `unexpected ${..}` };
  }
  return result;
}
```

`returned_from_block` returned in place only for a block a function's body
ended in, and only where nothing else read `result`, so a nested one, or a
`?` whose error left through a loop's `break`, stayed. A parser or an
interpreter, the corpus's `interpreter` and `calculator`, read like this
in nearly every function. And `tried_first`, which writes a `?` as its
guard, skipped what the first guard it wrote put in place, so a second `?`
there stayed `if (r.TAG !== "Err") { .. } else { return r; }`.

## Decision

**Each `result = v; break l;`, where block `l` is followed by `return
result;`, is `return v;`, in any list of statements, whatever else reads
`result`; and the block goes where nothing jumps to its end any more.**

```js
atom(parser) {
  const t = Parser.next(parser);
  if (t != null) {
    if (t.TAG === "Num") {
      return { TAG: "Ok", _0: { TAG: "Num", _0: t._0 } };
    }
    // ...
  }
  return { TAG: "Err", _0: `unexpected ${..}` };
}
```

- **A jump is followed to where it goes.** A block a `break l` follows has
  its own `break`s made that one and its statements put in its place. A
  loop a `break l` follows has its own `break`s made that one, not those
  of a loop inside it; an `if` of two branches has it at the end of each
  that goes on. That's kept only where it leaves no `break l`. Otherwise
  each `break l` stays where it was, and each `result = v` that ends at
  one is a `return`.
- **A branch that ends in a jump counts as an end**, as a `return` does:
  it never goes on to the block's end. A jump to that end stays where it
  is, and so does the block.
- **A loop's `continue` the program didn't write, left at its body's end,
  goes** (`tail_continues`); one it wrote stays, the guard it ends.
- **`tried_first` looks at what it put in place,** a `?` of its own too,
  and also takes `if (r.TAG !== "Err") { s } return r;` of an `s` that
  never goes on. Neither moves `s` where a name it declares is declared
  beside it.

## Why

- **It's the JS a person writes:** a `return` where the value is made, a
  guard for each `?`. The `interpreter` case, a lexer, a parser and an
  evaluator, went from 15 labeled blocks to 3, and `calculator` from 6 to
  2. The pilot's API calls and the `calc` example write each `?` as a
  guard.
- **It's right by construction:** a `break l` goes to the statement after
  `l`. Where that's `return result`, `result = v; return result` is `return
  v`, and nothing reads `result` after a `return`. The `returns_in_place`
  case has `?` in loops inside loops, a loop of its own left by its own
  `break`, a labeled loop, `continue`s and `let .. else`, each against
  native Rust. A mutation for each step is caught.

## Alternatives

- **Thread every jump.** A `break l` pushed into each branch, kept where
  it doesn't become a `return`, would leave the same jump written two or
  three times. Threading is kept only where it leaves none.
- **A pass that restructures MIR's return block before structuring.** The
  structuring would need to know which values are the function's, which
  tidy already does.

## Consequences

- An `Ok(match .. { .. })` whose arms each give a value still makes it in
  a variable, `tmp`, returned once after a block: each arm's `return {
  TAG: "Ok", .. }` would repeat the `Ok`.

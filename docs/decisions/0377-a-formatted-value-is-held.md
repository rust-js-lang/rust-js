# 0377. A format string's arguments are evaluated first, and held where read twice

Status: Accepted.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): Rust evaluates a
format string's arguments before it formats any, which the JS keeps; N of
where a value formatting reads twice is named; and B of a `Debug` that
prints while it formats, below.

## Context

`{:?}` of a `Result` or an `Option` reads the value more than once, its
tag and what it holds. Of a variable, rust-js writes the test in place;
of a value made where it's formatted, a call MIR folded into the
template, it wrote a function and called it at once:

```js
`${value.TAG === "Ok" ? `Ok(${value._0})` : ..} ${((result) => (result.TAG === "Ok" ? .. : ..))(parse("x"))}`
```

Two forms for one construct in one line, 248 of them in the corpus. No
person writes the second.

A format string's arguments were evaluated where the template formats
them, and MIR folded a call into it there: `println!("{:?} {:?}", a,
make())` called `a`'s `Debug` before `make()`, where Rust calls `make()`
first. A `Debug` that prints showed it.

## Decision

**A format string's arguments are evaluated before it formats any**:
where the template would evaluate an argument that does something after a
part whose formatting may run the program's code, a `Debug` of the crate's
or a dictionary's method, each argument that does something is held first,
in the order given, and the template formats what's held (MIR's
`formats_out_of_order`). A runtime helper's, a JS global's or a JS method of
text or an array, `.map(f)`, runs none, its callbacks looked at too. MIR's
folding already keeps the arguments in the order given, and one read twice
held.

**A value formatting reads more than once, given as it's made, is held
first, `const value = parse("x");`, and the test reads it**, as a value
read twice is held everywhere else (MIR tidy's `applied_values`, named by
`readable_temporaries`).

- **Only where the program's order is kept**: what the statement evaluates
  before it must read the same after it, `read_past_calls`, not a call
  whose effect it would precede, `${Counter.get(c)} ${..(Counter.bump(c))}`,
  nor what a test decides runs.
- `read_past_calls` knows more of what reads the same past a call: text
  and numbers made of such, `${a}-${b}`, `String(n)`, a call of the
  runtime's pure helpers, an item of a plain value, `pair[0]`, as its field,
  and `console`'s members, as `Math`'s. An assignment's value is looked at
  too, `out += ..`, after its target.
- A function's body put where it was called in a template is spliced into
  it: `(${a}, ${b})`, not `${`(${a}, ${b})`}`.

## Consequences

- **A `Debug` or `Display` that prints while it formats prints before the
  line, not inside it** (B): `print!` writes the line once it's formatted,
  where Rust writes each piece as it formats it, so `println!("{} {:?}", 1,
  loud)` whose `loud` prints shows its own line first. Formatting that
  prints is rare, and the line is the same.

- 190 of the corpus's 248 are held. The rest follow a call in the same
  line whose effect the value's would precede, `${Counter.get(c)}
  ${..(Counter.bump(c))}`, and stay where they are.

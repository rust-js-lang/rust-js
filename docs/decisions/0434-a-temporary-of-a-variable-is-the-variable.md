# 0434. A temporary of a variable is the variable, until it changes

Status: Accepted. Extends [0364](0364-mir-lowering.md).

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

# 0569. A temporary is named as rust-analyzer suggests

Status: Accepted. Amends [0364](0364-mir-lowering.md), whose tidy named a
temporary after the field it reads or else `value`; extends
[0352](0352-reclaimed-names.md) and [0565](0565-readability-is-a-gate.md),
whose first number to bring down this is.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. A name changes
nothing a program can observe; this decides how the JS reads.

## Context

A temporary MIR makes and JS keeps, because something reads it more than
once, an `if let`'s scrutinee, a `RefCell`'s borrow, a value `println!`
shows, was named after the field it reads, `routes` of `tree.routes`, and
otherwise `value`. A function that keeps several kept them as `value`,
`value$1`, `value$2`, as many as there were, since JS gives one block's
`const`s each a name of its own. ADR 0565 measured what that costs: 951 of
every ten thousand lines of the committed JS named something given apart,
`x$1`, the most of the seven shapes by far, and 2,111 of those names were
`value`'s.

| What the suffixed names were, on main | Declarations |
|---|---|
| `value$N` | 2,111 |
| `tmp$N`, a `let` its branches set | 456 |
| `result$N` | 84 |
| `arg$N`, an argument held | 59 |
| others, a program's own names | 672 |

A Rust developer meets the same question where they extract a variable,
and their tooling answers it: rust-analyzer's `suggest_name`, which names
an expression after the method or function that made it, without a
conversion's prefix, `uppercase` of `to_uppercase()`, and skips the ones
that name nothing, `unwrap`, `clone`, `iter`, `new`; else after its type,
through `Option`, `Box` and `Rc`. That is the name the developer has seen
for that expression already.

## Decision

**A temporary JS keeps is named as rust-analyzer suggests: after the call
that made it, where the source calls it by that name; else after the
program's type it holds; and the first of those no other name in the
function has. Where both are taken, or neither says, it's `value`, `arg`
or `tmp` as before.**

- **The call's name**, camel case: `trailingZeros` of `a.trailing_zeros()`,
  `borrowMut` of `count.borrow_mut()`, `firstKeyValue`, `floor` of
  `x.floor()`; a method without `to_`, `as_` or `into_`, a function without
  `from_`, `with_` or `into_`; none of rust-analyzer's methods that name
  nothing, nor `unwrap_err` and `expect_err`, their `Err` side, nor its
  names that say nothing, `new`, `default`, `str`; nor a keyword, `mut` of
  `as_mut()`.
- **Only a call the source writes by its name** (`fn_span`'s text): not one
  a macro writes, `vec![..]`'s `new_uninit`, `format!`'s `must_use`, nor an
  operator's, `a[i]`'s `index_mut`, nor a closure's, `f(x)`'s `call_mut`.
- **A reference MIR takes to a temporary, or its copy**, `&_5` as
  `format_args!` takes what `println!` shows, has that temporary's names:
  `get` of `println!("{:?}", ages.get("bob"))`.
- **The program's type**, of the program or a library, lower camel case,
  `point` of an `Option<Point>`, `httpServer` of an `HTTPServer`, through a
  reference, an `Option`, a `Box`, an `Rc`. Not a `Result`'s `Ok` type, as
  rust-analyzer's is: JS holds a `Result` as `{ TAG, _0 }`, which
  `order.TAG` would misname. Not std's types, `u32` or `String`.
- **A name taken is passed over**, not given apart: `find(points, 1)`'s
  result, with `find` read and `point` another's, is `value`. A field's
  name, a JSX prop's and a trait object's `receiver` come first, as before.

`mir/hints.rs` answers what each temporary is suggested to be named
(`temporaries_names`), from its MIR; `tidy/bindings.rs`'s
`readable_temporaries` is still the one place a temporary is named.

## Why

- **It's the name a Rust developer already has for it.** The source says
  `a.trailing_zeros()`; its IDE extracts it as `trailing_zeros`; the JS
  keeps it as `trailingZeros`. No word of rust-js's own, and nothing to
  learn: guessable from the call.
- **It's what the gate asked for.** The renames' share falls from 951.2 to
  829.9 in ten thousand lines, 1,046 fewer lines with a name given apart,
  and no other shape's share rises; `test/corpus/temporary_names.rs` is
  the rule, held against native Rust.
- **A name taken falls back, not apart**: `point$1` beside a `point`
  reads worse than `value`, which says it's a value and nothing more.

## Alternatives

- **Name after the JS that's kept**, `$trailingZeros`'s `trailingZeros`:
  tried first. A runtime helper's name is the operation's, and half of
  them read as verbs, `const fromJson = $fromJson(..)`, `const borrowMut`;
  the Rust call is the name the reader knows. Rejected.
- **Only the type**: safe, and it moved the share 1.8%. The calls are what
  a temporary mostly holds. Kept as the fallback.
- **Reuse `value` where the one before it is dead**: a block's `const`s
  can't share a name, and a `let` reassigned reads as state that isn't
  there. Rejected.
- **Name a `Result` after its `Ok` type**, as rust-analyzer does: `order.
  TAG === "Ok"` and `order._0` misread the object. Rejected; a `Result` is
  named after its call, `checkedDiv`, `parseNumbers`.

## Consequences

- 281 files of the corpus and the snapshots read the new
  names: `const floor = Math.floor(x)`, `const next = $next(entries)`,
  `const point = find(points, 3)`, where each was `value`, `value$1`.
- **Some names are verbs**, `next`, `get`, `borrowMut`, `peek`, as
  rust-analyzer's are; they say what made the value.
- **What a closure applied at once holds is still `value`**: `println!`'s
  `{:?}` of a call's result whose closure tidy applies (`applied_values`),
  `$checked(ages.size - 4, ..)`. Its value is no MIR temporary by then;
  naming it is the next step for the share.
- **A temporary in code a macro of the program writes**, `macro_rules!`'s,
  has no name the source writes, and keeps `value`.

## What holds it

- `test/corpus/temporary_names.rs`: names after the method, through a
  reference `println!` takes, after the type, and `value` where both are
  taken, against native Rust.
- `docs/readability.txt`, reblessed: renames at 829.9.
- `test/compiler.test.ts`'s and `test/lowering.test.ts`'s lines that read a
  temporary's name; a unit test of a type's name as a variable's.
- Mutations: the suggestions unused, a name taken given apart, a call the
  source doesn't write named, a keyword named, a method that names nothing
  named, a reference's call unnamed, the type unused, a `Result` named
  after its `Ok`; and the props', arguments' and temporaries' own, kept.

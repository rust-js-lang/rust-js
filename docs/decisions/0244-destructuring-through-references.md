# 0244. A value taken apart through a shared reference is destructured

Status: Accepted. Extends [0020](0020-structs-and-tuples.md) and
[0023](0023-strings-references-shared-state.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's ErrorDecoder takes its params apart as it gets them:

```js
const {errorMessage, errorCode} = useErrorDecoderParams();
```

Its Rust gets a `&'static ErrorDecoderParams`, and a pattern of a
reference binds each part by reference. rust-js kept the value whole and
read each part where it is, `tmp.errorMessage`, as it does a place that
might change. So did each closure that takes a pair apart by reference,
`|&(path, _)|`, `(param) => param[0] === target`.

## Decision

**A tuple or struct pattern of a shared reference is JS's destructuring:
each part bound by reference is read once.**

```rust
let Params { message, code } = params();
files.iter().find(|(path, _)| path == shown)
```

```js
const { message, code } = params();
files.find(([path]) => path === shown);
```

- **What's borrowed can't change while it's borrowed**, so a part read
  once is the part read where it is, for as long as it's bound.
- **A `Cell`, which can change, is one JS object** (ADR 0099), the same
  one either way.
- **A `ref mut` part, and a place taken apart in place, are as before.**
- **A closure inlined where it's called reads its parts**: `|&(_, age)|
  age` of `item` is `item[1]`, not the arrow called.

## Why

- **It's the JS a person writes.**
- **It's tested**: a compiler test takes a struct apart through a call's
  reference, one holding a `Cell`, which it sets, and a key closure
  inlined; the corpus and the playground's snapshots run and show each
  closure's pair destructured.
  Mutations keep the reference whole, bind each part where it is, and
  call the inlined closure.

## Amendment: a pattern's parts, however deep, one reading (2026-11-15, #1900)

A closure inlined where it's called reads each variable its parameter's
pattern binds as the part of its argument it is, `([, age]) => age` of
`item` as `item[1]`. An `Option`'s `map` read its closure's pattern by a
rule of its own, and that rule took apart only the top level. In
`o.map(|((k,), x)| (k, x))`, `k` was no part, so the JS read `k`, which
nothing declared, and it was refused (itertools 0.10.5's `cons_tuples`,
which every call of the crate needs, `multizip` among them). One reading
answers both now, `calls::pattern_parts`, however deep: `[[k], x]`'s `k`
is `o[0][0]`, `{ a: { b } }`'s `b` is `o.a.b`, a struct's in a tuple
and a tuple's in a struct alike. A pattern with a default or a rest has
no part to read, so its closure is called as before.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; the
JS was refused. A name the lost part read was always a fresh one, so it
read no other variable: `k$1` beside an outer `let k`. Corpus case
option_map_of_a_nested_pattern holds it against native Rust in both
builds: `((k,), x)`, `(((k,),),)`, a struct in a tuple, a tuple in a
struct, a type parameter's, and an outer `k`. A consumer of itertools
0.10.5's `multizip` builds and runs as native Rust. Mutation
`pattern-parts-top-level-only` (`lower/calls.ts`);
`apply-pattern-called` retargeted.
